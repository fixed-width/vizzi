//! Measure native notification delivery on an owned temporary fixture.

use notify::{RecursiveMode, Watcher};
use serde_json::json;
use std::{
    collections::BTreeMap,
    error::Error,
    fs,
    path::{Path, PathBuf},
    sync::mpsc,
    time::{Duration, Instant},
};
use vizzi_perf::{
    snapshot::{Fingerprint, Manifest},
    source, timing,
    watch::PendingChanges,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
type Notice = (Instant, notify::Result<notify::Event>);

fn path(index: usize) -> PathBuf {
    PathBuf::from(format!("group-{:05}/file-{index:08}.rs", index / 100))
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let files: usize = args.next().as_deref().unwrap_or("1000").parse()?;
    let samples: usize = args.next().as_deref().unwrap_or("50").parse()?;
    let needed = samples
        .checked_mul(2)
        .and_then(|n| n.checked_add(1))
        .ok_or("sample count too large")?;
    if samples == 0 || files < needed || files > 1_000_000 || args.next().is_some() {
        return Err("use vizzi-watch FILES SAMPLES; FILES must be 2*SAMPLES+1..=1000000".into());
    }
    let fixture = tempfile::tempdir()?;
    let root = fixture.path().canonicalize()?;
    let original = b"pub fn f() {}\n";
    let changed = b"pub fn g() {}\n";
    for index in 0..files {
        let target = root.join(path(index));
        fs::create_dir_all(target.parent().ok_or("missing parent")?)?;
        fs::write(target, original)?;
    }
    let mut manifest = source::capture(&root)?;
    let (tx, rx) = mpsc::channel();
    let mut watcher = notify::recommended_watcher(move |event| {
        let _ = tx.send((Instant::now(), event));
    })?;
    let start = Instant::now();
    watcher.watch(&root, RecursiveMode::Recursive)?;
    let registration = start.elapsed();
    let mut reports = Vec::new();
    for (mode, offset) in [("same_length_write", 0), ("atomic_replacement", samples)] {
        let mut callbacks = Vec::new();
        let mut verified = Vec::new();
        let mut writes = Vec::new();
        let mut rescan_requests = 0;
        for index in 0..samples {
            let relative = path(index + offset);
            let target = root.join(&relative);
            let started = Instant::now();
            if mode == "atomic_replacement" {
                let temporary = target.with_extension("temporary");
                fs::write(&temporary, changed)?;
                fs::rename(temporary, &target)?;
            } else {
                fs::write(&target, changed)?;
            }
            writes.push(started.elapsed());
            let observed = observe(&root, &relative, started, &rx, &mut manifest, changed)?;
            callbacks.push(observed.0);
            verified.push(observed.1);
            rescan_requests += observed.2;
        }
        reports.push(json!({"mode":mode,"attempted":samples,"successful":samples,
            "write_call":timing::summarize(&writes),"write_start_to_callback":timing::summarize(&callbacks),
            "write_start_to_verified_manifest_update":timing::summarize(&verified),"rescan_requests_observed":rescan_requests}));
    }
    let dropped = path(samples * 2);
    fs::write(root.join(&dropped), changed)?;
    let started = Instant::now();
    let recovered = source::capture(&root)?;
    let differences = recovered.changes_from(&manifest);
    let recovery_time = started.elapsed();
    if differences.len() != 1
        || differences[0].path != dropped
        || recovered.files().get(&dropped) != Some(&Fingerprint::of(changed))
    {
        return Err("reconciliation did not recover the deliberately ignored event".into());
    }
    let output = json!({"experiment":"native_filesystem_events","status":"complete","files":files,
    "registration_us":registration.as_nanos() as f64/1000.0,"operations":reports,
    "recovery":{"event_deliberately_ignored":true,"recovered":true,"changed_paths":differences.len(),"scan_and_compare_us":recovery_time.as_nanos() as f64/1000.0},
    "conditions":{"build":if cfg!(debug_assertions) {"debug"} else {"release"},"os":std::env::consts::OS,
        "fixture":"owned temporary directory; 100 files per directory; distinct file per operation; source repositories not modified",
        "event_clock":"watcher callback records arrival time; notifications may precede completion of the write",
        "verification":"fingerprint file after a candidate event, then update the mutable manifest; five-second deadline per operation",
        "coverage":"native filesystem notifications and content verification only; no parsing, semantic invalidation, immutable publication, or UI",
        "overflow":"recovery deliberately discards one event; actual operating-system queue overflow is not induced",
        "failure_policy":"timeouts, watcher failures, and incorrect updates abort the run rather than being omitted from summaries"
    }});
    drop(watcher);
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn observe(
    root: &Path,
    relative: &Path,
    started: Instant,
    rx: &mpsc::Receiver<Notice>,
    manifest: &mut Manifest,
    expected: &[u8],
) -> Result<(Duration, Duration, usize)> {
    let mut pending = PendingChanges::new(root.to_path_buf());
    let deadline = started + Duration::from_secs(5);
    let mut rescans = 0;
    loop {
        let (received, event) =
            rx.recv_timeout(deadline.saturating_duration_since(Instant::now()))?;
        if received < started {
            continue;
        }
        pending.record(event);
        let batch = pending.take();
        rescans += usize::from(batch.reconcile);
        if batch.reconcile {
            return Err("watcher requested reconciliation during the latency probe".into());
        }
        if !batch.paths.contains(relative) {
            continue;
        }
        let fingerprint = source::fingerprint_file(&root.join(relative))?;
        if fingerprint != Fingerprint::of(expected) {
            continue;
        }
        let changes = manifest.apply(BTreeMap::from([(
            relative.to_path_buf(),
            Some(fingerprint),
        )]));
        if changes.len() != 1 {
            return Err("verified notification did not change exactly one manifest entry".into());
        }
        return Ok((received.duration_since(started), started.elapsed(), rescans));
    }
}
