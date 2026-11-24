//! JDT LS over explicit source roots in a captured project.
use super::{
    gradle,
    inputs::{InputFiles, Inputs, PreparedInputs},
    project::Profile,
};
use crate::{
    dependency_sources::{ARCHIVE_BYTES, FILE_BYTES, Input, open_archive},
    java_archive::{self, destination as archive_destination},
    lsp::{Client, DocumentSync, Readiness, Setup},
};
use serde_json::json;
use std::{
    collections::BTreeSet,
    ffi::OsStr,
    fs,
    io::Read,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use vizzi_core::{
    model::{Document, Snapshot},
    policy::{Classification, JAVA_PROJECT_METADATA, SourceOnlyReason},
};

pub(super) struct JdtLs {
    pub roots: BTreeSet<String>,
    pub libraries: Vec<PathBuf>,
}

impl Profile for JdtLs {
    fn languages(&self) -> &'static [&'static str] {
        &["java"]
    }

    fn includes_path(&self, path: &str) -> bool {
        let parent = Path::new(path).parent();
        self.roots
            .iter()
            .any(|root| root == "." || parent.is_some_and(|parent| parent.starts_with(root)))
    }

    fn inputs<'a>(&'a self, root: &'a Path, snapshot: &'a Snapshot) -> Result<Inputs<'a>, String> {
        let mut captured = blake3::Hasher::new();
        captured.update(
            serde_json::to_string(&self.roots)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        );
        // Rebuild the project index after edits, including changes to unopened files.
        let mut first_source = None;
        for doc in snapshot
            .documents()
            .iter()
            .filter(|doc| self.includes_document(doc))
        {
            first_source.get_or_insert(doc.path());
            captured.update(&(doc.path().len() as u64).to_le_bytes());
            captured.update(doc.path().as_bytes());
            captured.update(doc.content().as_str().as_bytes());
        }
        let libraries = project_libraries(root, &self.libraries, &mut captured)?;
        Ok(InputFiles::default().freeze(
            &format!("jdtls:{}", captured.finalize().to_hex()),
            &[include_bytes!("java.rs")],
            move |mirror| prepare(mirror, &self.roots, libraries, first_source, None),
        ))
    }

    fn java_project_kind(&self, snapshot: &Snapshot) -> Option<&'static str> {
        let has = |name| {
            snapshot
                .file_by_path(name)
                .is_some_and(|doc| is_build_input(doc))
        };
        let maven = has("pom.xml");
        let gradle = [
            "settings.gradle",
            "settings.gradle.kts",
            "build.gradle",
            "build.gradle.kts",
        ]
        .into_iter()
        .any(has);
        match (maven, gradle) {
            (true, false) => Some("Maven"),
            (false, true) => Some("Gradle"),
            _ => None,
        }
    }

    fn imported_inputs<'a>(
        &'a self,
        root: &'a Path,
        snapshot: &'a Snapshot,
    ) -> Result<Inputs<'a>, String> {
        let kind = self
            .java_project_kind(snapshot)
            .ok_or("Select a workspace with a captured Maven or Gradle build at its root")?;
        let gradle = (kind == "Gradle")
            .then(|| gradle::installation(root, snapshot))
            .transpose()?;
        let mut hash = blake3::Hasher::new();
        if let Some(installation) = &gradle {
            installation.fingerprint(&mut hash);
        }
        let mut descriptors = Vec::new();
        let mut bytes = 0;
        for doc in snapshot
            .documents()
            .iter()
            .filter(|doc| is_build_input(doc))
        {
            bytes += doc.bytes();
            if doc.bytes() > 2 * 1024 * 1024 || bytes > 16 * 1024 * 1024 {
                return Err("Captured Java build descriptors exceed allowance".into());
            }
            hash.update(&(doc.path().len() as u64).to_le_bytes());
            hash.update(doc.path().as_bytes());
            hash.update(doc.content().as_str().as_bytes());
            descriptors.push(doc.as_ref());
        }
        Ok(InputFiles::default().freeze(
            &format!("jdtls-import:{kind}:{}", hash.finalize().to_hex()),
            &[include_bytes!("java.rs"), include_bytes!("gradle.rs")],
            move |mirror| {
                for doc in descriptors {
                    let path = mirror.join(doc.path());
                    fs::create_dir_all(path.parent().ok_or("Build descriptor has no directory")?)
                        .map_err(|e| e.to_string())?;
                    let data = doc
                        .captured()
                        .read_all(2 * 1024 * 1024)
                        .map_err(|e| e.to_string())?;
                    fs::write(path, data.bytes()).map_err(|e| e.to_string())?;
                }
                prepare(
                    mirror,
                    &self.roots,
                    json!([]),
                    None,
                    Some((kind, gradle.as_ref())),
                )
            },
        ))
    }

    fn verify_import(
        &self,
        client: &mut Client,
        mirror: &Path,
        snapshot: &Snapshot,
    ) -> Result<(), String> {
        const NATURES: &str = "org.eclipse.jdt.ls.core.natureIds";
        let kind = self.java_project_kind(snapshot);
        let nature = match kind {
            Some("Maven") => "org.eclipse.m2e.core.maven2Nature",
            Some("Gradle") => "org.eclipse.buildship.core.gradleprojectnature",
            _ => return Err("Java project import is unavailable".into()),
        };
        let projects = client.request(
            "workspace/executeCommand",
            json!({"command":"java.project.getAll","arguments":[]}),
            Duration::from_secs(10),
        )?;
        let projects = projects
            .as_array()
            .ok_or("JDT LS did not report imported projects")?;
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut loaded = false;
        let local_projects = projects
            .iter()
            .filter_map(serde_json::Value::as_str)
            .filter(|project| {
                url::Url::parse(project)
                    .ok()
                    .and_then(|uri| uri.to_file_path().ok())
                    .is_some_and(|path| path.starts_with(mirror))
            });
        for (index, project) in local_projects.enumerate() {
            if index == 32 {
                return Err(
                    "Java classpath verification exceeds the project count allowance".into(),
                );
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err("Java classpath verification timed out".into());
            }
            let Ok(settings) = client.request(
                "workspace/executeCommand",
                json!({"command":"java.project.getSettings","arguments":[project,[NATURES]]}),
                remaining,
            ) else {
                continue;
            };
            if let Some(natures) = settings[NATURES].as_array()
                && natures.iter().any(|value| value == nature)
                && natures
                    .iter()
                    .any(|value| value == "org.eclipse.jdt.core.javanature")
            {
                loaded = true;
                if kind == Some("Maven") {
                    verify_maven_classpath(client, project, deadline)?;
                } else {
                    return Ok(());
                }
            }
        }
        if let Some(error) = client.maven_error(mirror) {
            return Err(format!("Maven classpath import is incomplete. {error}"));
        }
        if loaded {
            return Ok(());
        }
        Err(
            "JDT LS did not load a Java classpath. Check the local build cache and import again."
                .into(),
        )
    }

    fn archive_input(
        &self,
        client: &mut Client,
        uri: &url::Url,
        configuration: &str,
    ) -> Result<Option<Input>, String> {
        if archive_destination(uri).is_none() {
            return Ok(None);
        }
        let attachment = client.request(
            "workspace/executeCommand",
            json!({
                "command": "java.project.resolveSourceAttachment",
                "arguments": [{"classFileUri": uri.as_str()}]
            }),
            Duration::from_secs(10),
        )?;
        let attributes = &attachment["attributes"];
        let Some(source) = attributes["sourceAttachmentPath"].as_str().map(Path::new) else {
            return Ok(None);
        };
        let binary = attributes["jarPath"]
            .as_str()
            .map(Path::new)
            .ok_or("JDT LS did not identify the library")?;
        if !java_archive::is_source_archive(source) {
            return Ok(None);
        }
        let text = archive_text(client, uri.as_str())?;
        java_archive::input(uri, source, binary, &text, configuration).map(Some)
    }

    fn verify_archive(&self, client: &mut Client, uri: &str, text: &str) -> Result<(), String> {
        if archive_text(client, uri)? == text {
            Ok(())
        } else {
            Err("Archive source differs from the current JDT LS document".into())
        }
    }
}

fn verify_maven_classpath(
    client: &mut Client,
    project: &str,
    deadline: Instant,
) -> Result<(), String> {
    let remaining = deadline.saturating_duration_since(Instant::now());
    if remaining.is_zero() {
        return Err("Java classpath verification timed out".into());
    }
    let paths = client.request(
        "workspace/executeCommand",
        json!({"command":"java.project.getClasspaths","arguments":[project,{"scope":"test"}]}),
        remaining,
    )?;
    for field in ["classpaths", "modulepaths"] {
        for value in paths[field]
            .as_array()
            .ok_or("JDT LS did not report the Maven classpath")?
        {
            let path = Path::new(
                value
                    .as_str()
                    .ok_or("JDT LS returned an invalid Maven classpath entry")?,
            );
            if matches!(
                path.extension().and_then(OsStr::to_str),
                Some("jar" | "zip")
            ) && !path.is_file()
            {
                return Err(format!(
                    "Maven classpath import is incomplete. Missing {}",
                    path.display()
                ));
            }
        }
    }
    Ok(())
}

fn is_build_input(doc: &Document) -> bool {
    matches!(doc.classification(), Classification::SourceOnly {
        language: Some(language), reason: SourceOnlyReason::ProjectMetadata, ..
    } if language == JAVA_PROJECT_METADATA)
}

fn project_libraries(
    root: &Path,
    paths: &[PathBuf],
    captured: &mut blake3::Hasher,
) -> Result<serde_json::Value, String> {
    let mut binaries = BTreeSet::new();
    let mut sources = serde_json::Map::new();
    for path in paths {
        let binary = super::inputs::absolute(root, path);
        let library = binary.to_str().ok_or("Java library path is not UTF-8")?;
        if library.contains(['*', '?', '[', ']', '{', '}'])
            || (!cfg!(windows) && library.contains('\\'))
        {
            return Err(format!(
                "Java library path contains glob characters: {library}"
            ));
        }
        let stem = binary
            .file_stem()
            .and_then(|name| name.to_str())
            .ok_or("Java library needs a UTF-8 file name")?;
        if binary.extension().and_then(|ext| ext.to_str()) != Some("jar") {
            return Err("--java-library expects a local JAR file".into());
        }
        if binaries.insert(binary.clone()) {
            fingerprint_library(&binary, captured)?;
            let attachment = binary.with_file_name(format!("{stem}-sources.jar"));
            if attachment.is_file() {
                fingerprint_library(&attachment, captured)?;
                sources.insert(
                    library.into(),
                    json!(attachment.to_str().ok_or("Java source path is not UTF-8")?),
                );
            }
        }
    }
    let include = binaries
        .iter()
        .map(|path| path.to_str().ok_or("Java library path is not UTF-8"))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(json!({"include": include, "sources": sources}))
}

fn fingerprint_library(path: &Path, hash: &mut blake3::Hasher) -> Result<(), String> {
    let resolved = path
        .canonicalize()
        .map_err(|e| format!("Java library {}: {e}", path.display()))?;
    let file = open_archive(&resolved)?;
    let mut content = blake3::Hasher::new();
    if std::io::copy(&mut file.take(ARCHIVE_BYTES + 1), &mut content).map_err(|e| e.to_string())?
        > ARCHIVE_BYTES
    {
        return Err("Java library exceeds allowance".into());
    }
    let name = path.as_os_str().as_encoded_bytes();
    hash.update(&(name.len() as u64).to_le_bytes());
    hash.update(name);
    hash.update(content.finalize().as_bytes());
    Ok(())
}

fn archive_text(client: &mut Client, uri: &str) -> Result<String, String> {
    let value = client.request(
        "java/classFileContents",
        json!({"uri":uri}),
        Duration::from_secs(10),
    )?;
    let serde_json::Value::String(text) = value else {
        return Err("JDT LS source content is unavailable".into());
    };
    if text.is_empty() || text.len() > FILE_BYTES {
        return Err("JDT LS source content exceeds allowance or is empty".into());
    }
    Ok(text)
}

pub(super) fn source_roots(roots: &[String]) -> Result<BTreeSet<String>, String> {
    if roots.is_empty() {
        return Ok(BTreeSet::from([".".into()]));
    }
    roots
        .iter()
        .map(|root| {
            if root.is_empty()
                || root.starts_with('/')
                || root.contains(['\\', ':', '\0'])
                || root.split('/').any(|part| part == "..")
            {
                return Err("--java-source-root requires a workspace-relative path".into());
            }
            let path = root
                .split('/')
                .filter(|part| !matches!(*part, "" | "."))
                .collect::<Vec<_>>()
                .join("/");
            Ok(if path.is_empty() { ".".into() } else { path })
        })
        .collect()
}

fn toolkit_path(path: &Path) -> Result<&str, String> {
    path.to_str()
        .ok_or_else(|| "Gradle installation paths must be UTF-8".into())
}

fn prepare(
    mirror: &Path,
    roots: &BTreeSet<String>,
    libraries: serde_json::Value,
    first_source: Option<&str>,
    project: Option<(&str, Option<&gradle::Installation>)>,
) -> Result<PreparedInputs, String> {
    let private_state = tempfile::Builder::new()
        .prefix("jdtls-")
        .tempdir_in(
            mirror
                .parent()
                .ok_or("Java semantic mirror has no parent")?,
        )
        .map_err(|e| e.to_string())?;
    let argument = |name| -> Result<String, String> {
        let path: PathBuf = private_state.path().join(name);
        path.into_os_string()
            .into_string()
            .map_err(|_| "JDT LS state path is not UTF-8".into())
    };
    let (kind, gradle) = project.unwrap_or_default();
    let gradle_home = gradle.map(|value| toolkit_path(&value.home)).transpose()?;
    let gradle_user_home = gradle
        .map(|value| toolkit_path(&value.user_home))
        .transpose()?;
    let gradle_java_home = gradle
        .and_then(|value| value.java_home.as_deref())
        .map(toolkit_path)
        .transpose()?;
    let settings = json!({
        "java": {
            "autobuild": {"enabled": false},
            "configuration": {"updateBuildConfiguration": "disabled"},
            "import": {
                "eclipse": {"enabled": false},
                "gradle": {
                    "enabled": kind == "Gradle",
                    "home": gradle_home,
                    "user": {"home": gradle_user_home},
                    "java": {"home": gradle_java_home},
                    "offline": {"enabled": true},
                    "wrapper": {"enabled": false},
                    "annotationProcessing": {"enabled": false}
                },
                "maven": {
                    "enabled": kind == "Maven",
                    "offline": {"enabled": true}
                }
            },
            "maven": {"downloadSources": false},
            "project": {"sourcePaths": roots, "referencedLibraries": libraries},
            "telemetry": {"enabled": false}
        }
    });
    let mut initialization = json!({
        "settings": settings,
        "extendedClientCapabilities": {"classFileContentsSupport": true}
    });
    if let Some(source) = first_source {
        initialization["triggerFiles"] = json!([crate::lsp::uri(&mirror.join(source))?]);
    }
    let setup = Setup {
        arguments: vec![
            "-configuration".into(),
            argument("configuration")?,
            "-data".into(),
            argument("workspace")?,
        ],
        initialization,
        configuration: settings,
        readiness: Readiness::JdtLs,
        document_sync: DocumentSync::Ordered,
        environment: ["CLIENT_PORT", "CLIENT_HOST", "CLIENT_PIPE"]
            .into_iter()
            .map(|name| (name.into(), None))
            .collect(),
    };
    Ok(PreparedInputs {
        private_state: Some(private_state),
        ..setup.into()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradle_import_resolves_an_installed_distribution_and_rejects_an_invalid_home() {
        let directory = tempfile::tempdir().unwrap();
        let home = directory.path().join("gradle");
        let bin = home.join("bin");
        fs::create_dir_all(&bin).unwrap();
        let launcher = if cfg!(windows) {
            "gradle.bat"
        } else {
            "gradle"
        };
        fs::write(bin.join(launcher), "").unwrap();
        let path = std::env::join_paths([&bin]).unwrap();
        assert!(gradle::local_gradle_home(Some(home.as_os_str()), None).is_err());
        assert!(gradle::local_gradle_home(None, Some(&path)).is_err());
        fs::create_dir(home.join("lib")).unwrap();
        fs::write(home.join("lib/other.jar"), "").unwrap();
        fs::create_dir(home.join("lib/gradle-launcher-directory.jar")).unwrap();
        assert!(gradle::local_gradle_home(None, Some(&path)).is_err());
        fs::write(home.join("lib/gradle-launcher-test.jar"), "").unwrap();
        assert_eq!(
            gradle::local_gradle_home(None, Some(&path)).unwrap(),
            home.canonicalize().unwrap()
        );
        assert!(
            gradle::local_gradle_home(Some(directory.path().as_os_str()), Some(&path)).is_err()
        );
        assert!(gradle::local_gradle_home(None, None).is_err());
    }

    #[test]
    fn archive_destinations_decode_java_segments_and_reject_invalid_paths() {
        let destination = |value: &str| archive_destination(&url::Url::parse(value).unwrap());
        assert_eq!(
            destination("jdt://contents/java.base/java.lang/Str%C3%A9.java?h"),
            Some(("java.base".into(), "java.lang".into(), "Stré.java".into()))
        );
        assert_eq!(
            destination("jdt://contents/helper.jar//Top$Nested.class?h"),
            Some(("helper.jar".into(), "".into(), "Top.java".into()))
        );
        assert_eq!(
            destination("jdt://contents/helper.jar/Helper.java?h"),
            Some(("helper.jar".into(), "".into(), "Helper.java".into()))
        );
        for value in [
            "https://contents/helper.jar/lib/Tool.java",
            "jdt://other/helper.jar/lib/Tool.java",
            "jdt://contents/helper.jar/../Tool.java",
            "jdt://contents/helper.jar/lib/%2FTool.java",
            "jdt://contents/helper.jar/lib..sub/Tool.java",
            "jdt://contents/helper.jar/lib/Tool.txt",
            "jdt://contents/helper.jar/lib/Tool.java/extra",
        ] {
            assert!(destination(value).is_none(), "{value}");
        }
    }

    #[test]
    #[cfg(unix)]
    fn library_paths_reject_globs_even_when_a_literal_file_exists() {
        let root = tempfile::tempdir().unwrap();
        for name in ["helper?.jar", "helper*.jar", "lib[1]/helper.jar"] {
            let path = root.path().join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(&path, b"literal file").unwrap();
            let error = project_libraries(root.path(), &[name.into()], &mut blake3::Hasher::new())
                .unwrap_err();
            assert!(error.contains("glob"), "{error}");
        }
    }

    #[test]
    fn source_roots_default_to_the_workspace_and_normalize_explicit_paths() {
        assert_eq!(source_roots(&[]).unwrap(), BTreeSet::from([".".into()]));
        let roots = ["src//main/java", "./src/test/java/", "src/main/java"].map(String::from);
        assert_eq!(
            source_roots(&roots).unwrap(),
            BTreeSet::from(["src/main/java".into(), "src/test/java".into()])
        );
    }

    #[test]
    fn source_roots_reject_absolute_paths_and_parent_traversal() {
        for path in [
            "",
            "/src",
            "src/../outside",
            "C:/src",
            "src\\test",
            "src\0test",
        ] {
            assert!(source_roots(&[path.into()]).is_err(), "{path:?}");
        }
    }
}
