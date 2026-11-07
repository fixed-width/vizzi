//! Native ranking retains the exact source pool and rejects expired contexts.
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
};

#[test]
fn native_reordering_preserves_sources_and_refuses_invalid_stale_or_expired_orders() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("a.rs"), "pub fn transaction_new() {}\n").unwrap();
    std::fs::write(root.path().join("b.rs"), "pub fn transaction_start() {}\n").unwrap();
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
    assert_eq!(read(&mut output)["ready"], true);
    let mut request = |body: Value| {
        writeln!(input, "{body}").unwrap();
        input.flush().unwrap();
        read(&mut output)
    };
    let retrieve = json!({"op":"retrieve","id":"held","question":"transaction","selector":"compact","limit":8,"budget":4000});
    let baseline = request(retrieve.clone());
    let mut expected = baseline["context"].clone();
    assert_eq!(expected["evidence"].as_array().unwrap().len(), 2);
    expected["evidence"].as_array_mut().unwrap().reverse();
    let ranked = request(json!({"op":"reorder","id":"held","order":[2,1]}));
    assert_eq!(ranked["context"], expected);
    assert_eq!(ranked["relevance"], "not_checked");
    assert_eq!(
        request(json!({"op":"reorder","id":"held","order":[1,2]}))["context"],
        baseline["context"]
    );
    for body in [
        json!({"op":"reorder","id":"held","order":[]}),
        json!({"op":"reorder","id":"held","order":[1,1]}),
        json!({"op":"reorder","id":"held","order":[1,2,3]}),
        json!({"op":"reorder","id":"held","order":[-1,2]}),
        json!({"op":"reorder","id":"held","order":[1.5,2]}),
        json!({"op":"reorder","id":"held","order":[1,2],"path":"injected.rs"}),
        json!({"op":"reorder","id":"unknown","order":[1,2]}),
    ] {
        assert!(request(body).get("error").is_some());
    }
    std::fs::write(
        root.path().join("a.rs"),
        "pub fn transaction_changed() {}\n",
    )
    .unwrap();
    assert!(request(json!({"op":"refresh"})).get("error").is_none());
    assert!(
        request(json!({"op":"reorder","id":"held","order":[1,2]}))["error"]
            .as_str()
            .unwrap()
            .contains("different source snapshot")
    );
    assert!(request(retrieve.clone()).get("error").is_none());
    for i in 0..64 {
        let mut next = retrieve.clone();
        next["id"] = json!(format!("other-{i}"));
        assert!(request(next).get("error").is_none());
    }
    assert!(
        request(json!({"op":"reorder","id":"held","order":[1,2]}))["error"]
            .as_str()
            .unwrap()
            .contains("expired or unknown")
    );
    drop(input);
    assert!(child.wait().unwrap().success());
}
