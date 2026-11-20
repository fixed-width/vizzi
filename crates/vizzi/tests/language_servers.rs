//! Language-aware semantic providers use captured source rather than name ranking.
use std::{collections::BTreeMap, sync::Arc};
use vizzi::{
    definitions::Provider,
    language_servers::{Configuration, Servers},
};
use vizzi_core::{
    analysis::Languages,
    model::{CapturePolicyId, Document, Snapshot, WorkspaceId},
    policy::{Classification, Provenance},
    structure::Structure,
};

fn snapshot(files: &[(&str, &str, &str)]) -> Arc<Snapshot> {
    let scope = WorkspaceId::derive(&[b"language server integration"]);
    let languages = Languages::builtin().unwrap();
    let mut parsers = BTreeMap::new();
    let docs = files
        .iter()
        .map(|(path, language, text)| {
            let parser = parsers
                .entry(*language)
                .or_insert_with(|| languages.select(language).unwrap().create().unwrap());
            Arc::new(
                Document::capture(
                    scope.clone(),
                    (*path).into(),
                    text.as_bytes(),
                    vec![parser.analyze(text.as_bytes()).unwrap()],
                    Classification::Assigned {
                        language: (*language).into(),
                        provenance: Provenance::ExplicitScope { scope: ".".into() },
                    },
                )
                .unwrap(),
            )
        })
        .collect();
    Arc::new(Snapshot::new(scope, docs, CapturePolicyId::derive(&[b"fixture"])).unwrap())
}

#[test]
fn configuration_is_lazy_and_unconfigured_languages_do_not_use_another_server() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    assert!(Configuration::default().is_empty());
    for configuration in [
        Configuration {
            rust_analyzer: Some("rust-analyzer".into()),
            ..Default::default()
        },
        Configuration {
            pyright: Some("pyright-langserver".into()),
            ..Default::default()
        },
        Configuration {
            clangd: Some("clangd".into()),
            ..Default::default()
        },
        Configuration {
            ruby_lsp: Some("ruby-lsp".into()),
            ..Default::default()
        },
    ] {
        assert!(!configuration.is_empty());
    }
    assert!(
        Servers::new(
            root.path().into(),
            cache.path().into(),
            Configuration {
                python_path: Some("python3".into()),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        Servers::new(
            root.path().into(),
            cache.path().into(),
            Configuration {
                compilation_database: Some("commands.json".into()),
                ..Default::default()
            }
        )
        .is_err()
    );
    let server = Servers::new(
        root.path().into(),
        cache.path().into(),
        Configuration {
            pyright: Some(root.path().join("must-not-start")),
            ..Default::default()
        },
    )
    .unwrap();
    let captured = snapshot(&[(
        "a.c",
        "c",
        "int leaf(){return 1;} int run(){return leaf();}",
    )]);
    let index = Structure::new(&captured);
    let owner = index
        .functions("a.c", "run", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    let call = index.outgoing(&owner.id, "a.c", 0).unwrap().items.remove(0);
    assert!(
        server
            .resolve(captured.clone(), &index.call(&call.id).unwrap())
            .unwrap_err()
            .contains("No language server configured for c")
    );
}

#[test]
fn managed_typescript_resolves_captured_aliases_references_and_cross_file_edits() {
    use vizzi::{cache::Cache, engine::Engine, source::Directory};
    let Some(executable) = std::env::var_os("VIZZI_TYPESCRIPT_SERVER") else {
        eprintln!("Set VIZZI_TYPESCRIPT_SERVER to exercise the managed native TypeScript server");
        return;
    };
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let config = |file: &str| {
        serde_json::json!({"compilerOptions":{
            "allowJs":true, "jsx":"preserve", "module":"ESNext", "moduleResolution":"Bundler",
            "paths":{"@service":[file]}
        }})
        .to_string()
    };
    let write = |path: &str, text: &str| std::fs::write(root.path().join(path), text).unwrap();
    write(
        "tsconfig.json",
        "{ // frozen project\n \"extends\": \"./tsconfig.base\",\n}",
    );
    write("tsconfig.base.json", &config("./old.ts"));
    write(
        "app.tsx",
        "import { make as select } from '@service';\nexport function launch() { const icon = '😀'; const tool = select(); return <div>{icon}{tool.run()}</div>; }",
    );
    write(
        "view.jsx",
        "import { make } from './old';\nexport function show() { return <span>{make().run()}</span>; }",
    );
    let package = |file: &str| {
        serde_json::json!({"name":"fixture-service", "exports":{"./tool":file}}).to_string()
    };
    write("package.json", &package("./old.ts"));
    write(
        "package-user.ts",
        "import { make } from 'fixture-service/tool';\nexport function packageCall() { return make().run(); }",
    );
    write(
        "old.ts",
        "export class Old { \"run\"() {} }\nexport const make = () => new Old();",
    );
    write(
        "new.ts",
        "export class New { run() {} }\nclass Next { run() {} }\nexport const make = () => new New();",
    );
    let engine = Engine::new(
        Directory::open(root.path()).unwrap(),
        Cache::new(cache.path().join("raw")),
        1,
    )
    .unwrap();
    let capture = || engine.refresh(None, &|| false).unwrap().unwrap().snapshot;
    let original = capture();
    let provider = Servers::new(
        root.path().into(),
        cache.path().join("semantic"),
        Configuration {
            typescript_server: Some(executable.into()),
            ..Default::default()
        },
    )
    .unwrap();
    write("tsconfig.base.json", &config("./new.ts"));
    write("package.json", &package("./new.ts"));
    let resolve = |snapshot: &Arc<Snapshot>, path: &str, caller: &str, name: &str| {
        let index = Structure::new(snapshot);
        let function = index
            .functions(path, caller, 0)
            .unwrap()
            .items
            .remove(0)
            .symbol;
        let invocation = index
            .outgoing(&function.id, path, 0)
            .unwrap()
            .items
            .into_iter()
            .find(|item| item.name.as_deref() == Some(name))
            .unwrap();
        let answer = provider
            .resolve(snapshot.clone(), &index.call(&invocation.id).unwrap())
            .unwrap();
        assert_eq!(answer.locations.len(), 1, "{answer:?}");
        let location = &answer.locations[0];
        let target = snapshot
            .file_by_path(&location.path)
            .unwrap()
            .resident_source()
            .unwrap();
        let at = vizzi::definitions::byte_offset(target, location.position).unwrap();
        (
            index.definition(&location.path, at).unwrap(),
            answer.context,
        )
    };
    let (alias, context) = resolve(&original, "app.tsx", "launch", "select");
    assert_eq!(alias.name, "make");
    assert_eq!(alias.source.path, "old.ts");
    let (old, _) = resolve(&original, "app.tsx", "launch", "run");
    assert_eq!(old.parents, ["Old"]);
    let refs = provider
        .references(
            original.clone(),
            &old,
            Structure::new(&original).identifier(&old.id).unwrap(),
        )
        .unwrap();
    assert!(
        refs.locations
            .iter()
            .any(|location| location.path == "app.tsx"),
        "{refs:?}"
    );
    assert_eq!(
        resolve(&original, "view.jsx", "show", "run").0.parents,
        ["Old"]
    );
    assert_eq!(
        resolve(&original, "package-user.ts", "packageCall", "run")
            .0
            .parents,
        ["Old"]
    );

    let updated = capture();
    let (new, new_context) = resolve(&updated, "app.tsx", "launch", "run");
    assert_eq!(new.parents, ["New"]);
    assert_eq!(
        resolve(&updated, "package-user.ts", "packageCall", "run")
            .0
            .parents,
        ["New"]
    );
    assert_ne!(context, new_context);
    write(
        "new.ts",
        "export class New { run() {} }\nclass Next { run() {} }\nexport const make = () => new Next();",
    );
    let edited = capture();
    assert_eq!(
        resolve(&edited, "app.tsx", "launch", "run").0.parents,
        ["Next"]
    );
}

#[test]
#[cfg(unix)]
fn managed_typescript_rechecks_reached_package_exports_when_the_manifest_changes() {
    let Some(fixture) = TypeScriptFixture::new(&[
        (
            "app.ts",
            "import { work } from 'edited-pkg';\nexport function launch() { return work(); }",
        ),
        (
            "node_modules/edited-pkg/package.json",
            r#"{"name":"edited-pkg","version":"1.0.0","exports":"./old.ts"}"#,
        ),
        (
            "node_modules/edited-pkg/old.ts",
            "export function work() { return 1; }",
        ),
        (
            "node_modules/edited-pkg/new.ts",
            "export function work() { return 2; }",
        ),
    ]) else {
        return;
    };
    let before = fixture.call("app.ts").unwrap();
    assert_eq!(before.locations[0].path, "old.ts");
    let before_id = before.locations[0].capture.as_deref().unwrap();
    let package = fixture
        .directory
        .path()
        .join("project/node_modules/edited-pkg");
    std::fs::write(
        package.join("package.json"),
        r#"{"name":"edited-pkg","version":"2.0.0","exports":"./new.ts"}"#,
    )
    .unwrap();
    let after = fixture.call("app.ts").unwrap();
    assert_eq!(after.locations[0].path, "new.ts");
    let catalog = fixture.provider.dependencies().unwrap();
    let old = catalog.get(before_id).unwrap().unwrap();
    let new = catalog
        .get(after.locations[0].capture.as_deref().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(old.origin.version.as_deref(), Some("1.0.0"));
    assert_eq!(new.origin.version.as_deref(), Some("2.0.0"));
    std::fs::write(
        package.join("package.json"),
        r#"{"name":"edited-pkg","version":"2.0.0","exports":"./old.ts"}"#,
    )
    .unwrap();
    let same_version_edit = fixture.call("app.ts").unwrap();
    assert_eq!(same_version_edit.locations[0].path, "old.ts");
    assert_ne!(
        same_version_edit.locations[0].capture.as_deref(),
        Some(before_id)
    );
}

#[test]
#[cfg(unix)]
fn managed_typescript_restores_the_importing_project_for_package_references() {
    let Some(fixture) = TypeScriptFixture::new(&[
        (
            "app.ts",
            "import { work } from 'restored-pkg';\nexport function launch() { return work(); }",
        ),
        (
            "node_modules/restored-pkg/package.json",
            r#"{"name":"restored-pkg","exports":"./entry.ts"}"#,
        ),
        (
            "node_modules/restored-pkg/entry.ts",
            "export function work() { return 7; }",
        ),
    ]) else {
        return;
    };
    let definition = fixture.call("app.ts").unwrap();
    let id = definition.locations[0].capture.as_deref().unwrap();
    let fresh = Servers::new(
        fixture.directory.path().join("project"),
        fixture.directory.path().join("semantic"),
        Configuration {
            typescript_server: std::env::var_os("VIZZI_TYPESCRIPT_SERVER").map(Into::into),
            ..Default::default()
        },
    )
    .unwrap();
    let restored = fresh.dependencies().unwrap().get(id).unwrap().unwrap();
    let target = restored
        .index
        .functions("", "work", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    let target = restored.symbol(target);
    let identifier = restored.index.identifier(&target.id).unwrap();
    let answer = fresh
        .references(fixture.snapshot.clone(), &target, identifier)
        .unwrap();
    assert!(
        answer
            .locations
            .iter()
            .any(|location| location.path == "app.ts" && location.position.line == 1),
        "{answer:?}"
    );
}

#[test]
#[cfg(unix)]
fn managed_typescript_follows_installed_packages_and_retains_the_resolution_path() {
    let Some(fixture) = TypeScriptFixture::new(&[
        (
            "tsconfig.json",
            r#"{"compilerOptions":{"allowJs":true,"maxNodeModuleJsDepth":1,"module":"ESNext","moduleResolution":"Bundler","preserveSymlinks":true}}"#,
        ),
        (
            "apps/client/app.ts",
            "import { work } from '@demo/router';\nexport function launch() { return work(); }",
        ),
        (
            "apps/client/node_modules/@demo/router/package.json",
            r#"{"name":"@demo/router","version":"1.2.3","exports":"./entry.mjs"}"#,
        ),
        (
            "apps/client/node_modules/@demo/router/entry.mjs",
            "import { downstream } from 'worker-pkg';\nexport function work() { return downstream(); }",
        ),
        (
            "node_modules/worker-pkg/package.json",
            r#"{"name":"worker-pkg","version":"4.5.6","exports":"./entry.ts"}"#,
        ),
        (
            "node_modules/worker-pkg/entry.ts",
            "export function downstream() { return 7; }",
        ),
    ]) else {
        return;
    };
    assert_eq!(fixture.snapshot.documents().len(), 2);
    let answer = fixture.call("apps/client/app.ts").unwrap();
    assert_eq!(answer.locations.len(), 1, "{answer:?}");
    assert!(answer.locations[0].capture.is_some(), "{answer:?}");
    let catalog = fixture.provider.dependencies().unwrap();
    let open = |location: &vizzi::definitions::Location| {
        let capture = catalog
            .get(location.capture.as_deref().unwrap())
            .unwrap()
            .unwrap();
        let at = vizzi::definitions::byte_offset(
            capture.doc().resident_source().unwrap(),
            location.position,
        )
        .unwrap();
        let symbol = capture.index.definition(capture.doc().path(), at).unwrap();
        let symbol = capture.symbol(symbol);
        (capture, symbol)
    };
    let (router, work) = open(&answer.locations[0]);
    assert_eq!(router.origin.label, "@demo/router");
    assert_eq!(router.origin.version.as_deref(), Some("1.2.3"));
    assert_eq!(
        router.input.mirror_path.as_deref(),
        Some("apps/client/node_modules/@demo/router/entry.mjs")
    );
    assert_eq!(work.name, "work");
    let site = router.calls(&work.id, 0).unwrap().items.remove(0);
    let next = fixture
        .provider
        .resolve(
            fixture.snapshot.clone(),
            &router.index.call(&site.id).unwrap(),
        )
        .unwrap();
    assert_eq!(next.locations.len(), 1, "{next:?}");
    assert!(next.locations[0].capture.is_some(), "{next:?}");
    let (worker, downstream) = open(&next.locations[0]);
    assert_eq!(worker.origin.label, "worker-pkg");
    assert_eq!(worker.origin.version.as_deref(), Some("4.5.6"));
    assert_eq!(downstream.name, "downstream");
    let references = fixture
        .provider
        .references(
            fixture.snapshot.clone(),
            &downstream,
            worker.index.identifier(&downstream.id).unwrap(),
        )
        .unwrap();
    assert!(
        references
            .locations
            .iter()
            .any(|location| location.capture.as_deref() == Some(router.origin.capture.as_str())),
        "{references:?}"
    );
    std::fs::write(
        router.path(),
        "// moved\nexport function work() { return 8; }",
    )
    .unwrap();
    assert!(router.verify().is_err());
    assert!(
        router
            .doc()
            .resident_source()
            .unwrap()
            .contains("downstream()")
    );
    let refreshed = fixture.call("apps/client/app.ts").unwrap();
    assert_eq!(refreshed.locations.len(), 1, "{refreshed:?}");
    let (new, updated) = open(&refreshed.locations[0]);
    assert_ne!(new.origin.capture, router.origin.capture);
    assert_eq!(updated.source.line, 2);
}

#[test]
#[cfg(unix)]
fn managed_typescript_admits_installed_declarations_using_local_package_exports() {
    let Some(fixture) = TypeScriptFixture::new(&[
        (
            "app.ts",
            "import { work } from 'typed-pkg';\nexport function launch() { return work(); }",
        ),
        (
            "node_modules/typed-pkg/package.json",
            r#"{"name":"typed-pkg","version":"1.0.0","exports":{"types":"./entry.d.ts","default":"./entry.js"}}"#,
        ),
        (
            "node_modules/typed-pkg/entry.d.ts",
            "export function work(): number;",
        ),
        (
            "node_modules/typed-pkg/entry.js",
            "export function work() { return 3; }",
        ),
    ]) else {
        return;
    };
    let answer = fixture.call("app.ts").unwrap();
    assert_eq!(answer.locations.len(), 1, "{answer:?}");
    let target = &answer.locations[0];
    let catalog = fixture.provider.dependencies().unwrap();
    let capture = catalog
        .get(target.capture.as_deref().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(capture.doc().path(), "entry.d.ts");
    let at =
        vizzi::definitions::byte_offset(capture.doc().resident_source().unwrap(), target.position)
            .unwrap();
    let symbol = capture.index.definition(capture.doc().path(), at).unwrap();
    assert_eq!(symbol.name, "work");
    assert_eq!(symbol.kind, "signature");
}

#[test]
fn managed_typescript_keeps_equal_prefix_path_mappings_in_captured_order() {
    let Some(fixture) = TypeScriptFixture::new(&[
        (
            "tsconfig.json",
            r#"{"compilerOptions":{"module":"ESNext","moduleResolution":"Bundler","paths":{"@service/*.js":["./specific/*"],"@service/*":["./fallback/*"]}}}"#,
        ),
        (
            "app.ts",
            "import { work } from '@service/foo.js';\nexport function launch() { return work(); }",
        ),
        ("specific/foo.ts", "export function work() { return 1; }"),
        ("fallback/foo.ts", "export function work() { return 2; }"),
    ]) else {
        return;
    };
    let answer = fixture.call("app.ts").unwrap();
    assert_eq!(answer.locations.len(), 1, "{answer:?}");
    assert_eq!(answer.locations[0].path, "specific/foo.ts");
}

#[test]
fn managed_typescript_accepts_a_bom_before_the_captured_jsonc() {
    let Some(fixture) = TypeScriptFixture::new(&[
        (
            "tsconfig.json",
            "\u{feff}{ // config written with a BOM\n\"compilerOptions\":{},\n}",
        ),
        (
            "app.ts",
            "function work() {}\nexport function launch() { work(); }",
        ),
    ]) else {
        return;
    };
    assert_eq!(
        fixture.call("app.ts").unwrap().locations[0].position.line,
        0
    );
}

#[test]
fn managed_typescript_reports_config_errors_only_when_the_active_project_needs_them() {
    let Some(fixture) = TypeScriptFixture::new(&[
        (
            "tsconfig.json",
            r#"{"exclude":["broken","plain","solution"]}"#,
        ),
        ("tsconfig.editor.json", r#"{"extends":"@outside/tsconfig"}"#),
        (
            "broken/tsconfig.json",
            r#"{"extends":"../tsconfig.editor.json","include":["**/*.ts"]}"#,
        ),
        (
            "plain/tsconfig.json",
            r#"{"extends":"@outside/unused","files":["excluded.ts"]}"#,
        ),
        (
            "solution/tsconfig.json",
            r#"{"references":[{"path":"../broken"}],"include":["app.ts"]}"#,
        ),
        (
            "app.ts",
            "function work() {}\nexport function launch() { work(); }",
        ),
        (
            "broken/app.ts",
            "function work() {}\nexport function launch() { work(); }",
        ),
        (
            "plain/app.ts",
            "function work() {}\nexport function launch() { work(); }",
        ),
        (
            "solution/app.ts",
            "function work() {}\nexport function launch() { work(); }",
        ),
    ]) else {
        return;
    };
    assert_eq!(fixture.call("app.ts").unwrap().locations[0].path, "app.ts");
    let error = fixture.call("broken/app.ts").unwrap_err();
    assert!(
        error.contains("tsconfig.editor.json") && error.contains("@outside/tsconfig"),
        "{error}"
    );
    let error = fixture.call("solution/app.ts").unwrap_err();
    assert!(
        error.contains("broken") && error.contains("@outside/tsconfig"),
        "{error}"
    );
    assert_eq!(
        fixture.call("plain/app.ts").unwrap().locations[0].path,
        "plain/app.ts"
    );
    assert_eq!(fixture.call("app.ts").unwrap().locations[0].path, "app.ts");
}

#[test]
fn managed_typescript_resolves_a_call_without_requesting_a_large_outline() {
    use std::fmt::Write;
    let mut text = "function work() {}\nexport function launch() { work(); }\n".to_owned();
    for i in 0..43_000 {
        writeln!(text, "function filler_{i:05}() {{}}").unwrap();
    }
    let Some(fixture) = TypeScriptFixture::new(&[
        ("app.ts", &text),
        ("tsconfig.unused.json", r#"{"extends":"@outside/unused"}"#),
    ]) else {
        return;
    };
    let answer = fixture.call("app.ts").unwrap();
    assert_eq!(answer.locations.len(), 1, "{answer:?}");
    assert_eq!(answer.locations[0].position.line, 0);
}

struct TypeScriptFixture {
    provider: Servers,
    snapshot: Arc<Snapshot>,
    directory: tempfile::TempDir,
}

impl TypeScriptFixture {
    fn new(files: &[(&str, &str)]) -> Option<Self> {
        use vizzi::{cache::Cache, engine::Engine, source::Directory};
        let executable = std::env::var_os("VIZZI_TYPESCRIPT_SERVER")?;
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join("project");
        for (path, text) in files {
            let path = root.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
        let engine = Engine::new(
            Directory::open(&root).unwrap(),
            Cache::new(directory.path().join("raw")),
            1,
        )
        .unwrap();
        let snapshot = engine.refresh(None, &|| false).unwrap().unwrap().snapshot;
        let provider = Servers::new(
            root,
            directory.path().join("semantic"),
            Configuration {
                typescript_server: Some(executable.into()),
                ..Default::default()
            },
        )
        .unwrap();
        Some(Self {
            provider,
            snapshot,
            directory,
        })
    }

    fn call(&self, path: &str) -> Result<vizzi::definitions::Answer, String> {
        let index = Structure::new(&self.snapshot);
        let owner = index
            .functions(path, "launch", 0)
            .unwrap()
            .items
            .remove(0)
            .symbol;
        let call = index.outgoing(&owner.id, path, 0).unwrap().items.remove(0);
        self.provider
            .resolve(self.snapshot.clone(), &index.call(&call.id).unwrap())
    }
}

#[test]
fn managed_servers_expose_the_shared_catalog_for_admitted_dependency_source() {
    use vizzi::dependency_sources::Input;
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let dependency = tempfile::tempdir().unwrap();
    let text = "def leaf(): return 1\n";
    std::fs::write(dependency.path().join("leaf.py"), text).unwrap();
    let servers = Servers::new(
        root.path().into(),
        cache.path().into(),
        Configuration {
            pyright: Some("must-not-start".into()),
            rust_analyzer: Some("must-not-start".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let captured = servers
        .dependencies()
        .expect("managed providers expose dependency source")
        .capture(Input {
            root: dependency.path().canonicalize().unwrap(),
            path: "leaf.py".into(),
            language: "python".into(),
            package: "fixture".into(),
            version: None,
            configuration: "a".repeat(64),
            mirror_path: None,
            installation: None,
        })
        .unwrap();
    let retained = servers
        .dependencies()
        .unwrap()
        .get(&captured.origin.capture)
        .unwrap()
        .unwrap();
    assert_eq!(retained.doc().resident_source(), Some(text));
    assert_eq!(retained.doc().path(), "leaf.py");
}

#[cfg(unix)]
#[test]
fn ruby_reopens_changed_captures_and_verifies_definitions_and_references() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let executable = cache.path().join("peer");
    std::fs::write(&executable,r#"#!/usr/bin/python3
import json,sys
opened={}
def read():
    headers={}
    while True:
        line=sys.stdin.buffer.readline()
        if not line:return None
        if line==b'\r\n':break
        k,v=line.decode().split(':',1);headers[k.lower()]=v.strip()
    return json.loads(sys.stdin.buffer.read(int(headers['content-length'])))
def send(value):
    data=json.dumps(value).encode();sys.stdout.buffer.write(('Content-Length: %d\r\n\r\n'%len(data)).encode()+data);sys.stdout.buffer.flush()
def location(uri,text,offset):
    prefix=text[:offset]
    position={'line':prefix.count('\n'),'character':len(prefix.rsplit('\n',1)[-1])}
    return {'uri':uri,'range':{'start':position,'end':position}}
while True:
    request=read()
    if request is None:break
    method=request.get('method');params=request.get('params',{})
    if method=='initialize':send({'id':request['id'],'result':{'capabilities':{'definitionProvider':True,'positionEncoding':'utf-16'},'serverInfo':{'version':'ruby-lifecycle-peer'}}})
    elif method=='initialized':send({'method':'$/progress','params':{'token':'indexing-progress','value':{'kind':'end'}}})
    elif method=='textDocument/didOpen':
        doc=params['textDocument'];assert doc['uri'] not in opened
        assert doc['languageId']=='ruby';opened[doc['uri']]=doc['text']
    elif method=='textDocument/didClose':
        uri=params['textDocument']['uri'];assert uri in opened;del opened[uri]
    elif method=='textDocument/didChange':raise AssertionError('Ruby full captures must reopen')
    elif method=='textDocument/documentSymbol':
        assert params['textDocument']['uri'] in opened;send({'id':request['id'],'result':[]})
    elif method=='textDocument/definition':
        uri=params['textDocument']['uri'];text=opened[uri]
        locations=[location(uri+'s','',0)]
        if 'def process' in text:
            locations.append(location(uri,text,text.index('process')))
        send({'id':request['id'],'result':locations})
    elif method=='textDocument/references':
        uri=params['textDocument']['uri'];text=opened[uri]
        locations=[location(uri,text,text.index('process')),location(uri,text,text.rindex('process'))]
        send({'id':request['id'],'result':locations})
"#).unwrap();
    std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o700)).unwrap();
    let server = Servers::new(
        root.path().into(),
        cache.path().into(),
        Configuration {
            ruby_lsp: Some(executable),
            ..Default::default()
        },
    )
    .unwrap();
    for (definition_line, text) in [
        (
            Some(0),
            "def process(value); value; end\ndef run; process(1); end\n",
        ),
        (
            Some(1),
            "\ndef process(value); value; end\ndef run; process(1); end\n",
        ),
        (None, "def run; process(1); end\n"),
    ] {
        let captured = snapshot(&[("app.rb", "ruby", text)]);
        let index = Structure::new(&captured);
        let owner = index
            .functions("app.rb", "run", 0)
            .unwrap()
            .items
            .remove(0)
            .symbol;
        let site = index
            .outgoing(&owner.id, "app.rb", 0)
            .unwrap()
            .items
            .remove(0);
        let result = server
            .resolve(captured.clone(), &index.call(&site.id).unwrap())
            .unwrap();
        assert_eq!(
            result.locations.len(),
            usize::from(definition_line.is_some())
        );
        assert_eq!(
            result.locations.first().map(|loc| loc.position.line),
            definition_line
        );
        assert_eq!(result.outside, usize::from(definition_line.is_none()));
        if let Some(line) = definition_line {
            let symbol = index
                .functions("app.rb", "process", 0)
                .unwrap()
                .items
                .remove(0)
                .symbol;
            let reference = server
                .references(captured, &symbol, text.find("process").unwrap())
                .unwrap();
            assert_eq!(reference.locations.len(), 1);
            assert_eq!(reference.locations[0].position.line, line + 1);
        }
    }
}

#[test]
#[ignore = "requires installed ruby-lsp; local source only"]
fn ruby_definitions_references_and_updates_use_captured_source_without_running_project_configuration()
 {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let text = "module Audio\n class Other\n  def process(value); -value; end\n end\n class Meter\n  def process(value); value; end\n end\n def self.run(value)\n  Meter.new.process(value)\n  Other.new.process(value)\n end\nend\n";
    let marker = root.path().join("project-was-executed");
    let trap = format!(
        "File.write({:?}, 'executed')\nraise 'project configuration must not run'\n",
        marker.to_str().unwrap()
    );
    std::fs::write(root.path().join("Gemfile"), &trap).unwrap();
    std::fs::create_dir_all(root.path().join("ruby_lsp/local")).unwrap();
    std::fs::write(root.path().join("ruby_lsp/local/addon.rb"), &trap).unwrap();
    std::fs::write(root.path().join("app.rb"), "raise 'live bytes differ'\n").unwrap();
    let server = Servers::new(
        root.path().into(),
        cache.path().into(),
        Configuration {
            ruby_lsp: Some(
                std::env::var_os("VIZZI_RUBY_LSP")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|| "ruby-lsp".into()),
            ),
            ..Default::default()
        },
    )
    .unwrap();
    let mut previous = None;
    for source in [text.to_owned(), format!("\n{text}")] {
        let captured = snapshot(&[
            ("app.rb", "ruby", &source),
            ("Gemfile", "ruby", &trap),
            ("ruby_lsp/local/addon.rb", "ruby", &trap),
        ]);
        let index = Structure::new(&captured);
        let owner = index
            .functions("app.rb", "Audio::run", 0)
            .unwrap()
            .items
            .remove(0)
            .symbol;
        let invocation = index
            .outgoing(&owner.id, "app.rb", 0)
            .unwrap()
            .items
            .into_iter()
            .find(|call| call.name.as_deref() == Some("process"))
            .unwrap();
        let answer = server
            .resolve(captured.clone(), &index.call(&invocation.id).unwrap())
            .unwrap();
        assert_eq!(answer.locations.len(), 1, "outside: {}", answer.outside);
        assert_eq!(answer.outside, 0);
        let position =
            vizzi::definitions::byte_offset(&source, answer.locations[0].position).unwrap();
        let target=index.definition("app.rb",position).unwrap_or_else(||panic!("Ruby definition did not match captured declaration: {:?}, byte {position}, source {source:?}",answer.locations));
        assert!(target.parents.iter().any(|name| name == "Meter"));
        assert!(!target.parents.iter().any(|name| name == "Other"));
        assert_eq!(target.name, "process");
        for site in index.outgoing(&owner.id, "app.rb", 0).unwrap().items {
            let _ = server.resolve(captured.clone(), &index.call(&site.id).unwrap());
        }
        let references = server
            .references(
                captured.clone(),
                &target,
                index.identifier(&target.id).unwrap(),
            )
            .unwrap();
        assert_eq!(references.locations.len(), 1);
        if let Some((line, context)) = previous {
            assert_eq!(answer.locations[0].position.line, line + 1);
            assert_ne!(answer.context, context);
        }
        previous = Some((answer.locations[0].position.line, answer.context));
        assert!(!marker.exists());
    }
    assert!(!root.path().join(".ruby-lsp").exists());
}

#[test]
#[ignore = "requires installed ruby-lsp and local Ruby standard library source"]
fn ruby_local_standard_library_and_installed_gem_definitions_use_the_dependency_catalog() {
    for (library, expression, method, lock) in [
        ("core", "'example'.upcase", "upcase", None),
        (
            "pathname",
            "Pathname.new('example').cleanpath",
            "cleanpath",
            None,
        ),
        (
            "logger",
            "Logger.new($stdout).add(1, 'example')",
            "add",
            Some(
                "GEM\n  specs:\n    logger (1.7.0)\n\nPLATFORMS\n  ruby\n\nDEPENDENCIES\n  logger\n",
            ),
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let require = if library == "core" {
            String::new()
        } else {
            format!("require '{library}'\n")
        };
        let text = format!("{require}module Audio\n def self.run\n  {expression}\n end\nend\n");
        if let Some(lock) = lock {
            std::fs::write(root.path().join("Gemfile.lock"), lock).unwrap();
        }
        let captured = snapshot(&[("app.rb", "ruby", text.as_str())]);
        let index = Structure::new(&captured);
        let server = Servers::new(
            root.path().into(),
            cache.path().into(),
            Configuration {
                ruby_lsp: Some(
                    std::env::var_os("VIZZI_RUBY_LSP")
                        .map(std::path::PathBuf::from)
                        .unwrap_or_else(|| "ruby-lsp".into()),
                ),
                ..Default::default()
            },
        )
        .unwrap();
        let owner = index
            .functions("app.rb", "run", 0)
            .unwrap()
            .items
            .remove(0)
            .symbol;
        let invocation = index
            .outgoing(&owner.id, "app.rb", 0)
            .unwrap()
            .items
            .into_iter()
            .find(|c| c.name.as_deref() == Some(method))
            .unwrap();
        let answer = server
            .resolve(captured.clone(), &index.call(&invocation.id).unwrap())
            .unwrap();
        if library == "core" {
            assert!(answer.locations.is_empty());
            assert!(answer.outside > 0);
            continue;
        }
        assert!(!answer.locations.is_empty(), "{library}");
        assert_eq!(answer.outside, 0);
        let mut gem_source = false;
        for location in &answer.locations {
            let capture = server
                .dependencies()
                .unwrap()
                .get(location.capture.as_ref().unwrap())
                .unwrap()
                .unwrap();
            assert_eq!(capture.input.language, "ruby");
            gem_source |= capture.input.package.starts_with("logger-");
            let body = capture
                .doc()
                .captured()
                .read_text(2 * 1024 * 1024)
                .unwrap()
                .unwrap();
            let byte = vizzi::definitions::byte_offset(&body, location.position).unwrap();
            assert_eq!(
                capture
                    .index
                    .definition(capture.doc().path(), byte)
                    .expect("Ruby source declaration at server location")
                    .name,
                method
            );
        }
        if library == "logger" {
            assert!(gem_source, "installed gem capture");
        } else {
            assert_eq!(answer.locations.len(), 1);
        }
    }
}

#[test]
#[ignore = "requires installed pyright-langserver; local source only"]
fn python_fields_aliases_and_references_resolve_against_captured_source() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let app = "from worker import Meter as Selected\nclass Other:\n    def process(self, value): return -1\nclass Service:\n    def __init__(self): self.meter = Selected()\n    def run(self):\n        self.meter.process(1)\n        callback = self.meter.process\n";
    let worker = "class Meter:\n    def process(self, value): return value\n";
    std::fs::write(root.path().join("app.py"), app).unwrap();
    std::fs::write(root.path().join("worker.py"), worker).unwrap();
    let captured = snapshot(&[("app.py", "python", app), ("worker.py", "python", worker)]);
    std::fs::write(root.path().join("worker.py"), "class Wrong: pass\n").unwrap();
    let service = Servers::new(
        root.path().into(),
        cache.path().into(),
        Configuration {
            pyright: Some("pyright-langserver".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let index = Structure::new(&captured);
    let owner = index
        .functions("app.py", "Service::run", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    let call = index
        .outgoing(&owner.id, "app.py", 0)
        .unwrap()
        .items
        .into_iter()
        .find(|call| call.name.as_deref() == Some("process"))
        .unwrap();
    let answer = service
        .resolve(captured.clone(), &index.call(&call.id).unwrap())
        .unwrap();
    assert_eq!(answer.outside, 0);
    assert_eq!(answer.locations.len(), 1);
    assert_eq!(answer.locations[0].path, "worker.py");
    let target = index
        .functions("worker.py", "Meter::process", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    let refs = service
        .references(
            captured.clone(),
            &target,
            index.identifier(&target.id).unwrap(),
        )
        .unwrap();
    assert_eq!(refs.locations.len(), 2);
    assert_eq!(
        refs.context, answer.context,
        "opening another captured document keeps semantic inputs stable"
    );
    let mut invalid = index.call(&call.id).unwrap();
    invalid.invocation.source.version = "f".repeat(64);
    assert!(service.resolve(captured.clone(), &invalid).is_err());
    let mut invalid = index.call(&call.id).unwrap();
    invalid.identifier = Some(0..4);
    assert!(service.resolve(captured.clone(), &invalid).is_err());
    std::fs::write(
        root.path().join("pyrightconfig.json"),
        "{\"typeCheckingMode\":\"basic\"}",
    )
    .unwrap();
    let configured = service
        .resolve(captured.clone(), &index.call(&call.id).unwrap())
        .unwrap();
    assert_ne!(configured.context, answer.context);
    assert_eq!(configured.locations[0].path, "worker.py");
    let shifted = format!("\n{worker}");
    let updated = snapshot(&[("app.py", "python", app), ("worker.py", "python", &shifted)]);
    let updated_index = Structure::new(&updated);
    let owner = updated_index
        .functions("app.py", "Service::run", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    let invocation = updated_index
        .outgoing(&owner.id, "app.py", 0)
        .unwrap()
        .items
        .into_iter()
        .find(|call| call.name.as_deref() == Some("process"))
        .unwrap();
    let moved = service
        .resolve(
            updated.clone(),
            &updated_index.call(&invocation.id).unwrap(),
        )
        .unwrap();
    assert_eq!(moved.locations[0].position.line, 2);
    assert_ne!(moved.context, configured.context);
    let removed = snapshot(&[("app.py", "python", app)]);
    let removed_index = Structure::new(&removed);
    let removed_owner = removed_index
        .functions("app.py", "Service::run", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    let removed_invocation = removed_index
        .outgoing(&removed_owner.id, "app.py", 0)
        .unwrap()
        .items
        .into_iter()
        .find(|call| call.name.as_deref() == Some("process"))
        .unwrap();
    let removed_call = removed_index.call(&removed_invocation.id).unwrap();
    let missing = service.resolve(removed, &removed_call).unwrap();
    assert!(missing.locations.is_empty());
    assert_eq!(
        missing.outside, 0,
        "a removed module must leave the server's import graph"
    );
    assert!(
        refs.locations
            .iter()
            .all(|location| location.path == "app.py")
    );
    assert_eq!(
        std::fs::read_to_string(root.path().join("worker.py")).unwrap(),
        "class Wrong: pass\n"
    );
}

#[test]
#[ignore = "requires installed pyright-langserver; local source only"]
fn python_configuration_switches_import_roots() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let app = "from worker import Meter\ndef run(): return Meter().process()\n";
    let worker = "class Meter:\n    def process(self): return 1\n";
    let captured = snapshot(&[
        ("app.py", "python", app),
        ("first/worker.py", "python", worker),
        ("second/worker.py", "python", worker),
    ]);
    let service = Servers::new(
        root.path().into(),
        cache.path().into(),
        Configuration {
            pyright: Some("pyright-langserver".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let index = Structure::new(&captured);
    let owner = index
        .functions("app.py", "run", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    let invocation = index
        .outgoing(&owner.id, "app.py", 0)
        .unwrap()
        .items
        .into_iter()
        .find(|call| call.name.as_deref() == Some("process"))
        .unwrap();
    for directory in ["first", "second"] {
        std::fs::write(
            root.path().join("pyrightconfig.json"),
            serde_json::to_vec(&serde_json::json!({"extraPaths":[directory]})).unwrap(),
        )
        .unwrap();
        let answer = service
            .resolve(captured.clone(), &index.call(&invocation.id).unwrap())
            .unwrap();
        assert_eq!(answer.locations.len(), 1);
        assert_eq!(answer.locations[0].path, format!("{directory}/worker.py"));
    }
}

#[test]
#[ignore = "requires installed clangd; local source only"]
fn clangd_resolves_cpp_receiver_types_and_c_functions_with_compilation_context() {
    for (language, filename, text, owner_name, expected) in [
        (
            "cpp",
            "app.cpp",
            "struct Other { int process(int x){return -x;} };\nstruct Meter { int process(int x){return x;} };\nint run(Meter& meter){ return meter.process(2); }\n",
            "run",
            "Meter",
        ),
        (
            "c",
            "app.c",
            "int target(int x){return x;}\nint run(){return target(2);}\n",
            "run",
            "",
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join(filename), text).unwrap();
        let captured = snapshot(&[(filename, language, text)]);
        std::fs::write(root.path().join("compile_commands.json"),serde_json::to_vec(&serde_json::json!([{"directory":root.path(),"file":root.path().join(filename),"arguments":[if language=="c"{"clang"}else{"clang++"},"-c",root.path().join(filename)]}])).unwrap()).unwrap();
        std::fs::write(
            root.path().join(filename),
            "// live bytes differ from capture\n",
        )
        .unwrap();
        let executable = std::env::var_os("VIZZI_CLANGD")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| "clangd".into());
        let service = Servers::new(
            root.path().into(),
            cache.path().into(),
            Configuration {
                clangd: Some(executable),
                ..Default::default()
            },
        )
        .unwrap();
        let index = Structure::new(&captured);
        let owner = index
            .functions(filename, owner_name, 0)
            .unwrap()
            .items
            .remove(0)
            .symbol;
        let invocation = index
            .outgoing(&owner.id, filename, 0)
            .unwrap()
            .items
            .remove(0);
        let answer = service
            .resolve(captured.clone(), &index.call(&invocation.id).unwrap())
            .unwrap();
        assert_eq!(
            answer.locations.len(),
            1,
            "{language}: outside {}",
            answer.outside
        );
        assert_eq!(answer.locations[0].path, filename);
        let text_doc = captured.file_by_path(filename).unwrap();
        let byte = vizzi::definitions::byte_offset(text, answer.locations[0].position).unwrap();
        let target = index.definition(filename, byte).unwrap();
        assert!(target.parents.iter().any(|parent| parent == expected) || expected.is_empty());
        let refs = service
            .references(
                captured.clone(),
                &target,
                index.identifier(&target.id).unwrap(),
            )
            .unwrap();
        assert_eq!(refs.locations.len(), 1, "{language}");
        assert!(text_doc.resident_source().unwrap().contains("target") || language == "cpp");
    }
}

#[test]
#[ignore = "requires installed clangd; local source only"]
fn clangd_header_declarations_and_out_of_class_definitions_are_admitted() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let header = "struct Other { int process(int); };\nstruct Meter { int process(int); };\n";
    let app = "#include \"worker.h\"\nint run(Meter& meter){return meter.process(2);}\n";
    let implementation = "#include \"worker.h\"\nint Meter::process(int value){return value;}\nint Other::process(int value){return -value;}\n";
    let files = [
        ("worker.h", "cpp", header),
        ("app.cpp", "cpp", app),
        ("worker.cpp", "cpp", implementation),
    ];
    for (path, _, text) in &files {
        std::fs::write(root.path().join(path), text).unwrap();
    }
    let database = serde_json::json!([{"directory":root.path(),"file":"app.cpp","arguments":["clang++","-I",root.path(),"-c","app.cpp"]},{"directory":root.path(),"file":"worker.cpp","arguments":["clang++","-I",root.path(),"-c","worker.cpp"]}]);
    std::fs::write(
        root.path().join("compile_commands.json"),
        serde_json::to_vec(&database).unwrap(),
    )
    .unwrap();
    let captured = snapshot(&files);
    let index = Structure::new(&captured);
    let service = Servers::new(
        root.path().into(),
        cache.path().into(),
        Configuration {
            clangd: Some(
                std::env::var_os("VIZZI_CLANGD")
                    .map(std::path::PathBuf::from)
                    .unwrap_or_else(|| "clangd".into()),
            ),
            ..Default::default()
        },
    )
    .unwrap();
    let owner = index
        .functions("app.cpp", "run", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    let invocation = index
        .outgoing(&owner.id, "app.cpp", 0)
        .unwrap()
        .items
        .remove(0);
    let answer = service
        .resolve(captured.clone(), &index.call(&invocation.id).unwrap())
        .unwrap();
    assert!(!answer.locations.is_empty());
    assert_eq!(answer.outside, 0);
    for location in answer.locations {
        let doc = captured.file_by_path(&location.path).unwrap();
        let text = doc.resident_source().unwrap();
        let target = index
            .definition(
                &location.path,
                vizzi::definitions::byte_offset(text, location.position).unwrap(),
            )
            .unwrap();
        assert!(
            target.parents.iter().any(|parent| parent == "Meter")
                || target.name == "Meter::process"
        );
    }
}

#[test]
#[ignore = "requires installed pyright-langserver; local source only"]
fn python_aliases_follow_local_dependency_files_without_adding_them_to_the_snapshot() {
    let root = tempfile::tempdir().unwrap();
    let dependency = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let app = "from worker import Meter as Selected\ndef run():\n    meter=Selected()\n    return meter.process(1)\n";
    std::fs::write(root.path().join("app.py"), app).unwrap();
    std::fs::write(
        root.path().join("pyrightconfig.json"),
        serde_json::to_vec(&serde_json::json!({"extraPaths":[dependency.path()]})).unwrap(),
    )
    .unwrap();
    std::fs::write(dependency.path().join("worker.py"),"from helpers import leaf\nclass Meter:\n    def process(self, value): return leaf(value)\n").unwrap();
    std::fs::write(
        dependency.path().join("helpers.py"),
        "def leaf(value): return value\n",
    )
    .unwrap();
    let captured = snapshot(&[("app.py", "python", app)]);
    let index = Structure::new(&captured);
    let owner = index
        .functions("app.py", "run", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    let call = index
        .outgoing(&owner.id, "app.py", 0)
        .unwrap()
        .items
        .into_iter()
        .find(|call| call.name.as_deref() == Some("process"))
        .unwrap();
    let service = Servers::new(
        root.path().into(),
        cache.path().into(),
        Configuration {
            pyright: Some("pyright-langserver".into()),
            ..Default::default()
        },
    )
    .unwrap();
    let answer = service
        .resolve(captured.clone(), &index.call(&call.id).unwrap())
        .unwrap();
    assert_eq!(answer.locations.len(), 1);
    let capture = service
        .dependencies()
        .unwrap()
        .get(answer.locations[0].capture.as_deref().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(capture.doc().path(), "worker.py");
    assert_eq!(capture.origin.language, "python");
    assert!(captured.file_by_path("worker.py").is_none());
    let target = capture
        .index
        .functions("", "Meter::process", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    let nested = capture
        .index
        .outgoing(&target.id, "worker.py", 0)
        .unwrap()
        .items
        .remove(0);
    let nested = service
        .resolve(captured.clone(), &capture.index.call(&nested.id).unwrap())
        .unwrap();
    assert_eq!(nested.locations.len(), 1);
    let leaf = service
        .dependencies()
        .unwrap()
        .get(nested.locations[0].capture.as_deref().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(leaf.doc().path(), "helpers.py");
    assert_ne!(leaf.origin.capture, capture.origin.capture);
}

#[test]
#[ignore = "requires installed pyright-langserver and Python venv; local source only"]
fn python_interpreter_selects_an_existing_environment_without_installing_packages() {
    let root = tempfile::tempdir().unwrap();
    let environment = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let output = std::process::Command::new("python3")
        .args(["-m", "venv", "--without-pip"])
        .arg(environment.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let library = std::fs::read_dir(environment.path().join("lib"))
        .unwrap()
        .map(|entry| entry.unwrap().path().join("site-packages"))
        .find(|path| path.is_dir())
        .unwrap();
    std::fs::write(
        library.join("captured_demo.py"),
        "class Meter:\n    def process(self, value): return value\n",
    )
    .unwrap();
    let verify = std::process::Command::new(environment.path().join("bin/python"))
        .args([
            "-I",
            "-c",
            "from captured_demo import Meter; assert Meter().process(1)==1",
        ])
        .output()
        .unwrap();
    assert!(
        verify.status.success(),
        "{}",
        String::from_utf8_lossy(&verify.stderr)
    );
    let app = "from captured_demo import Meter\ndef run():\n    meter=Meter()\n    return meter.process(1)\n";
    std::fs::write(root.path().join("app.py"), app).unwrap();
    let captured = snapshot(&[("app.py", "python", app)]);
    let index = Structure::new(&captured);
    let owner = index
        .functions("app.py", "run", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    let invocation = index
        .outgoing(&owner.id, "app.py", 0)
        .unwrap()
        .items
        .into_iter()
        .find(|call| call.name.as_deref() == Some("process"))
        .unwrap();
    let server = Servers::new(
        root.path().into(),
        cache.path().into(),
        Configuration {
            pyright: Some("pyright-langserver".into()),
            python_path: Some(environment.path().join("bin/python")),
            ..Default::default()
        },
    )
    .unwrap();
    let answer = server
        .resolve(captured.clone(), &index.call(&invocation.id).unwrap())
        .unwrap();
    assert_eq!(answer.locations.len(), 1);
    let source = server
        .dependencies()
        .unwrap()
        .get(answer.locations[0].capture.as_deref().unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(source.doc().path(), "captured_demo.py");
    assert_eq!(source.origin.label, "captured_demo");
}
