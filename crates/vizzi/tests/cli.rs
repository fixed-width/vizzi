//! Live diagnostic transport with native observation and atomic source replacement.

use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

struct Session {
    child: Child,
    input: Option<ChildStdin>,
    output: mpsc::Receiver<String>,
}
impl Session {
    fn start(root: &std::path::Path, cache: &std::path::Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_vizzi"))
            .args(["--stdio", "--root"])
            .arg(root)
            .arg("--cache")
            .arg(cache)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let input = child.stdin.take();
        let output = child.stdout.take().unwrap();
        let (send, receive) = mpsc::channel();
        std::thread::spawn(move || {
            for line in BufReader::new(output).lines() {
                if send.send(line.unwrap()).is_err() {
                    break;
                }
            }
        });
        Self {
            child,
            input,
            output: receive,
        }
    }
    fn request(&mut self, uri: &str) -> Value {
        writeln!(
            self.input.as_mut().unwrap(),
            "{}",
            json!({"id": 42, "uri": uri})
        )
        .unwrap();
        self.input.as_mut().unwrap().flush().unwrap();
        let response: Value =
            serde_json::from_str(&self.output.recv_timeout(Duration::from_secs(8)).unwrap())
                .unwrap();
        assert_eq!(response["status"], 200);
        assert_eq!(response["id"], 42);
        serde_json::from_str(response["body"].as_str().unwrap()).unwrap()
    }
}
impl Drop for Session {
    fn drop(&mut self) {
        self.input.take();
        if std::thread::panicking() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}
#[test]
fn live_edit_keeps_selection_and_pinned_source_coherent() {
    let source = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(source.path().join("a.rs"), "fn one() {}").unwrap();
    let mut session = Session::start(source.path(), cache.path());
    let first = session.request("/api/view");
    let selected = &first["source"]["outlines"][0]["declarations"][0]["id"];
    let uri = format!(
        "/api/view?file={}&entity={}",
        first["selection"]["file"].as_str().unwrap(),
        selected.as_str().unwrap()
    );
    let mut replacement = tempfile::NamedTempFile::new_in(source.path()).unwrap();
    replacement
        .write_all(b"// shifted\nfn one() { let changed = 2; }")
        .unwrap();
    replacement.persist(source.path().join("a.rs")).unwrap();
    let started = Instant::now();
    let updated = loop {
        let view = session.request(&uri);
        if view["snapshot"] != first["snapshot"] {
            break view;
        }
        assert!(
            started.elapsed() < Duration::from_secs(8),
            "no observation published after an edit"
        );
        std::thread::sleep(Duration::from_millis(10));
    };
    assert_eq!(updated["selection"]["entity"], *selected);
    assert_eq!(
        updated["source"]["outlines"][0]["declarations"][0]["start_line"],
        2
    );
    assert_eq!(
        updated["source"]["text"],
        "// shifted\nfn one() { let changed = 2; }"
    );
    let pinned = session.request(&format!(
        "/api/source?snapshot={}&file={}",
        first["snapshot"].as_str().unwrap(),
        first["selection"]["file"].as_str().unwrap()
    ));
    assert_eq!(pinned["text"], "fn one() {}");
}
