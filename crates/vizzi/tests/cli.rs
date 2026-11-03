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
        Self::start_with_budget(root, cache, 0)
    }
    fn start_with_budget(root: &std::path::Path, cache: &std::path::Path, budget: usize) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_vizzi"))
            .args(["--stdio", "--root"])
            .arg(root)
            .arg("--cache")
            .arg(cache)
            .arg("--syntax-cache-mib")
            .arg(budget.to_string())
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

#[test]
fn opt_in_retention_reuses_a_tree_through_the_live_host() {
    let source = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let path = source.path().join("a.rs");
    let original = (0..2000)
        .map(|index| format!("fn item_{index}() {{}}\n"))
        .collect::<String>();
    std::fs::write(&path, &original).unwrap();
    let mut session = Session::start_with_budget(source.path(), cache.path(), 8);
    let start = Instant::now();
    let mut previous = loop {
        let status = session.request("/api/status");
        if status["pending"] == false && status["measurements"]["reused"] == 1 {
            break status["snapshot"].clone();
        }
        assert!(start.elapsed() < Duration::from_secs(8));
        std::thread::sleep(Duration::from_millis(5));
    };
    for index in 1..=2 {
        std::fs::write(&path, format!("// edit {index}\n{original}")).unwrap();
        let start = Instant::now();
        let updated = loop {
            let status = session.request("/api/status");
            if status["snapshot"] != previous {
                break status;
            }
            assert!(start.elapsed() < Duration::from_secs(8));
            std::thread::sleep(Duration::from_millis(5));
        };
        assert_eq!(
            updated["measurements"]["tree_reuses"],
            if index == 1 { 0 } else { 1 }
        );
        previous = updated["snapshot"].clone();
    }
}

#[test]
fn invalid_retention_budgets_fail_without_populating_the_analysis_cache() {
    let source = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(source.path().join("a.rs"), "fn example() {}").unwrap();
    for value in ["-1", "invalid", &usize::MAX.to_string()] {
        let output = Command::new(env!("CARGO_BIN_EXE_vizzi"))
            .arg("--root")
            .arg(source.path())
            .arg("--cache")
            .arg(cache.path())
            .args(["--once", "--syntax-cache-mib", value])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert_eq!(std::fs::read_dir(cache.path()).unwrap().count(), 0);
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
    std::fs::write(source.path().join("b.rs"), "fn untouched() {}").unwrap();
    let mut session = Session::start(source.path(), cache.path());
    let ready = Instant::now();
    loop {
        let status = session.request("/api/status");
        if status["pending"] == false
            && status["measurements"]["reconciled"] == true
            && status["measurements"]["reused"] == 2
        {
            break;
        }
        assert!(
            ready.elapsed() < Duration::from_secs(8),
            "initial reconciliation did not finish"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
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
    let status = session.request("/api/status");
    assert_eq!(status["measurements"]["read_files"], 1);
    assert_eq!(status["measurements"]["retained"], 1);
    assert_eq!(status["measurements"]["reconciled"], false);
}

#[test]
fn reconciliation_runs_while_write_hints_keep_arriving() {
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    let source = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let file = source.path().join("a.rs");
    std::fs::write(&file, "fn a() {}").unwrap();
    let mut session = Session::start(source.path(), cache.path());
    let initial = Instant::now();
    let baseline = loop {
        let state = session.request("/api/status");
        if state["pending"] == false
            && state["measurements"]["reused"] == 1
            && state["measurements"]["reconciled"] == true
        {
            break (
                state["snapshot"].clone(),
                state["recovery"]["completed"].as_u64().unwrap(),
            );
        }
        assert!(initial.elapsed() < Duration::from_secs(8));
        std::thread::sleep(Duration::from_millis(5));
    };
    let stop = Arc::new(AtomicBool::new(false));
    let writer_stop = stop.clone();
    let writer = std::thread::spawn(move || {
        let mut number = 0;
        while !writer_stop.load(Ordering::Acquire) {
            std::fs::write(&file, format!("fn a() {{ let n = {number}; }}")).unwrap();
            number += 1;
            std::thread::sleep(Duration::from_millis(75));
        }
    });
    let started = Instant::now();
    let mut recovered = false;
    while started.elapsed() < Duration::from_secs(6) {
        let state = session.request("/api/status");
        if state["snapshot"] != baseline.0
            && state["pending"] == false
            && state["recovery"]["completed"].as_u64().unwrap() > baseline.1
        {
            recovered = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    stop.store(true, Ordering::Release);
    writer.join().unwrap();
    assert!(
        recovered,
        "continuous hints starved periodic reconciliation"
    );
}

#[test]
fn newly_discovered_directories_keep_their_file_watches() {
    let source = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(source.path().join("a.rs"), "fn a() {}").unwrap();
    let mut session = Session::start(source.path(), cache.path());
    let wait = Instant::now();
    loop {
        let status = session.request("/api/status");
        if status["pending"] == false && status["measurements"]["reused"] == 1 {
            break;
        }
        assert!(wait.elapsed() < Duration::from_secs(8));
        std::thread::sleep(Duration::from_millis(5));
    }
    std::fs::create_dir(source.path().join("nested")).unwrap();
    let file = source.path().join("nested/new.rs");
    std::fs::write(&file, "fn new() {}").unwrap();
    let wait = Instant::now();
    let previous = loop {
        let status = session.request("/api/status");
        if status["pending"] == false
            && status["measurements"]["files"] == 2
            && status["measurements"]["reused"] == 2
        {
            break status["snapshot"].clone();
        }
        assert!(wait.elapsed() < Duration::from_secs(8));
        std::thread::sleep(Duration::from_millis(5));
    };
    std::fs::write(&file, "fn updated() {}").unwrap();
    let wait = Instant::now();
    let status = loop {
        let status = session.request("/api/status");
        if status["snapshot"] != previous && status["pending"] == false {
            break status;
        }
        assert!(wait.elapsed() < Duration::from_secs(8));
        std::thread::sleep(Duration::from_millis(5));
    };
    assert_eq!(status["measurements"]["reconciled"], false);
    assert_eq!(status["measurements"]["read_files"], 1);
}

#[test]
fn admission_option_preserves_source_and_rejects_invalid_budgets() {
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("a.rs"),
        " ".repeat(3 * 1024 * 1024 / 4 + 1),
    )
    .unwrap();
    std::fs::write(root.path().join("b.rs"), " ".repeat(3 * 1024 * 1024 / 4)).unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_vizzi"))
        .args([
            "--root",
            root.path().to_str().unwrap(),
            "--cache",
            cache.path().to_str().unwrap(),
            "--once",
            "--analysis-input-mib",
            "1",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let status: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(status["measurements"]["deferred"], 1);
    assert_eq!(status["measurements"]["files"], 2);
    assert_eq!(status["measurements"]["parsed"], 1);
    for value in ["invalid", "18446744073709551615"] {
        let output = std::process::Command::new(env!("CARGO_BIN_EXE_vizzi"))
            .args([
                "--root",
                root.path().to_str().unwrap(),
                "--cache",
                cache.path().to_str().unwrap(),
                "--once",
                "--analysis-input-mib",
                value,
            ])
            .output()
            .unwrap();
        assert!(!output.status.success());
    }
}
