//! Native experiment capture never executes source or follows source symlinks.
use vizzi_qa::native::capture;
#[test]
fn capture_includes_docs_and_manifests_but_excludes_artifacts_and_lpc_source() {
    let root = tempfile::tempdir().unwrap();
    for (path, text) in [
        ("src/a.rs", "fn a() {}"),
        ("README.md", "Read this"),
        ("Cargo.toml", "[workspace]"),
        ("LICENSE", "Permission is hereby granted"),
        ("web/view.js", "export const view = true;"),
        ("test.c", "LPC content"),
        ("results/answer.md", "gold"),
        ("experiments/qa/library.md", "gold"),
    ] {
        let file = root.path().join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(file, text).unwrap();
    }
    let (files, stats) = capture(root.path()).unwrap();
    assert_eq!(stats.selected, 5);
    assert_eq!(stats.omitted, 3);
    assert_eq!(
        files
            .iter()
            .map(|f| f.path.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        [
            "Cargo.toml",
            "LICENSE",
            "README.md",
            "src/a.rs",
            "web/view.js"
        ]
        .into_iter()
        .collect()
    );
    assert!(files.iter().any(|f| f.path == "Cargo.toml"));
    assert!(!files.iter().any(|f| f.text == "gold"));
}

#[test]
fn acquisition_ceilings_are_enforced_without_large_test_repositories() {
    use vizzi_qa::native::{capture_with_limits, pilot_limits};
    let limits = pilot_limits();
    assert_eq!(limits.max_paths, 100_000);
    assert_eq!(limits.max_file_bytes, 2_097_152);
    assert_eq!(limits.max_bytes, 33_554_432);
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("one.md"), "first doc").unwrap();
    std::fs::write(root.path().join("two.md"), "other doc").unwrap();
    for limited in [
        vizzi::review_source::CaptureLimits {
            max_paths: 1,
            ..limits
        },
        vizzi::review_source::CaptureLimits {
            max_file_bytes: 4,
            ..limits
        },
        vizzi::review_source::CaptureLimits {
            max_bytes: 12,
            ..limits
        },
    ] {
        assert!(capture_with_limits(root.path(), limited).is_err());
    }
    assert_eq!(
        capture_with_limits(root.path(), limits).unwrap().1.selected,
        2
    );
    assert_eq!(
        capture_with_limits(
            root.path(),
            vizzi::review_source::CaptureLimits {
                max_bytes: 18,
                ..limits
            }
        )
        .unwrap()
        .1
        .selected,
        2
    );
}

#[cfg(unix)]
#[test]
fn non_utf8_source_paths_and_text_are_counted_as_omissions() {
    use std::os::unix::ffi::OsStringExt;
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("bad.md"), [0xff]).unwrap();
    std::fs::write(
        root.path()
            .join(std::ffi::OsString::from_vec(b"bad-\xff.rs".to_vec())),
        "fn x() {}",
    )
    .unwrap();
    let (files, stats) = capture(root.path()).unwrap();
    assert!(files.is_empty());
    assert_eq!(stats.omitted, 2);
}

#[test]
fn protocol_keeps_sixty_four_contexts_then_expires_the_oldest() {
    use std::{
        io::{BufRead, BufReader, Write},
        process::{Command, Stdio},
    };
    let root = tempfile::tempdir().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_vizzi-qa"))
        .arg(root.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let read = |output: &mut BufReader<std::process::ChildStdout>| {
        let mut line = String::new();
        output.read_line(&mut line).unwrap();
        serde_json::from_str::<serde_json::Value>(&line).unwrap()
    };
    read(&mut output);
    for id in 0..64 {
        writeln!(
            input,
            "{}",
            serde_json::json!({"op":"retrieve","id":id.to_string(),"question":"needle"})
        )
        .unwrap();
        input.flush().unwrap();
        assert!(read(&mut output).get("error").is_none());
    }
    let validate = serde_json::json!({"op":"validate","id":"0","answer":{"status":"insufficient_evidence","answer":"No supplied evidence","citations":[]}});
    writeln!(input, "{validate}").unwrap();
    input.flush().unwrap();
    assert_eq!(read(&mut output)["provenance_valid"], true);
    writeln!(
        input,
        "{}",
        serde_json::json!({"op":"retrieve","id":"extra","question":"needle"})
    )
    .unwrap();
    input.flush().unwrap();
    read(&mut output);
    writeln!(input, "{validate}").unwrap();
    input.flush().unwrap();
    assert!(
        read(&mut output)["error"]
            .as_str()
            .unwrap()
            .contains("expired")
    );
    for (length, ok) in [(256, true), (257, false)] {
        writeln!(
            input,
            "{}",
            serde_json::json!({"op":"retrieve","id":"i".repeat(length),"question":"needle"})
        )
        .unwrap();
        input.flush().unwrap();
        assert_eq!(read(&mut output).get("error").is_none(), ok);
    }
    drop(input);
    assert!(child.wait().unwrap().success());
}

#[test]
fn protocol_frame_boundary_preserves_alignment_and_refuses_one_more_byte() {
    use std::{
        io::{BufRead, BufReader, Write},
        process::{Command, Stdio},
    };
    let root = tempfile::tempdir().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_vizzi-qa"))
        .arg(root.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let read = |output: &mut BufReader<std::process::ChildStdout>| {
        let mut line = String::new();
        output.read_line(&mut line).unwrap();
        serde_json::from_str::<serde_json::Value>(&line).unwrap()
    };
    read(&mut output);
    let mut frame = String::from("{\"op\":\"refresh\"}");
    frame.push_str(&" ".repeat(128 * 1024 - 1 - frame.len()));
    frame.push('\n');
    input.write_all(frame.as_bytes()).unwrap();
    input.write_all(b"{\"op\":\"refresh\"}\n").unwrap();
    input.flush().unwrap();
    assert!(read(&mut output).get("error").is_none());
    assert!(read(&mut output).get("error").is_none());
    drop(input);
    assert!(child.wait().unwrap().success());
    let mut child = Command::new(env!("CARGO_BIN_EXE_vizzi-qa"))
        .arg(root.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    frame.insert(frame.len() - 1, ' ');
    let _ = child.stdin.take().unwrap().write_all(frame.as_bytes());
    let result = child.wait_with_output().unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("128 KiB"));
}
#[cfg(unix)]
#[test]
fn source_symlinks_are_omitted_instead_of_opening_their_targets() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let target = outside.path().join("outside.rs");
    std::fs::write(&target, "private outside source").unwrap();
    std::os::unix::fs::symlink(target, root.path().join("link.rs")).unwrap();
    let (files, stats) = capture(root.path()).unwrap();
    assert!(files.is_empty());
    assert_eq!(stats.omitted, 1);
}
#[test]
fn native_protocol_detects_same_metadata_edits_and_refuses_old_contexts() {
    use std::{
        io::{BufRead, BufReader, Write},
        process::{Command, Stdio},
    };
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("config.toml");
    std::fs::write(&path, "retry_limit = 3\n").unwrap();
    let modified = std::fs::metadata(&path).unwrap().modified().unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_vizzi-qa"))
        .arg(root.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let read = |output: &mut BufReader<std::process::ChildStdout>| {
        let mut line = String::new();
        output.read_line(&mut line).unwrap();
        serde_json::from_str::<serde_json::Value>(&line).unwrap()
    };
    let first = read(&mut output);
    assert_eq!(first["ready"], true);
    writeln!(
        input,
        "{}",
        serde_json::json!({"op":"retrieve","id":"old","question":"retry_limit"})
    )
    .unwrap();
    input.flush().unwrap();
    let context = read(&mut output);
    assert!(
        context["context"]["evidence"][0]["text"]
            .as_str()
            .unwrap()
            .contains("3")
    );
    std::fs::write(&path, "retry_limit = 7\n").unwrap();
    std::fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_times(std::fs::FileTimes::new().set_modified(modified))
        .unwrap();
    writeln!(input, "{}", serde_json::json!({"op":"refresh"})).unwrap();
    input.flush().unwrap();
    let refresh = read(&mut output);
    assert_eq!(refresh["stats"]["changed"], 1);
    assert_ne!(refresh["snapshot"], first["snapshot"]);
    writeln!(input,"{}",serde_json::json!({"op":"validate","id":"old","answer":{"status":"answered","answer":"Three retries","citations":[{"label":1,"quote":"retry_limit = 3"}]}})).unwrap();
    input.flush().unwrap();
    assert!(
        read(&mut output)["error"]
            .as_str()
            .unwrap()
            .contains("different source snapshot")
    );
    drop(input);
    assert!(child.wait().unwrap().success());
}
