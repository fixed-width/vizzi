//! Optional real-host foreground edit measurements around observed tail truncation.
use crate::{
    application::{Application, Status},
    cache::{Cache, external_path},
    engine::Engine,
    host::{Host, HostOptions},
    source::{CaptureScope, Directory},
    source_spool::{CaptureTrace, SourceSpool},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    io::Read,
    path::PathBuf,
    sync::{Arc, mpsc},
    time::{Duration, Instant},
};
use vizzi_core::{
    analysis::Languages,
    model::{Analysis, CapturedSource, ContentId, Document},
    policy::PolicyConfig,
};
pub(crate) type Error = Box<dyn std::error::Error>;

#[derive(Deserialize)]
pub(crate) struct Config {
    pub(crate) root: PathBuf,
    pub(crate) cache: PathBuf,
    pub(crate) policy: PolicyConfig,
    pub(crate) target: String,
    pub(crate) owner: String,
    pub(crate) probe: String,
    pub(crate) spool_mib: usize,
    pub(crate) tail_mib: usize,
    pub(crate) pairs: usize,
}
pub(crate) fn engine(config: &Config, spool: Option<Arc<SourceSpool>>) -> Result<Engine, Error> {
    let mut directory = Directory::with_policy(
        &config.root,
        config.policy.clone(),
        Arc::new(Languages::builtin()?),
    )?;
    if let Some(spool) = spool {
        directory = directory.with_spool(spool)?;
    }
    Ok(Engine::new(
        directory,
        Cache::new(external_path(&config.cache, &config.root)?),
        4,
    )?)
}
pub(crate) fn wait(
    app: &Application,
    timeout: Duration,
    mut predicate: impl FnMut() -> bool,
) -> Result<(), Error> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(error) = app.status().error {
            return Err(format!("foreground host failed: {error}").into());
        }
        if predicate() {
            return Ok(());
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err("foreground probe timed out".into());
        }
        std::thread::sleep(remaining.min(Duration::from_millis(2)));
    }
}
pub(crate) fn initial_ready(status: &Status) -> bool {
    !status.pending && !status.recovery.running && status.recovery.completed >= 1
}
pub(crate) fn settled(pending: bool, matches: impl FnOnce() -> bool) -> bool {
    !pending && matches()
}
pub(crate) fn capture_mark(
    marks: &[(Instant, Instant)],
    read_files: usize,
) -> Result<(Instant, Instant), Error> {
    let [mark] = marks else {
        return Err("foreground edit was not a one-file capture".into());
    };
    if read_files != 1 {
        return Err("foreground edit was not a one-file capture".into());
    }
    if mark.1 < mark.0 {
        return Err("capture readiness precedes entry".into());
    }
    Ok(*mark)
}
pub(crate) fn validate_tail(tail: &CapturedSource, bytes: usize) -> Result<(), Error> {
    if tail.len() != bytes {
        return Err("pressure tail length differs".into());
    }
    let last = bytes.checked_sub(1).ok_or("pressure tail is empty")?;
    if tail.reader().read(last..bytes, 1)?.as_ref() != b"z" {
        return Err("pressure tail bytes differ".into());
    }
    Ok(())
}
pub(crate) fn verify_publication(
    source: &Document,
    bytes: &[u8],
    fresh: &Analysis,
    name: &str,
    old: &CapturedSource,
    original: &[u8],
) -> Result<(), Error> {
    if source.captured().read_all(usize::MAX)?.bytes() != bytes {
        return Err("foreground source differs".into());
    }
    if source.analyses() != std::slice::from_ref(fresh) {
        return Err("foreground facts differ".into());
    }
    if !source.analyses()[0]
        .facts
        .iter()
        .any(|fact| fact.name == name)
    {
        return Err("expected declaration missing".into());
    }
    if old.read_all(usize::MAX)?.bytes() != original {
        return Err("old pin differs".into());
    }
    Ok(())
}
pub(crate) fn millis(clock: Instant, event: Instant) -> f64 {
    event.duration_since(clock).as_secs_f64() * 1000.0
}
pub(crate) fn edit(
    language: &str,
    original: &[u8],
    probe: &str,
    sample: usize,
) -> (Vec<u8>, String) {
    let name = format!("maintenance_edit_{sample}");
    let (marker, definition) = match language {
        "rust" => ("//", format!("fn {name}() {{}}\n")),
        "python" => ("#", format!("def {name}(): pass\n")),
        "ruby" => ("#", format!("def {name}; {sample}; end\n")),
        _ => ("//", format!("int {name}(void) {{ return {sample}; }}\n")),
    };
    let mut bytes = format!("{marker} foreground {probe} sample {sample}\n").into_bytes();
    bytes.extend_from_slice(original);
    bytes.push(b'\n');
    bytes.extend_from_slice(definition.as_bytes());
    (bytes, name)
}

pub(crate) fn budgets(config: &Config) -> Result<(usize, usize), Error> {
    if config.owner.is_empty()
        || config.probe.is_empty()
        || config.pairs == 0
        || config.pairs > 8
        || config.tail_mib == 0
        || std::fs::read_to_string(config.root.join(".vizzi-foreground-owned"))? != config.owner
    {
        return Err(
            "foreground probe requires owned source, positive budgets and 1..8 sample pairs".into(),
        );
    }
    let capacity = config
        .spool_mib
        .checked_mul(1024 * 1024)
        .ok_or("spool capacity overflow")?;
    let tail_bytes = config
        .tail_mib
        .checked_mul(1024 * 1024)
        .ok_or("tail length overflow")?;
    if tail_bytes >= capacity {
        return Err("tail must leave space for the live workspace".into());
    }
    Ok((capacity, tail_bytes))
}

/// Measures explicit-hint edits after completed cleanup and during real truncation.
/// Only a marked owned source copy is edited; the source and original observation are restored.
pub fn run(configuration: &[u8], emit: &mut impl FnMut(Value)) -> Result<Value, Error> {
    let config: Config = serde_json::from_slice(configuration)?;
    let (capacity, tail_bytes) = budgets(&config)?;
    let spool = Arc::new(SourceSpool::new(
        &config
            .root
            .parent()
            .ok_or("missing owned-copy parent")?
            .join("spool"),
        capacity,
    )?);
    emit(json!({"event":"baseline_start"}));
    let host = Host::start_with_options(
        engine(&config, Some(spool.clone()))?,
        HostOptions {
            watch: false,
            reconcile_interval: Duration::from_secs(3600),
            idle_maintenance: false,
        },
    )?;
    let app = host.application();
    wait(&app, Duration::from_secs(120), || {
        initial_ready(&app.status())
    })?;
    let baseline = app.snapshot().ok_or("missing baseline")?;
    let baseline_id = baseline.id().clone();
    let file = baseline
        .file_by_path(&config.target)
        .ok_or("target outside captured scope")?;
    if file.analyses().len() != 1 {
        return Err("target needs one assigned analyser".into());
    }
    let language = file.analyses()[0].context.language.clone();
    let original = file.captured().read_all(usize::MAX)?.bytes().to_vec();
    let path = config.root.join(&config.target);
    let mut cases = Vec::new();
    emit(json!({"event":"baseline_ready","files":baseline.documents().len()}));
    for pair in 0..config.pairs {
        let order = if pair % 2 == 0 {
            [false, true]
        } else {
            [true, false]
        };
        for overlap in order {
            let sample = cases.len();
            let condition = if overlap {
                "during_cleanup"
            } else {
                "after_cleanup"
            };
            emit(json!({"event":"pressure_start","sample":sample,"condition":condition}));
            let tail = spool
                .capture(
                    &mut std::io::repeat(b'z').take(tail_bytes as u64),
                    None,
                    &|| false,
                )?
                .ok_or("tail capture cancelled")?;
            validate_tail(&tail, tail_bytes)?;
            let case = std::thread::scope(|scope| -> Result<Value, Error> {
                let before = spool.stats();
                let trace = Arc::new(CaptureTrace::default());
                spool.trace_capture(Some(trace.clone()));
                let clock = Instant::now();
                let (entered, at_cut) = mpsc::channel();
                let maintenance_spool = spool.clone();
                let mut maintenance = Some(scope.spawn(move || {
                    drop(tail);
                    maintenance_spool.observe_reclaim(|cut| {
                        let _ = entered.send(cut);
                    })
                }));
                let observed_cut = at_cut.recv_timeout(Duration::from_secs(120))?;
                let completed = if overlap {
                    None
                } else {
                    Some(
                        maintenance
                            .take()
                            .ok_or("missing maintenance task")?
                            .join()
                            .map_err(|_| "maintenance panicked")??,
                    )
                };
                let (bytes, name) = edit(&language, &original, &config.probe, sample);
                let expected = ContentId::derive(&[&bytes]);
                emit(json!({"event":"phase_start","sample":sample,"condition":condition}));
                let write_started = Instant::now();
                std::fs::write(&path, &bytes)?;
                let hinted = Instant::now();
                host.request(CaptureScope::Paths(
                    [PathBuf::from(&config.target)].into_iter().collect(),
                ));
                wait(&app, Duration::from_secs(120), || {
                    settled(app.status().pending, || {
                        app.snapshot().is_some_and(|s| {
                            s.file_by_path(&config.target)
                                .is_some_and(|f| f.content() == &expected)
                        })
                    })
                })?;
                let published = Instant::now();
                let visible = app.snapshot().ok_or("missing foreground publication")?;
                let status = app.status();
                spool.trace_capture(None);
                let cleanup = if overlap {
                    maintenance
                        .take()
                        .ok_or("missing maintenance task")?
                        .join()
                        .map_err(|_| "maintenance panicked")??
                } else {
                    completed.flatten()
                }
                .ok_or("no actual truncate observed")?;
                let marks = trace
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .clone();
                let mark = capture_mark(
                    &marks,
                    status
                        .measurements
                        .as_ref()
                        .ok_or("foreground measurements missing")?
                        .read_files,
                )?;
                let source = visible
                    .file_by_path(&config.target)
                    .ok_or("changed source missing")?;
                let fresh = Languages::builtin()?
                    .select(&language)
                    .ok_or("analyser unavailable")?
                    .create()?
                    .analyze(&bytes)?;
                verify_publication(source, &bytes, &fresh, &name, file.captured(), &original)?;
                let gate = (mark.1 - mark.0).as_secs_f64() * 1000.0;
                let case = json!({"pair":pair,"sample":sample,"condition":condition,"tail_bytes":tail_bytes,
                "cleanup_start_ms":millis(clock,cleanup.started),"cleanup_end_ms":millis(clock,cleanup.finished),
                "cleanup_ms":(cleanup.finished-cleanup.started).as_secs_f64()*1000.0,"cut_bytes":cleanup.cut_bytes,
                "observed_cut_bytes":observed_cut,"write_start_ms":millis(clock,write_started),"hint_ms":millis(clock,hinted),
                "published_ms":millis(clock,published),"write_to_published_ms":(published-write_started).as_secs_f64()*1000.0,
                "capture_marks":[{"entered_ms":millis(clock,marks[0].0),"ready_ms":millis(clock,marks[0].1),"gate_ms":gate}],
                "write_overlaps_cleanup":write_started>=cleanup.started && write_started<cleanup.finished,
                "capture_overlaps_cleanup":marks[0].0>=cleanup.started && marks[0].0<cleanup.finished,
                "measurements":status.measurements,"spool_before":before,"spool_after":spool.stats(),
                "source_matches":true,"fresh_facts_match":true,"expected_declaration":true,"old_pin_matches":true});
                emit(json!({"event":"phase_done","sample":sample,"condition":condition}));
                Ok(case)
            })?;
            cases.push(case);
            std::fs::write(&path, &original)?;
            host.request(CaptureScope::Paths(
                [PathBuf::from(&config.target)].into_iter().collect(),
            ));
            wait(&app, Duration::from_secs(120), || {
                settled(app.status().pending, || {
                    app.snapshot().is_some_and(|s| s.id() == &baseline_id)
                })
            })?;
        }
    }
    let clean = engine(&config, None)?
        .refresh(None, &|| false)?
        .ok_or("clean verification cancelled")?;
    if clean.snapshot.id() != &baseline_id {
        return Err("restored source differs from clean observation".into());
    }
    let files = baseline.documents().len();
    drop(clean);
    drop(baseline);
    drop(host);
    drop(app);
    spool.reclaim()?;
    Ok(
        json!({"files":files,"cases":cases,"baseline_snapshot":baseline_id,"restored_matches":true,
        "final_clean_matches":true,"spool":spool.stats()}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn readiness_waits_for_completed_initial_recovery_and_no_work() {
        let app = Application::new("readiness".into()).unwrap();
        let mut status = app.status();
        assert!(!initial_ready(&status));
        assert!(!settled(true, || panic!(
            "pending work must not inspect a result"
        )));
        assert!(!settled(false, || false));
        assert!(settled(false, || true));
        status.recovery.completed = 1;
        assert!(initial_ready(&status));
        status.pending = true;
        assert!(!initial_ready(&status));
        status.pending = false;
        status.recovery.running = true;
        assert!(!initial_ready(&status));
        assert!(wait(&app, Duration::ZERO, || false).is_err());
        wait(&app, Duration::ZERO, || true).unwrap();
        let ticket = app.request();
        app.fail(ticket, "fixture".into());
        assert!(
            wait(&app, Duration::ZERO, || true)
                .unwrap_err()
                .to_string()
                .contains("fixture")
        );
    }
    #[test]
    fn generated_declarations_are_valid_in_every_supported_probe_language() {
        let languages = Languages::builtin().unwrap();
        for language in ["rust", "python", "ruby", "c", "cpp"] {
            let (bytes, name) = edit(language, b"\n", "probe", 7);
            let fresh = languages
                .select(language)
                .unwrap()
                .create()
                .unwrap()
                .analyze(&bytes)
                .unwrap();
            assert!(fresh.facts.iter().any(|fact| fact.name == name));
            assert_eq!(fresh.coverage, vizzi_core::model::Coverage::Syntax);
        }
    }
    #[test]
    fn protocol_checks_reject_missing_marks_bad_order_and_incorrect_tail() {
        let now = Instant::now();
        let later = now + Duration::from_millis(1);
        assert_eq!(capture_mark(&[(now, now)], 1).unwrap(), (now, now));
        assert!(capture_mark(&[], 1).is_err());
        assert!(capture_mark(&[(now, later), (now, later)], 1).is_err());
        assert!(capture_mark(&[(now, later)], 0).is_err());
        assert!(capture_mark(&[(now, later)], 2).is_err());
        assert!(capture_mark(&[(later, now)], 1).is_err());
        let tail = CapturedSource::capture(b"z".to_vec(), None);
        validate_tail(&tail, 1).unwrap();
        assert!(validate_tail(&tail, 2).is_err());
        assert!(validate_tail(&CapturedSource::capture(b"x".to_vec(), None), 1).is_err());
        assert!(validate_tail(&CapturedSource::capture(vec![], None), 0).is_err());
    }
    #[test]
    fn publication_checks_reject_wrong_source_facts_declaration_and_old_pin() {
        let root = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let bytes = b"fn marker() {}\n";
        std::fs::write(root.path().join("a.rs"), bytes).unwrap();
        let snapshot = Engine::new(
            Directory::open(root.path()).unwrap(),
            Cache::new(cache.path().into()),
            1,
        )
        .unwrap()
        .refresh(None, &|| false)
        .unwrap()
        .unwrap()
        .snapshot;
        let doc = &snapshot.documents()[0];
        let analysis = &doc.analyses()[0];
        let old = CapturedSource::capture(b"old".to_vec(), None);
        verify_publication(doc, bytes, analysis, "marker", &old, b"old").unwrap();
        assert!(verify_publication(doc, b"wrong", analysis, "marker", &old, b"old").is_err());
        let mut other = analysis.clone();
        other.facts.clear();
        assert!(verify_publication(doc, bytes, &other, "marker", &old, b"old").is_err());
        assert!(verify_publication(doc, bytes, analysis, "absent", &old, b"old").is_err());
        assert!(verify_publication(doc, bytes, analysis, "marker", &old, b"wrong").is_err());
    }
}
