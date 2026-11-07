//! Public stdio protocol options and ID validation.
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
};

#[test]
fn protocol_exposes_bounded_selection_and_native_id_resolution() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("README.md"),
        format!(
            "# Transport\n{}",
            "Packet transport configuration.\n".repeat(100)
        ),
    )
    .unwrap();
    std::fs::write(
        root.path().join("message.rs"),
        "struct Message; impl Message { fn new() -> Self { Self } }\n",
    )
    .unwrap();
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
    let legacy = request(json!({"op":"retrieve","id":"legacy","question":"Transport"}));
    let explicit = request(
        json!({"op":"retrieve","id":"explicit","question":"Transport","selector":"legacy","limit":6,"budget":8000}),
    );
    assert_eq!(legacy["context"], explicit["context"]);
    let compact = request(
        json!({"op":"retrieve","id":"small","question":"Transport","selector":"compact","limit":1,"budget":160}),
    );
    let evidence = compact["context"]["evidence"]
        .as_array()
        .expect("compact retrieval must succeed");
    assert_eq!(evidence.len(), 1);
    assert!(evidence[0]["text"].as_str().unwrap().len() <= 160);
    let validation = request(
        json!({"op":"validate_ids","id":"small","answer":{"status":"answered","answer":"Packet transport","citations":[1]}}),
    );
    let searched = request(
        json!({"op":"retrieve","id":"searched","question":"where are messages created?","selector":"search","limit":1,"budget":160}),
    );
    assert!(searched.get("error").is_none(), "{searched}");
    assert_eq!(searched["context"]["evidence"][0]["path"], "message.rs");
    assert_eq!(
        request(json!({"op":"reorder","id":"searched","order":[1]}))["context"],
        searched["context"]
    );
    assert_eq!(validation["provenance_valid"], true);
    assert_eq!(validation["references"][0]["path"], "README.md");
    assert_eq!(
        validation["references"][0]["snapshot"],
        compact["context"]["snapshot"]
    );
    for options in [
        json!({"selector":"guessed"}),
        json!({"limit":0}),
        json!({"limit":9}),
        json!({"budget":0}),
        json!({"budget":16001}),
        json!({"extra":true}),
    ] {
        let mut body = json!({"op":"retrieve","id":"bad","question":"Transport"});
        body.as_object_mut()
            .unwrap()
            .extend(options.as_object().unwrap().clone());
        assert!(request(body).get("error").is_some());
    }
    assert!(request(json!({"op":"validate_ids","id":"missing","answer":{"status":"answered","answer":"Transport","citations":[1]}})).get("error").is_some());
    std::fs::write(root.path().join("README.md"), "# Changed transport\n").unwrap();
    assert!(request(json!({"op":"refresh"})).get("error").is_none());
    assert!(request(json!({"op":"validate_ids","id":"small","answer":{"status":"answered","answer":"Transport","citations":[1]}}))["error"].as_str().unwrap().contains("different source snapshot"));
    drop(input);
    assert!(child.wait().unwrap().success());
}
