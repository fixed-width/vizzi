//! Native source packs remain tied to held contexts and captured identities.
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
};

#[test]
fn native_selection_refuses_stale_and_injected_ranges_without_replacing_a_held_context() {
    let root = tempfile::tempdir().unwrap();
    let text = "// caller\nfn submit() {}\n";
    std::fs::write(root.path().join("a.rs"), text).unwrap();
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
        serde_json::from_str::<Value>(&line).unwrap()
    };
    let ready = read(&mut output);
    let snapshot = ready["snapshot"].clone();
    let mut request = |body: Value| {
        writeln!(input, "{body}").unwrap();
        input.flush().unwrap();
        read(&mut output)
    };
    let range = json!({"path":"a.rs","content":blake3::hash(text.as_bytes()).to_hex().to_string(),"start":10,"end":text.len()});
    let selection =
        json!({"op":"select","id":"selected","snapshot":snapshot,"ranges":[range],"budget":16000});
    let selected = request(selection.clone());
    assert!(selected.get("error").is_none(), "{selected}");
    assert_eq!(
        selected["context"]["evidence"][0]["text"],
        "fn submit() {}\n"
    );
    assert_eq!(selected["context"]["evidence"][0]["start_line"], 2);
    let validate = json!({"op":"validate_ids","id":"selected","answer":{"status":"answered","answer":"submit is the caller","citations":[1]}});
    assert_eq!(request(validate.clone())["references"][0]["path"], "a.rs");
    for field in ["text", "unexpected"] {
        let mut bad = selection.clone();
        bad["ranges"][0][field] = json!("injected");
        assert!(request(bad).get("error").is_some());
    }
    let mut bad = selection.clone();
    bad["ranges"][0]["end"] = json!(text.len() + 1);
    assert!(request(bad).get("error").is_some());
    assert_eq!(request(validate.clone())["provenance_valid"], true);
    for (size, ok) in [(256, true), (257, false)] {
        let mut body = selection.clone();
        body["id"] = json!("x".repeat(size));
        assert_eq!(request(body).get("error").is_none(), ok);
    }
    std::fs::write(root.path().join("a.rs"), "fn replacement() {}\n").unwrap();
    assert!(request(json!({"op":"refresh"})).get("error").is_none());
    assert!(
        request(selection)["error"]
            .as_str()
            .unwrap()
            .contains("different source snapshot")
    );
    assert!(
        request(validate)["error"]
            .as_str()
            .unwrap()
            .contains("different source snapshot")
    );
    drop(input);
    assert!(child.wait().unwrap().success());
}
