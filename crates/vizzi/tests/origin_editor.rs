//! Originating-editor delivery through the existing verified-location seam.
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    path::Path,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use vizzi::editor::{ArchiveEditorLocation, Editor, EditorLocation, OriginEditor, SourceWitness};

fn descriptor(root: &Path, port: u16, session: &str, token: &str) -> Value {
    json!({"protocol":"vizzi-origin","version":1,"root":root,"editor":"Fixture editor","session":session,"port":port,"token":token})
}
fn write_descriptor(file: &Path, value: &Value) {
    std::fs::write(file, value.to_string()).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
}
fn peer(root: &Path) -> (Value, Arc<Mutex<Vec<Value>>>, thread::JoinHandle<()>) {
    peer_reply(root, |request| {
        format!("{}\n", json!({"protocol":"vizzi-origin","version":1,"session":request["session"],"request":request["request"],"status":"opened"})).into_bytes()
    })
}
fn peer_reply(
    root: &Path,
    reply: impl FnOnce(&Value) -> Vec<u8> + Send + 'static,
) -> (Value, Arc<Mutex<Vec<Value>>>, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let received = Arc::new(Mutex::new(Vec::new()));
    let seen = received.clone();
    let handle = thread::spawn(move || {
        listener.set_nonblocking(true).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(2);
        let (mut stream, _) = loop {
            match listener.accept() {
                Ok(accepted) => break accepted,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "No originating-editor request arrived"
                    );
                    thread::sleep(Duration::from_millis(5));
                }
                Err(error) => panic!("Callback accept failed: {error}"),
            }
        };
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let mut text = String::new();
        BufReader::new(stream.try_clone().unwrap())
            .read_line(&mut text)
            .unwrap();
        let request: Value = serde_json::from_str(&text).unwrap();
        seen.lock().unwrap().push(request.clone());
        stream.write_all(&reply(&request)).ok();
    });
    (
        descriptor(root, port, &"1".repeat(64), &"2".repeat(64)),
        received,
        handle,
    )
}

#[test]
fn source_witness_uses_standard_sha256_and_literal_byte_length() {
    let witness = SourceWitness::from_bytes(b"hello world");
    assert_eq!(witness.bytes, 11);
    assert_eq!(
        witness.sha256,
        "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
    );
}

#[test]
fn archive_navigation_requires_an_advertised_origin_session() {
    let temp = tempfile::tempdir().unwrap();
    let directory = temp.path().canonicalize().unwrap();
    let root = directory.join("project");
    std::fs::create_dir(&root).unwrap();
    let file = directory.join("origin.json");
    write_descriptor(
        &file,
        &descriptor(&root, 1, &"1".repeat(64), &"2".repeat(64)),
    );
    let editor = OriginEditor::from_file(&root, &file).unwrap();
    assert!(!editor.opens_java_archives());
    let location = ArchiveEditorLocation {
        capture: "a".repeat(64),
        source: EditorLocation {
            path: directory.join("helper-sources.jar"),
            line: 2,
            source: SourceWitness::from_bytes(b"package demo;\nclass Helper {}\n"),
        },
        binary: directory.join("helper.jar"),
        uri: "jdt://contents/helper.jar/demo/Helper.class?project".into(),
    };
    assert!(
        editor
            .open_archive(&location)
            .unwrap_err()
            .to_string()
            .contains("cannot open")
    );
}

#[test]
fn descriptor_is_pinned_and_replacement_cannot_take_over_return_navigation() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let file = root.parent().unwrap().join(format!(
        "origin-test-{}.json",
        root.file_name().unwrap().to_string_lossy()
    ));
    let (original, seen, worker) = peer(&root);
    write_descriptor(&file, &original);
    let editor = OriginEditor::from_file(&root, &file).unwrap();
    assert_eq!(editor.name(), "Fixture editor");
    let mut replacement = original.clone();
    replacement["session"] = json!("3".repeat(64));
    replacement["token"] = json!("4".repeat(64));
    replacement["port"] = json!(1);
    write_descriptor(&file, &replacement);
    let location = EditorLocation {
        path: root.join("a file;$(literal).rs"),
        line: 7,
        source: SourceWitness::from_bytes(b"fn first() {}\n"),
    };
    editor.open(&location).unwrap();
    worker.join().unwrap();
    std::fs::remove_file(file).unwrap();
    let requests = seen.lock().unwrap();
    assert_eq!(requests[0]["session"], original["session"]);
    assert_eq!(requests[0]["token"], original["token"]);
    assert_eq!(
        requests[0]["path"],
        location.path.to_string_lossy().as_ref()
    );
    assert_eq!(requests[0]["line"], 7);
    assert_eq!(requests[0]["protocol"], "vizzi-origin");
    assert_eq!(requests[0]["version"], 1);
    assert_eq!(requests[0]["root"], root.to_string_lossy().as_ref());
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    let expiry = requests[0]["expires_at_ms"].as_u64().unwrap();
    assert!(expiry > now && expiry <= now + 8000);
    assert_eq!(
        requests[0]["source"],
        serde_json::to_value(location.source).unwrap()
    );
}

#[test]
fn lost_origin_never_launches_a_standalone_editor() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    drop(listener);
    let file = root.parent().unwrap().join(format!(
        "origin-closed-{}.json",
        root.file_name().unwrap().to_string_lossy()
    ));
    write_descriptor(
        &file,
        &descriptor(&root, port, &"1".repeat(64), &"2".repeat(64)),
    );
    let editor = OriginEditor::from_file(&root, &file).unwrap();
    std::fs::remove_file(file).unwrap();
    let error = editor
        .open(&EditorLocation {
            path: root.join("a.rs"),
            line: 1,
            source: SourceWitness::from_bytes(b"x"),
        })
        .unwrap_err();
    assert!(error.to_string().contains("originating editor"));
}

#[test]
fn wrong_root_or_protocol_or_nonprivate_descriptors_are_refused() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let file = root.parent().unwrap().join(format!(
        "origin-invalid-{}.json",
        root.file_name().unwrap().to_string_lossy()
    ));
    let value = descriptor(&root, 12345, &"1".repeat(64), &"2".repeat(64));
    for changed in [
        json!({"root":"/other"}),
        json!({"protocol":"semantic"}),
        json!({"port":0}),
        json!({"token":"short"}),
        json!({"version":2}),
    ] {
        let mut invalid = value.clone();
        for (k, v) in changed.as_object().unwrap() {
            invalid[k] = v.clone();
        }
        write_descriptor(&file, &invalid);
        assert!(OriginEditor::from_file(&root, &file).is_err());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        write_descriptor(&file, &value);
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(OriginEditor::from_file(&root, &file).is_err());
    }
    std::fs::remove_file(file).unwrap();
}

#[test]
fn acknowledgments_must_match_the_request_and_preserve_editor_refusals() {
    for change in [
        json!({"protocol":"other"}),
        json!({"version":2}),
        json!({"session":"0".repeat(64)}),
        json!({"request":"0".repeat(64)}),
        json!({"status":"maybe"}),
        json!({"unexpected":true}),
        json!({"status":"error","code":"dirty_target","message":"Save the target buffer first"}),
        json!({"status":"error","message":"x".repeat(512),"code":"x".repeat(64)}),
        json!({"status":"error","message":"x".repeat(513)}),
        json!({"status":"error","code":"x".repeat(65)}),
        json!({"message":"x".repeat(513)}),
        json!({"code":"x".repeat(65)}),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("project");
        std::fs::create_dir(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let expected = change
            .get("message")
            .and_then(Value::as_str)
            .filter(|s| s.len() <= 512)
            .map(str::to_owned);
        let (value, _, worker) = peer_reply(&root, move |request| {
            let mut reply = json!({"protocol":"vizzi-origin","version":1,"session":request["session"],"request":request["request"],"status":"opened"});
            for (key, value) in change.as_object().unwrap() {
                reply[key] = value.clone();
            }
            format!("{reply}\n").into_bytes()
        });
        let file = temp.path().join("connection.json");
        write_descriptor(&file, &value);
        let editor = OriginEditor::from_file(&root, &file).unwrap();
        let error = editor
            .open(&EditorLocation {
                path: root.join("a.rs"),
                line: 1,
                source: SourceWitness::from_bytes(b"x"),
            })
            .unwrap_err();
        if let Some(expected) = expected {
            assert_eq!(error.to_string(), expected);
        }
        worker.join().unwrap();
    }
}

#[test]
fn lost_oversized_and_multiple_reply_frames_are_not_acknowledgments() {
    for reply in [vec![], vec![b'x'; 64 * 1024 + 1], b"{}\n{}\n".to_vec()] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("project");
        std::fs::create_dir(&root).unwrap();
        let root = root.canonicalize().unwrap();
        let (value, _, worker) = peer_reply(&root, move |_| reply);
        let file = temp.path().join("connection.json");
        write_descriptor(&file, &value);
        let editor = OriginEditor::from_file(&root, &file).unwrap();
        assert!(
            editor
                .open(&EditorLocation {
                    path: root.join("a.rs"),
                    line: 1,
                    source: SourceWitness::from_bytes(b"x")
                })
                .is_err()
        );
        worker.join().unwrap();
    }
}

#[test]
fn descriptors_cannot_live_in_source_or_exceed_the_private_regular_file_allowance() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    std::fs::create_dir(&root).unwrap();
    let root = root.canonicalize().unwrap();
    let value = descriptor(&root, 12345, &"1".repeat(64), &"2".repeat(64));
    let inside = root.join("connection.json");
    write_descriptor(&inside, &value);
    assert!(OriginEditor::from_file(&root, &inside).is_err());
    assert!(
        OriginEditor::from_file(&root, temp.path())
            .err()
            .unwrap()
            .to_string()
            .contains("regular file")
    );
    let file = temp.path().join("connection.json");
    std::fs::write(&file, vec![b' '; 4097]).unwrap();
    assert!(OriginEditor::from_file(&root, &file).is_err());
    for change in [
        json!({"session":"short"}),
        json!({"editor":""}),
        json!({"editor":"x".repeat(65)}),
        json!({"editor":"Bad\nname"}),
        json!({"extra":true}),
    ] {
        let mut invalid = value.clone();
        for (k, v) in change.as_object().unwrap() {
            invalid[k] = v.clone();
        }
        write_descriptor(&file, &invalid);
        assert!(OriginEditor::from_file(&root, &file).is_err());
    }
    #[cfg(unix)]
    {
        let fifo = temp.path().join("fifo");
        let created = std::process::Command::new("mkfifo")
            .args(["-m", "600"])
            .arg(&fifo)
            .output()
            .unwrap();
        assert!(created.status.success(), "mkfifo failed: {created:?}");
        assert!(
            OriginEditor::from_file(&root, &fifo)
                .err()
                .unwrap()
                .to_string()
                .contains("regular file")
        );
    }
}

#[test]
fn exact_descriptor_reply_and_source_allowances_are_accepted() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap().join("project");
    std::fs::create_dir(&root).unwrap();
    let (mut value, seen, worker) = peer_reply(&root, |request| {
        let mut reply=format!("{}",json!({"protocol":"vizzi-origin","version":1,"session":request["session"],"request":request["request"],"status":"opened"})).into_bytes();
        reply.resize(64 * 1024 - 1, b' ');
        reply.push(b'\n');
        reply
    });
    value["editor"] = json!("x".repeat(64));
    let file = temp.path().join("connection.json");
    let mut contents = value.to_string();
    contents.extend(std::iter::repeat_n(' ', 4096 - contents.len()));
    std::fs::write(&file, contents).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o600)).unwrap();
    }
    let editor = OriginEditor::from_file(&root, &file).unwrap();
    assert_eq!(editor.name().len(), 64);
    let location = EditorLocation {
        path: Path::new("/").join("x".repeat(4095)),
        line: 1,
        source: SourceWitness {
            bytes: 16 * 1024 * 1024,
            sha256: "abcdef0123456789".repeat(4),
        },
    };
    editor.open(&location).unwrap();
    worker.join().unwrap();
    assert_eq!(seen.lock().unwrap()[0]["source"]["bytes"], 16 * 1024 * 1024);
    for invalid in [
        EditorLocation {
            path: "relative.rs".into(),
            ..location.clone()
        },
        EditorLocation {
            path: Path::new("/").join("x".repeat(4096)),
            ..location.clone()
        },
        EditorLocation {
            line: 0,
            ..location.clone()
        },
        EditorLocation {
            source: SourceWitness {
                bytes: 16 * 1024 * 1024 + 1,
                ..location.source.clone()
            },
            ..location.clone()
        },
        EditorLocation {
            source: SourceWitness {
                sha256: "G".repeat(64),
                ..location.source.clone()
            },
            ..location
        },
    ] {
        assert!(
            editor
                .open(&invalid)
                .unwrap_err()
                .to_string()
                .contains("Invalid verified")
        );
    }
}
