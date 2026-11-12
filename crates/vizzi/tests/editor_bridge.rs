//! Editor-owned providers are exercised through the native language-server interface.
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    path::PathBuf,
    sync::Arc,
    thread,
    time::{Duration, Instant},
};
use vizzi::{
    cache::Cache,
    definitions::Provider,
    engine::Engine,
    language_servers::{Configuration, Servers},
    source::Directory,
};
use vizzi_core::{
    model::Snapshot,
    structure::{CallLookup, Structure},
};

const TEXT: &str = "# 😀\ndef leaf(): pass\ndef run(): leaf()\n";
struct Fixture {
    directory: tempfile::TempDir,
    root: PathBuf,
    connection: PathBuf,
    snapshot: Arc<Snapshot>,
}
impl Fixture {
    fn new() -> Self {
        Self::with_text(TEXT)
    }
    fn with_text(text: &str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap().join("project");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("a.py"), text).unwrap();
        let snapshot = Engine::new(
            Directory::open(&root).unwrap(),
            Cache::new(directory.path().join("cache")),
            1,
        )
        .unwrap()
        .refresh(None, &|| false)
        .unwrap()
        .unwrap()
        .snapshot;
        let connection = directory.path().join("connection.json");
        let fixture = Self {
            directory,
            root,
            connection,
            snapshot,
        };
        fixture.manifest(9);
        fixture
    }
    fn manifest(&self, port: u16) {
        fs::write(
            &self.connection,
            serde_json::to_vec(
                &json!({"version":1,"root":self.root,"port":port,"token":"a".repeat(64)}),
            )
            .unwrap(),
        )
        .unwrap();
    }
    fn provider(&self) -> Servers {
        Servers::new(
            self.root.clone(),
            self.directory.path().join("semantic"),
            Configuration {
                vscode_connection: Some(self.connection.clone()),
                ..Default::default()
            },
        )
        .unwrap()
    }
    fn call(&self) -> CallLookup {
        let index = Structure::new(&self.snapshot);
        let run = index
            .functions("a.py", "run", 0)
            .unwrap()
            .items
            .remove(0)
            .symbol;
        let site = index.outgoing(&run.id, "a.py", 0).unwrap().items.remove(0);
        index.call(&site.id).unwrap()
    }
    fn answer(&self) -> Value {
        json!({"version":1,"status":"ok","session":"session-one","epoch":3,"outside":0,
            "locations":[{"path":self.root.join("a.py"),"position":{"line":1,"character":4},"text":self.snapshot.file_by_path("a.py").unwrap().resident_source().unwrap()}]})
    }
}
struct Peer(thread::JoinHandle<Value>);
impl Peer {
    fn start(fixture: &Fixture, response: impl FnOnce(Value) -> Vec<u8> + Send + 'static) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        fixture.manifest(listener.local_addr().unwrap().port());
        listener.set_nonblocking(true).unwrap();
        Self(thread::spawn(move || {
            let start = Instant::now();
            let (mut socket, _) = loop {
                match listener.accept() {
                    Ok(value) => break value,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(
                            start.elapsed() < Duration::from_secs(3),
                            "native peer never connected"
                        );
                        thread::sleep(Duration::from_millis(1));
                    }
                    Err(error) => panic!("{error}"),
                }
            };
            socket
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut line = String::new();
            BufReader::new(socket.try_clone().unwrap())
                .read_line(&mut line)
                .unwrap();
            let request: Value = if line.is_empty() {
                Value::Null
            } else {
                serde_json::from_str(&line).unwrap()
            };
            let bytes = response(request.clone());
            socket.write_all(&bytes).unwrap();
            request
        }))
    }
    fn finish(self) -> Value {
        self.0.join().unwrap()
    }
}
fn frame(response: Value) -> Vec<u8> {
    format!("{response}\n").into_bytes()
}

#[test]
fn editor_definitions_and_references_reuse_the_connection_with_captured_positions() {
    let fixture = Fixture::new();
    let provider = fixture.provider();
    let response = fixture.answer();
    let peer = Peer::start(&fixture, move |request| {
        assert_eq!(request["method"], "definition");
        frame(response)
    });
    let answer = provider
        .resolve(fixture.snapshot.clone(), &fixture.call())
        .unwrap();
    assert_eq!(answer.locations.len(), 1);
    assert_eq!(answer.locations[0].position.line, 1);
    assert_eq!(answer.context.len(), 64);
    assert!(answer.context.bytes().all(|byte| byte.is_ascii_hexdigit()));
    let previous_context = answer.context;
    let request = peer.finish();
    assert_eq!(request["text"], TEXT);
    assert_eq!(request["position"], json!({"line":2,"character":11}));
    assert_eq!(request["token"], "a".repeat(64));
    assert_eq!(request["root"], fixture.root.to_str().unwrap());
    let index = Structure::new(&fixture.snapshot);
    let symbol = index
        .functions("a.py", "leaf", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    let mut response = fixture.answer();
    response["session"] = json!("session-two");
    response["locations"][0]["position"] = json!({"line":2,"character":11});
    let peer = Peer::start(&fixture, move |request| {
        assert_eq!(request["method"], "references");
        frame(response)
    });
    let answer = provider
        .references(
            fixture.snapshot.clone(),
            &symbol,
            TEXT.find("leaf").unwrap(),
        )
        .unwrap();
    assert_eq!(answer.locations[0].position.line, 2);
    assert_ne!(answer.context, previous_context);
    assert_eq!(peer.finish()["position"], json!({"line":1,"character":4}));
    assert!(!fixture.directory.path().join("semantic").exists());
}

#[test]
fn live_source_changes_and_other_workspace_inputs_invalidate_editor_analysis() {
    let fixture = Fixture::new();
    fs::write(fixture.root.join("a.py"), "def different(): pass\n").unwrap();
    let error = fixture
        .provider()
        .resolve(fixture.snapshot.clone(), &fixture.call())
        .unwrap_err();
    assert!(error.contains("match saved files"), "{error}");
}

#[test]
fn edits_during_lookup_and_mismatched_target_text_are_rejected() {
    for edit_disk in [false, true] {
        let fixture = Fixture::new();
        let mut response = fixture.answer();
        let path = fixture.root.join("a.py");
        if !edit_disk {
            response["locations"][0]["text"] = json!("different");
        }
        let peer = Peer::start(&fixture, move |_| {
            if edit_disk {
                fs::write(path, "def changed(): pass").unwrap();
            }
            frame(response)
        });
        let error = fixture
            .provider()
            .resolve(fixture.snapshot.clone(), &fixture.call())
            .unwrap_err();
        assert!(
            error.contains(if edit_disk {
                "match saved files"
            } else {
                "target differs"
            }),
            "{error}"
        );
        peer.finish();
    }
}

#[test]
fn invalid_manifests_fail_before_connecting_and_missing_bridges_never_spawn_servers() {
    let fixture = Fixture::new();
    let original: Value = serde_json::from_slice(&fs::read(&fixture.connection).unwrap()).unwrap();
    for (key, value) in [
        ("version", json!(2)),
        ("port", json!(0)),
        ("token", json!("bad")),
        ("root", json!(fixture.directory.path())),
        ("token", json!("z".repeat(64))),
    ] {
        let mut manifest = original.clone();
        manifest[key] = value;
        fs::write(&fixture.connection, manifest.to_string()).unwrap();
        let error = fixture
            .provider()
            .resolve(fixture.snapshot.clone(), &fixture.call())
            .unwrap_err();
        assert!(
            error.contains(if key == "root" {
                "another workspace root"
            } else {
                "Unsupported or invalid"
            }),
            "{error}"
        );
    }
    fs::remove_file(&fixture.connection).unwrap();
    let error = fixture
        .provider()
        .resolve(fixture.snapshot.clone(), &fixture.call())
        .unwrap_err();
    assert!(error.contains("Cannot read"), "{error}");
    fixture.manifest(9);
    assert!(
        fixture
            .provider()
            .resolve(fixture.snapshot.clone(), &fixture.call())
            .is_err()
    );
    assert!(!fixture.directory.path().join("semantic").exists());
}

#[test]
fn mismatched_reference_declarations_are_rejected_before_querying_the_editor() {
    let fixture = Fixture::new();
    let index = Structure::new(&fixture.snapshot);
    let symbol = index
        .functions("a.py", "leaf", 0)
        .unwrap()
        .items
        .remove(0)
        .symbol;
    let error = fixture
        .provider()
        .references(fixture.snapshot.clone(), &symbol, TEXT.find("run").unwrap())
        .unwrap_err();
    assert!(error.contains("declaration name differs"), "{error}");
    for field in ["kind", "context"] {
        let mut symbol = symbol.clone();
        if field == "kind" {
            symbol.kind = "class".into();
        } else {
            symbol.source.context = "other-context".into();
        }
        let error = fixture
            .provider()
            .references(
                fixture.snapshot.clone(),
                &symbol,
                TEXT.find("leaf").unwrap(),
            )
            .unwrap_err();
        assert!(
            error.contains("declaration identifier is unavailable"),
            "{error}"
        );
    }
}

#[test]
fn malformed_responses_invalid_positions_and_explicit_editor_errors_are_unavailable() {
    let fixture = Fixture::new();
    let original = fixture.answer();
    let mut responses = vec![
        b"invalid\n".to_vec(),
        vec![],
        frame(json!({"version":1,"status":"error","message":"Save buffers"})),
    ];
    for (field, value) in [
        ("version", json!(2)),
        ("session", json!("")),
        ("outside", json!(65537)),
        ("locations", json!([])),
    ] {
        let mut response = original.clone();
        response[field] = value;
        if field == "locations" {
            response["locations"] =
                json!([{"path":"relative.py","position":{"line":0,"character":0},"text":TEXT}]);
        }
        responses.push(frame(response));
    }
    for position in [
        json!({"line":100,"character":0}),
        json!({"line":0,"character":3}),
    ] {
        let mut response = original.clone();
        response["locations"][0]["position"] = position;
        responses.push(frame(response));
    }
    for response in responses {
        let peer = Peer::start(&fixture, move |_| response);
        assert!(
            fixture
                .provider()
                .resolve(fixture.snapshot.clone(), &fixture.call())
                .is_err()
        );
        peer.finish();
    }
}

#[test]
fn uncaptured_targets_stay_outside_instead_of_becoming_local_name_matches() {
    let fixture = Fixture::new();
    let mut response = fixture.answer();
    response["locations"][0]["path"] = json!(fixture.root.join("missing.py"));
    response["locations"].as_array_mut().unwrap().push(json!({"path":fixture.directory.path().join("external.py"),"position":{"line":0,"character":0},"text":"outside"}));
    response["outside"] = json!(2);
    let peer = Peer::start(&fixture, move |_| frame(response));
    let answer = fixture
        .provider()
        .resolve(fixture.snapshot.clone(), &fixture.call())
        .unwrap();
    assert!(answer.locations.is_empty());
    assert_eq!(answer.outside, 4);
    peer.finish();
}

#[test]
fn replacing_the_editor_session_during_lookup_rejects_the_old_reply() {
    let fixture = Fixture::new();
    let response = fixture.answer();
    let connection = fixture.connection.clone();
    let peer = Peer::start(&fixture, move |_| {
        let mut manifest: Value = serde_json::from_slice(&fs::read(&connection).unwrap()).unwrap();
        manifest["token"] = json!("b".repeat(64));
        fs::write(connection, manifest.to_string()).unwrap();
        frame(response)
    });
    let error = fixture
        .provider()
        .resolve(fixture.snapshot.clone(), &fixture.call())
        .unwrap_err();
    assert!(error.contains("connection changed"), "{error}");
    peer.finish();
}

#[test]
fn ordinary_large_source_fits_the_bridge_without_reducing_protocol_allowances() {
    let text = format!("# {}\n{TEXT}", "x".repeat(300 * 1024));
    let fixture = Fixture::with_text(&text);
    let mut response = fixture.answer();
    response["locations"][0]["position"]["line"] = json!(2);
    let peer = Peer::start(&fixture, move |_| frame(response));
    assert_eq!(
        fixture
            .provider()
            .resolve(fixture.snapshot.clone(), &fixture.call())
            .unwrap()
            .locations[0]
            .position
            .line,
        2
    );
    peer.finish();
}

#[test]
fn exact_response_frame_and_result_allowances_are_inclusive() {
    let fixture = Fixture::new();
    let mut response = fixture.answer();
    response["session"] = json!("s".repeat(128));
    response["outside"] = json!(65536);
    response["locations"] = Value::Array(vec![response["locations"][0].clone(); 256]);
    let peer = Peer::start(&fixture, move |_| frame(response));
    let answer = fixture
        .provider()
        .resolve(fixture.snapshot.clone(), &fixture.call())
        .unwrap();
    assert_eq!(answer.locations.len(), 256);
    assert_eq!(answer.outside, 65536);
    peer.finish();
    for length in [16 * 1024 * 1024, 16 * 1024 * 1024 + 1] {
        let mut response = fixture.answer();
        response["padding"] = json!("");
        let padding = length - frame(response.clone()).len();
        response["padding"] = json!("x".repeat(padding));
        let peer = Peer::start(&fixture, move |_| frame(response));
        let result = fixture
            .provider()
            .resolve(fixture.snapshot.clone(), &fixture.call());
        if length == 16 * 1024 * 1024 {
            assert!(result.is_ok(), "{result:?}");
        } else {
            assert!(result.unwrap_err().contains("response exceeds allowance"));
        }
        peer.finish();
    }
    for field in ["session", "locations"] {
        let mut response = fixture.answer();
        response[field] = if field == "session" {
            json!("s".repeat(129))
        } else {
            Value::Array(vec![response["locations"][0].clone(); 257])
        };
        let peer = Peer::start(&fixture, move |_| frame(response));
        assert!(
            fixture
                .provider()
                .resolve(fixture.snapshot.clone(), &fixture.call())
                .unwrap_err()
                .contains("oversized editor result")
        );
        peer.finish();
    }
}

#[test]
fn exact_request_frame_allowance_is_inclusive_and_excess_is_not_written() {
    for excess in [false, true] {
        let mut fixture = Fixture::new();
        let base = "# \ndef leaf(): pass\ndef run(): leaf()\n";
        let overhead=frame(json!({"version":1,"token":"a".repeat(64),"root":fixture.root,"method":"definition","path":"a.py","text":base,"position":{"line":2,"character":11}})).len();
        let encoded = 16 * 1024 * 1024 - overhead + usize::from(excess);
        let text = format!(
            "# {}{}\ndef leaf(): pass\ndef run(): leaf()\n",
            "\u{1}".repeat(encoded / 6),
            "x".repeat(encoded % 6)
        );
        fs::write(fixture.root.join("a.py"), &text).unwrap();
        fixture.snapshot = Engine::new(
            Directory::open(&fixture.root).unwrap(),
            Cache::new(fixture.directory.path().join("recapture")),
            1,
        )
        .unwrap()
        .refresh(None, &|| false)
        .unwrap()
        .unwrap()
        .snapshot;
        let peer = Peer::start(&fixture, move |request| {
            if excess {
                assert_eq!(request, Value::Null);
                vec![]
            } else {
                frame(
                    json!({"version":1,"status":"ok","session":"request-limit","epoch":0,"outside":0,"locations":[]}),
                )
            }
        });
        let result = fixture
            .provider()
            .resolve(fixture.snapshot.clone(), &fixture.call());
        peer.finish();
        if excess {
            assert!(result.unwrap_err().contains("request exceeds allowance"));
        } else {
            assert!(result.is_ok(), "{result:?}");
        }
    }
}

fn metadata(
    fixture: &Fixture,
    path: String,
    source: Arc<vizzi_core::model::CapturedSource>,
) -> Arc<vizzi_core::model::Document> {
    use vizzi_core::policy::{Classification, Provenance, SourceOnlyReason};
    Arc::new(
        vizzi_core::model::Document::capture_source(
            fixture.snapshot.workspace().clone(),
            path,
            source,
            vec![],
            Classification::SourceOnly {
                language: None,
                provenance: Provenance::Extension {
                    extension: "toml".into(),
                },
                reason: SourceOnlyReason::ProjectMetadata,
            },
        )
        .unwrap(),
    )
}

#[test]
fn exact_file_and_workspace_byte_allowances_are_accepted_and_excess_bytes_are_rejected() {
    use vizzi_core::model::CapturedSource;
    let mut fixture = Fixture::new();
    let bytes = vec![b' '; 4 * 1024 * 1024];
    let source = CapturedSource::capture(bytes.clone(), None);
    let mut docs = fixture.snapshot.documents().to_vec();
    for index in 0..32 {
        let path = format!("metadata-{index:02}.toml");
        let target = fixture.root.join(&path);
        let captured = if index == 31 {
            let tail = &bytes[..bytes.len() - TEXT.len()];
            fs::write(&target, tail).unwrap();
            CapturedSource::capture(tail.to_vec(), None)
        } else {
            if index == 0 {
                fs::write(&target, &bytes).unwrap();
            } else {
                fs::hard_link(fixture.root.join("metadata-00.toml"), &target).unwrap();
            }
            source.clone()
        };
        docs.push(metadata(&fixture, path, captured));
    }
    fixture.snapshot = Arc::new(
        Snapshot::new(
            fixture.snapshot.workspace().clone(),
            docs.clone(),
            fixture.snapshot.policy().clone(),
        )
        .unwrap(),
    );
    let response = fixture.answer();
    let peer = Peer::start(&fixture, move |_| frame(response));
    assert!(
        fixture
            .provider()
            .resolve(fixture.snapshot.clone(), &fixture.call())
            .is_ok()
    );
    peer.finish();
    let extra = "extra.toml";
    fs::write(fixture.root.join(extra), b" ").unwrap();
    docs.push(metadata(
        &fixture,
        extra.into(),
        CapturedSource::capture(vec![b' '], None),
    ));
    let excessive = Arc::new(
        Snapshot::new(
            fixture.snapshot.workspace().clone(),
            docs,
            fixture.snapshot.policy().clone(),
        )
        .unwrap(),
    );
    assert!(
        fixture
            .provider()
            .resolve(excessive, &fixture.call())
            .unwrap_err()
            .contains("workspace exceeds byte allowance")
    );
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(fixture.root.join("metadata-00.toml"))
        .unwrap();
    file.write_all(b" ").unwrap();
    assert!(
        fixture
            .provider()
            .resolve(fixture.snapshot.clone(), &fixture.call())
            .unwrap_err()
            .contains("source exceeds allowance")
    );
}

#[test]
fn exact_workspace_file_allowance_is_accepted_and_an_extra_file_is_rejected() {
    use vizzi_core::model::CapturedSource;
    let mut fixture = Fixture::new();
    let mut docs = fixture.snapshot.documents().to_vec();
    let source = CapturedSource::capture(vec![], None);
    for index in 0..8191 {
        let path = format!("empty-{index:04}.toml");
        if index == 0 {
            fs::write(fixture.root.join(&path), b"").unwrap();
        } else {
            fs::hard_link(
                fixture.root.join("empty-0000.toml"),
                fixture.root.join(&path),
            )
            .unwrap();
        }
        docs.push(metadata(&fixture, path, source.clone()));
    }
    fixture.snapshot = Arc::new(
        Snapshot::new(
            fixture.snapshot.workspace().clone(),
            docs.clone(),
            fixture.snapshot.policy().clone(),
        )
        .unwrap(),
    );
    let response = fixture.answer();
    let peer = Peer::start(&fixture, move |_| frame(response));
    assert!(
        fixture
            .provider()
            .resolve(fixture.snapshot.clone(), &fixture.call())
            .is_ok()
    );
    peer.finish();
    docs.push(metadata(&fixture, "extra.toml".into(), source));
    let excessive = Arc::new(
        Snapshot::new(
            fixture.snapshot.workspace().clone(),
            docs,
            fixture.snapshot.policy().clone(),
        )
        .unwrap(),
    );
    assert!(
        fixture
            .provider()
            .resolve(excessive, &fixture.call())
            .unwrap_err()
            .contains("workspace exceeds file allowance")
    );
}

#[test]
fn cpp_references_query_the_method_identifier_in_qualified_definitions() {
    use vizzi_core::{
        analysis::Languages,
        model::Document,
        policy::{Classification, Provenance},
    };
    for (text, name, character) in [
        (
            "struct Meter { int leaf(); };\nint Meter::leaf(){ return 1; }\n",
            "Meter::leaf",
            11,
        ),
        (
            "struct Meter { operator bool() const; };\nMeter::operator bool() const { return true; }\n",
            "Meter::operator bool",
            7,
        ),
        (
            "namespace std { struct string {}; }\nstruct Meter { operator std::string() const { return {}; } };\n",
            "operator std::string",
            15,
        ),
    ] {
        let mut fixture = Fixture::new();
        fs::remove_file(fixture.root.join("a.py")).unwrap();
        fs::write(fixture.root.join("a.cpp"), text).unwrap();
        let languages = Languages::builtin().unwrap();
        let mut parser = languages.select("cpp").unwrap().create().unwrap();
        let doc = Arc::new(
            Document::capture(
                fixture.snapshot.workspace().clone(),
                "a.cpp".into(),
                text.as_bytes(),
                vec![parser.analyze(text.as_bytes()).unwrap()],
                Classification::Assigned {
                    language: "cpp".into(),
                    provenance: Provenance::ExplicitScope { scope: ".".into() },
                },
            )
            .unwrap(),
        );
        fixture.snapshot = Arc::new(
            Snapshot::new(
                fixture.snapshot.workspace().clone(),
                vec![doc],
                fixture.snapshot.policy().clone(),
            )
            .unwrap(),
        );
        let index = Structure::new(&fixture.snapshot);
        let symbol = index
            .functions("a.cpp", "", 0)
            .unwrap()
            .items
            .into_iter()
            .find(|entry| entry.symbol.source.line == 2)
            .unwrap()
            .symbol;
        let peer = Peer::start(&fixture, move |request| {
            assert_eq!(request["method"], "references");
            assert_eq!(request["position"], json!({"line":1,"character":character}));
            frame(
                json!({"version":1,"status":"ok","session":"cpp-editor","epoch":0,"outside":0,"locations":[]}),
            )
        });
        let result = fixture.provider().references(
            fixture.snapshot.clone(),
            &symbol,
            text.find(name).unwrap(),
        );
        let request = peer.finish();
        assert!(result.is_ok(), "{result:?}; request: {request}");
    }
}

#[cfg(unix)]
#[test]
fn editor_and_native_root_spellings_can_share_one_physical_workspace() {
    let fixture = Fixture::new();
    let alias = fixture.directory.path().join("alias");
    std::os::unix::fs::symlink(&fixture.root, &alias).unwrap();
    let mut response = fixture.answer();
    response["locations"][0]["path"] = json!(alias.join("a.py"));
    let expected = alias.clone();
    let peer = Peer::start(&fixture, move |request| {
        assert_eq!(request["root"], json!(expected));
        frame(response)
    });
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(&fixture.connection).unwrap()).unwrap();
    manifest["root"] = json!(alias);
    fs::write(&fixture.connection, manifest.to_string()).unwrap();
    let result = fixture
        .provider()
        .resolve(fixture.snapshot.clone(), &fixture.call());
    assert!(result.is_ok(), "{result:?}");
    assert_eq!(result.unwrap().locations[0].path, "a.py");
    peer.finish();
}
