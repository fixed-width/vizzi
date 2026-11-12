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
fn ruby_full_capture_updates_follow_the_document_lifecycle_protocol() {
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
        uri=params['textDocument']['uri'];text=opened[uri];offset=text.index('process');prefix=text[:offset]
        position={'line':prefix.count('\n'),'character':len(prefix.rsplit('\n',1)[-1])}
        send({'id':request['id'],'result':[{'uri':uri,'range':{'start':position,'end':position}}]})
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
    for (line, text) in [
        (
            0,
            "def process(value); value; end\ndef run; process(1); end\n",
        ),
        (
            1,
            "\ndef process(value); value; end\ndef run; process(1); end\n",
        ),
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
        assert_eq!(result.locations.len(), 1);
        assert_eq!(result.locations[0].position.line, line);
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
