//! Managed JavaScript and TypeScript analysis over captured project inputs.

use super::{
    inputs::{InputFiles, Inputs, PreparedInputs, absolute, save},
    project::Profile,
};
use crate::{
    dependency_sources::Input,
    ecmascript_dependencies::{self, Installations},
    lsp::{Client, DocumentSync, Readiness, Setup, uri},
};
use jsonc_parser::cst::{CstInputValue, CstRootNode};
use serde_json::{Map, Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    time::Duration,
};
use vizzi_core::{
    analysis::ECMASCRIPT_LANGUAGES,
    model::{Document, Snapshot},
    policy::{Classification, ECMASCRIPT_PROJECT_METADATA, SourceOnlyReason},
};

const METADATA_BYTES: usize = 8 * 1024 * 1024;
const METADATA_FILES: usize = 64;

pub(super) struct TypeScript;

impl Profile for TypeScript {
    fn languages(&self) -> &'static [&'static str] {
        ECMASCRIPT_LANGUAGES
    }

    fn document_language<'a>(&self, path: &Path, language: &'a str) -> &'a str {
        match (path.extension().and_then(|ext| ext.to_str()), language) {
            (Some("jsx"), "javascript") => "javascriptreact",
            (_, "tsx") => "typescriptreact",
            _ => language,
        }
    }

    fn inputs<'a>(&'a self, root: &'a Path, snapshot: &'a Snapshot) -> Result<Inputs<'a>, String> {
        let mut captured = blake3::Hasher::new();
        let mut metadata = Vec::new();
        let mut search_paths = Vec::new();
        let mut bytes = 0;
        for doc in snapshot.documents() {
            if is_project_input(doc) {
                if doc.path().contains([':', '\0']) {
                    return Err("Invalid captured TypeScript project path".into());
                }
                bytes += doc.bytes();
                if metadata.len() == METADATA_FILES || bytes > METADATA_BYTES {
                    return Err("Captured TypeScript project inputs exceed allowance".into());
                }
                metadata.push(doc.as_ref());
            } else if !doc
                .classification()
                .assigned_language()
                .is_some_and(|lang| self.accepts(lang))
            {
                continue;
            }
            // Reopen the whole project when an import target or its dependency changes.
            captured.update(&(doc.path().len() as u64).to_le_bytes());
            captured.update(doc.path().as_bytes());
            captured.update(doc.content().as_str().as_bytes());
            search_paths.push(doc.path());
        }
        let installed = Installations::discover(root, search_paths)?;
        installed.fingerprint(&mut captured);
        Ok(InputFiles::default().freeze(
            &format!("typescript:{}", captured.finalize().to_hex()),
            &[
                include_bytes!("typescript.rs"),
                include_bytes!("../ecmascript_dependencies.rs"),
            ],
            move |mirror| {
                let prepared = prepare(mirror, &metadata)?;
                installed.link(mirror)?;
                Ok(prepared)
            },
        ))
    }

    fn dependency_input(
        &self,
        root: &Path,
        path: &Path,
        configuration: &str,
        _: &str,
    ) -> Result<Input, String> {
        ecmascript_dependencies::input(root, path, configuration)
    }

    fn project_error(
        &self,
        client: &mut Client,
        path: &Path,
        rejected_configs: &BTreeMap<PathBuf, String>,
    ) -> Result<Option<String>, String> {
        if rejected_configs.is_empty() {
            return Ok(None);
        }
        let project = client.request(
            "custom/projectInfo",
            json!({"textDocument":{"uri":uri(path)?}}),
            Duration::from_secs(10),
        )?;
        let config = project["configFilePath"]
            .as_str()
            .ok_or("TypeScript project info has no configuration path")?;
        let path = Path::new(config);
        let path = path.canonicalize().unwrap_or_else(|_| path.to_owned());
        Ok(rejected_configs.get(&path).cloned())
    }
}

fn is_project_input(doc: &Document) -> bool {
    matches!(doc.classification(), Classification::SourceOnly {
        language: Some(language),
        reason: SourceOnlyReason::ProjectMetadata,
        ..
    } if language == ECMASCRIPT_PROJECT_METADATA)
}

fn prepare(mirror: &Path, metadata: &[&Document]) -> Result<PreparedInputs, String> {
    let configs = metadata
        .iter()
        .map(|doc| mirror.join(doc.path()))
        .filter(|path| path.file_name().is_some_and(|name| name != "package.json"))
        .collect::<BTreeSet<_>>();
    let mut projects = BTreeMap::new();
    let mut rejected_configs = BTreeMap::new();
    for doc in metadata {
        let path = mirror.join(doc.path());
        let text = doc
            .captured()
            .read_text(METADATA_BYTES)
            .map_err(|error| error.to_string())
            .and_then(|text| text.ok_or_else(|| "TypeScript project input is not UTF-8".into()));
        if configs.contains(&path) {
            match text
                .as_deref()
                .map_err(Clone::clone)
                .and_then(|text| prepare_config(text, &path, mirror, &configs))
            {
                Ok(config) => {
                    projects.insert(path, config);
                }
                Err(error) => {
                    let original = text.ok().and_then(|text| parse_config(&text).ok());
                    save_placeholder(&path, mirror, original.as_ref().map(|(_, value)| value))?;
                    rejected_configs.insert(path.clone(), config_error(&path, mirror, &error));
                }
            }
        } else {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            fs::write(&path, text?.as_bytes()).map_err(|error| error.to_string())?;
        }
    }
    check_projects(&projects, mirror, &mut rejected_configs);
    for (path, project) in projects {
        if rejected_configs.contains_key(&path) {
            save_placeholder(&path, mirror, Some(&project.config))?;
        } else {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|error| error.to_string())?;
            }
            fs::write(&path, project.tree.to_string()).map_err(|error| error.to_string())?;
        }
    }
    let tsconfig = mirror.join("tsconfig.json");
    if !configs.contains(&tsconfig) && !configs.contains(&mirror.join("jsconfig.json")) {
        save_placeholder(&tsconfig, mirror, None)?;
    }
    let setup = Setup {
        arguments: vec!["--lsp".into(), "--stdio".into()],
        initialization: json!({
            "runExternalCode": false,
            "disablePushDiagnostics": true
        }),
        configuration: json!({
            "js/ts": {"tsserver": {"automaticTypeAcquisition": {"enabled": false}}}
        }),
        readiness: Readiness::Immediate,
        document_sync: DocumentSync::Ordered,
        environment: ["NODE_OPTIONS", "NODE_PATH"]
            .into_iter()
            .map(|name| (name.into(), None))
            .collect(),
    };
    Ok(PreparedInputs {
        setup,
        rejected_configs,
        private_state: None,
    })
}

fn save_placeholder(path: &Path, mirror: &Path, original: Option<&Value>) -> Result<(), String> {
    let mut config = json!({
        "compilerOptions": {
            "allowJs": true, "jsx": "preserve", "noEmit": true,
            "module": "esnext", "moduleResolution": "bundler", "types": [],
            "maxNodeModuleJsDepth": 1
        },
        "typeAcquisition": {"enable": false}
    });
    if let (Some(original), Some(base)) = (original, path.parent()) {
        for key in ["include", "exclude", "files"] {
            if let Some(paths) = original.get(key)
                && path_list(paths, base, mirror).is_ok()
            {
                config[key] = paths.clone();
            }
        }
        if let Some(allow_js) = original["compilerOptions"]["allowJs"].as_bool() {
            config["compilerOptions"]["allowJs"] = allow_js.into();
        }
    }
    save(path, &config)
}

fn local_path(value: &Value, base: &Path, mirror: &Path) -> Result<PathBuf, String> {
    let value = value
        .as_str()
        .ok_or("TypeScript project paths must be strings")?;
    let path = Path::new(value);
    let path = absolute(base, path);
    if Path::new(value).is_absolute()
        || value.contains(['\\', ':', '\0'])
        || value.contains("${")
        || !path.starts_with(mirror)
    {
        return Err(format!(
            "TypeScript project path must stay inside the captured workspace: {value}"
        ));
    }
    Ok(path)
}

fn path_list(values: &Value, base: &Path, mirror: &Path) -> Result<(), String> {
    for value in values
        .as_array()
        .ok_or("TypeScript project paths must be an array")?
    {
        local_path(value, base, mirror)?;
    }
    Ok(())
}

fn inherited(
    value: &Value,
    base: &Path,
    mirror: &Path,
    configs: &BTreeSet<PathBuf>,
) -> Result<PathBuf, String> {
    let name = value
        .as_str()
        .ok_or("TypeScript extends must contain paths")?;
    if !name.starts_with("./") && !name.starts_with("../") {
        return Err(format!(
            "TypeScript extends requires a captured relative config: {name}"
        ));
    }
    let path = local_path(value, base, mirror)?;
    if configs.contains(&path) {
        return Ok(path);
    }
    if path.extension().is_none_or(|extension| extension != "json") {
        let mut json_path = path.into_os_string();
        json_path.push(".json");
        let path = PathBuf::from(json_path);
        if configs.contains(&path) {
            return Ok(path);
        }
    }
    Err(format!(
        "Extended TypeScript config is outside the capture: {name}"
    ))
}

struct ProjectConfig {
    config: Value,
    tree: CstRootNode,
    parents: Vec<PathBuf>,
    references: BTreeSet<PathBuf>,
}

fn parse_config(text: &str) -> Result<(CstRootNode, Value), String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let tree = CstRootNode::parse(text, &jsonc_parser::ParseOptions::default())
        .map_err(|error| error.to_string())?;
    let config = tree
        .object_value()
        .and_then(|object| object.to_serde_value())
        .ok_or("TypeScript config must be an object")?;
    Ok((tree, config))
}

fn prepare_config(
    text: &str,
    path: &Path,
    mirror: &Path,
    configs: &BTreeSet<PathBuf>,
) -> Result<ProjectConfig, String> {
    let base = path
        .parent()
        .ok_or("TypeScript config has no parent directory")?;
    let (tree, config) = parse_config(text)?;
    let parents = config.get("extends").map_or(Ok(Vec::new()), |extends| {
        extends
            .as_array()
            .map_or(std::slice::from_ref(extends), Vec::as_slice)
            .iter()
            .map(|entry| inherited(entry, base, mirror, configs))
            .collect::<Result<_, _>>()
    })?;
    let mut referenced = BTreeSet::new();
    if let Some(references) = config.get("references") {
        for reference in references
            .as_array()
            .ok_or("TypeScript references must be an array")?
        {
            let mut path = local_path(&reference["path"], base, mirror)?;
            if !configs.contains(&path) {
                path.push("tsconfig.json");
            }
            if !configs.contains(&path) {
                return Err(format!(
                    "Referenced TypeScript config is outside the capture: {}",
                    reference["path"]
                ));
            }
            referenced.insert(path);
        }
    }
    for key in ["include", "exclude", "files"] {
        if let Some(paths) = config.get(key) {
            path_list(paths, base, mirror)?;
        }
    }
    if let Some(options) = config.get("compilerOptions") {
        let options = options
            .as_object()
            .ok_or("TypeScript compilerOptions must be an object")?;
        compiler_options(options, base, mirror)?;
    }
    let object = tree
        .object_value()
        .ok_or("TypeScript config must be an object")?;
    for property in object.properties() {
        match property.decoded_name().as_deref() {
            Some("watchOptions" | "typeAcquisition") => property.remove(),
            Some("compilerOptions") => {
                if let Some(options) = property.object_value() {
                    for option in options.properties() {
                        if option.decoded_name().as_deref() == Some("plugins") {
                            option.remove();
                        }
                    }
                }
            }
            _ => {}
        }
    }
    object.append(
        "typeAcquisition",
        CstInputValue::Object(vec![("enable".into(), CstInputValue::Bool(false))]),
    );
    Ok(ProjectConfig {
        config,
        tree,
        parents,
        references: referenced,
    })
}

fn compiler_options(
    options: &Map<String, Value>,
    base: &Path,
    mirror: &Path,
) -> Result<(), String> {
    for key in [
        "baseUrl",
        "rootDir",
        "outDir",
        "outFile",
        "declarationDir",
        "tsBuildInfoFile",
        "mapRoot",
        "sourceRoot",
    ] {
        if let Some(path) = options.get(key) {
            local_path(path, base, mirror)?;
        }
    }
    for key in ["rootDirs", "typeRoots"] {
        if let Some(paths) = options.get(key) {
            path_list(paths, base, mirror)?;
        }
    }
    Ok(())
}

#[derive(Clone, Default)]
struct AliasOptions<'a> {
    base_url: Option<PathBuf>,
    paths: Option<(&'a Value, &'a Path)>,
}

enum AliasCheck<'a> {
    Visiting,
    Checked(Result<AliasOptions<'a>, String>),
}

fn alias_options<'a>(
    path: &Path,
    projects: &'a BTreeMap<PathBuf, ProjectConfig>,
    mirror: &Path,
    rejected: &BTreeMap<PathBuf, String>,
    checked: &mut BTreeMap<PathBuf, AliasCheck<'a>>,
) -> Result<AliasOptions<'a>, String> {
    if let Some(error) = rejected.get(path) {
        return Err(error.clone());
    }
    if let Some(existing) = checked.get(path) {
        return match existing {
            AliasCheck::Visiting => Err("Cyclic TypeScript config inheritance".into()),
            AliasCheck::Checked(result) => result.clone(),
        };
    }
    let (path, project) = projects
        .get_key_value(path)
        .ok_or("TypeScript config is outside the capture")?;
    checked.insert(path.clone(), AliasCheck::Visiting);
    let result = (|| {
        let mut options = AliasOptions::default();
        for ancestor in &project.parents {
            let parent = alias_options(ancestor, projects, mirror, rejected, checked)?;
            options.base_url = parent.base_url.or(options.base_url);
            options.paths = parent.paths.or(options.paths);
        }
        let base = path
            .parent()
            .ok_or("TypeScript config has no parent directory")?;
        let compiler = &project.config["compilerOptions"];
        if let Some(url) = compiler.get("baseUrl") {
            options.base_url = Some(local_path(url, base, mirror)?);
        }
        if let Some(paths) = compiler.get("paths") {
            options.paths = Some((paths, base));
        }
        if let Some((paths, origin)) = options.paths {
            let base = options.base_url.as_deref().unwrap_or(origin);
            for targets in paths
                .as_object()
                .ok_or("TypeScript paths must be an object")?
                .values()
            {
                path_list(targets, base, mirror)?;
            }
        }
        Ok(options)
    })()
    .map_err(|error: String| config_error(path, mirror, &error));
    checked.insert(path.clone(), AliasCheck::Checked(result.clone()));
    result
}

fn config_error(path: &Path, mirror: &Path, error: &str) -> String {
    let path = path.strip_prefix(mirror).unwrap_or(path).display();
    let (detail, _) = crate::review_application::clip(error, 384);
    format!("{path}: {detail}")
}

fn check_projects(
    projects: &BTreeMap<PathBuf, ProjectConfig>,
    mirror: &Path,
    rejected: &mut BTreeMap<PathBuf, String>,
) {
    let mut checked = BTreeMap::new();
    for path in projects.keys() {
        if let Err(error) = alias_options(path, projects, mirror, rejected, &mut checked) {
            rejected.insert(path.clone(), error);
        }
    }
    loop {
        let previous = rejected.len();
        for (path, project) in projects {
            if rejected.contains_key(path) {
                continue;
            }
            if let Some(error) = project
                .parents
                .iter()
                .chain(&project.references)
                .find_map(|reference| rejected.get(reference))
            {
                rejected.insert(path.clone(), config_error(path, mirror, error));
            }
        }
        if rejected.len() == previous {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prepares_captured_jsonc_projects_with_relative_inheritance_and_refs() {
        let directory = tempfile::tempdir().unwrap();
        let mirror = directory.path();
        let path = mirror.join("packages/app/tsconfig.json");
        let configs = [
            mirror.join("tsconfig.base.json"),
            mirror.join("packages/lib/tsconfig.json"),
        ]
        .into();
        let project = prepare_config(
            r#"{
                // Use captured sibling settings and project declarations.
                "extends": ["../../tsconfig.base"],
                "references": [{"path":"../lib"}],
                "include": ["src/**/*", "../lib/**/*.ts"],
                "compilerOptions": {
                    "paths": {"@library/*": ["../lib/*"]},
                    "plugins": [{"name":"project-plugin"}],
                },
                "typeAcquisition": {"enable":true},
                "watchOptions": {"watchFile":"FixedPollingInterval"},
            }"#,
            &path,
            mirror,
            &configs,
        )
        .unwrap();
        let config = project.tree.to_serde_value().unwrap();
        assert_eq!(config["extends"][0], "../../tsconfig.base");
        assert_eq!(config["references"][0]["path"], "../lib");
        assert_eq!(
            config["compilerOptions"]["paths"]["@library/*"],
            json!(["../lib/*"])
        );
        assert!(config["compilerOptions"].get("plugins").is_none());
        assert_eq!(config["typeAcquisition"], json!({"enable":false}));
        assert!(config.get("watchOptions").is_none());
    }

    #[test]
    fn refuses_uncaptured_config_references_and_escaping_config_paths() {
        let directory = tempfile::tempdir().unwrap();
        let mirror = directory.path();
        let path = mirror.join("src/tsconfig.json");
        for text in [
            r#"{"extends":"@outside/tsconfig"}"#,
            r#"{"extends":"./tsconfig.unknown.json"}"#,
            r#"{"references":[{"path":"../not-captured"}]}"#,
            r#"{"include":["../../other/**"]}"#,
            r#"{"compilerOptions":{"baseUrl":"C:/elsewhere"}}"#,
            r#"{"compilerOptions":{"rootDirs":["${configDir}/other"]}}"#,
            r#"{"compilerOptions":{"outDir":"/elsewhere"}}"#,
        ] {
            assert!(
                prepare_config(text, &path, mirror, &BTreeSet::new()).is_err(),
                "{text}"
            );
        }
    }

    #[test]
    fn validates_alias_paths_with_the_effective_inherited_base_url() {
        let directory = tempfile::tempdir().unwrap();
        let mirror = directory.path();
        let base = mirror.join("tsconfig.base.json");
        let nested = mirror.join("src/tsconfig.json");
        let configs = [base.clone(), nested.clone()].into();
        let nested_config = r#"{
            "extends": "../tsconfig.base.json",
            "compilerOptions": {"paths":{"@app/*":["../shared/*"]}}
        }"#;
        for (url, valid) in [(".", false), ("./src", true)] {
            let parent = json!({"compilerOptions":{"baseUrl":url}}).to_string();
            let projects = [
                (
                    base.clone(),
                    prepare_config(&parent, &base, mirror, &configs).unwrap(),
                ),
                (
                    nested.clone(),
                    prepare_config(nested_config, &nested, mirror, &configs).unwrap(),
                ),
            ]
            .into();
            let mut rejected = BTreeMap::new();
            check_projects(&projects, mirror, &mut rejected);
            assert_eq!(rejected.is_empty(), valid, "{url}");
        }
        let self_reference = prepare_config(
            r#"{"extends":"./tsconfig.base.json"}"#,
            &base,
            mirror,
            &configs,
        )
        .unwrap();
        let projects = [(base.clone(), self_reference)].into();
        let mut rejected = BTreeMap::new();
        check_projects(&projects, mirror, &mut rejected);
        assert!(rejected[&base].contains("Cyclic"));
    }

    #[test]
    fn sends_the_lsp_language_for_each_ecmascript_dialect() {
        let profile = TypeScript;
        for (path, grammar, lsp) in [
            ("app.tsx", "tsx", "typescriptreact"),
            ("app.jsx", "javascript", "javascriptreact"),
            ("app.mts", "typescript", "typescript"),
            ("app.mjs", "javascript", "javascript"),
        ] {
            assert_eq!(profile.document_language(Path::new(path), grammar), lsp);
        }
    }

    #[test]
    fn starts_the_native_server_with_external_code_and_typing_downloads_disabled() {
        let mirror = tempfile::tempdir().unwrap();
        let setup = prepare(mirror.path(), &[]).unwrap().setup;
        assert_eq!(setup.arguments, ["--lsp", "--stdio"]);
        assert_eq!(setup.initialization["runExternalCode"], false);
        assert_eq!(
            setup.configuration["js/ts"]["tsserver"]["automaticTypeAcquisition"]["enabled"],
            false
        );
        assert_eq!(setup.document_sync, DocumentSync::Ordered);
        let config: Value =
            serde_json::from_slice(&fs::read(mirror.path().join("tsconfig.json")).unwrap())
                .unwrap();
        assert_eq!(config["compilerOptions"]["allowJs"], true);
    }
}
