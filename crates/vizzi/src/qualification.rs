//! Controlled retained-snapshot qualification through public native interfaces.

use crate::{application::Application, cache::Cache, engine::Engine, source::Directory};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use vizzi_core::{
    analysis::{Analyzer, LanguageAdapter, Languages},
    c_family::{CAdapter, CppAdapter},
    model::{Analysis, Context, ContextId, ModelError, SnapshotId},
    policy::PolicyConfig,
    python::PythonAdapter,
    query::{View, ViewQuery},
    ruby::RubyAdapter,
    rust::RustAdapter,
};

type Error = Box<dyn std::error::Error>;

#[derive(Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum Mode {
    Reconcile,
    Reparse,
    Cancel,
}

#[derive(Deserialize, Serialize)]
struct Config {
    root: PathBuf,
    cache: PathBuf,
    #[serde(default)]
    phase_cache: Option<PathBuf>,
    workers: usize,
    policy: PolicyConfig,
    mode: Mode,
    context_revision: String,
    query_file: String,
    cancel_after_ms: u64,
    query_interval_ms: u64,
}

struct Factory {
    inner: Arc<dyn LanguageAdapter>,
    revision: String,
    clock: Instant,
    first_parse: Arc<AtomicU64>,
    immediate_cancel: Option<Arc<Application>>,
    cancel_requested: Arc<AtomicBool>,
    cancel_at: Arc<AtomicU64>,
}
impl LanguageAdapter for Factory {
    fn context(&self) -> Context {
        let mut context = self.inner.context();
        if !self.revision.is_empty() {
            context.id = ContextId::derive(&[
                context.id.as_str().as_bytes(),
                b"qualification-context",
                self.revision.as_bytes(),
            ]);
            context
                .description
                .push_str("; qualification context revision");
        }
        context
    }
    fn create(&self) -> Result<Box<dyn Analyzer>, ModelError> {
        Ok(Box::new(Parser {
            inner: self.inner.create()?,
            context: self.context(),
            clock: self.clock,
            first_parse: self.first_parse.clone(),
            immediate_cancel: self.immediate_cancel.clone(),
            cancel_requested: self.cancel_requested.clone(),
            cancel_at: self.cancel_at.clone(),
        }))
    }
}
struct Parser {
    inner: Box<dyn Analyzer>,
    context: Context,
    clock: Instant,
    first_parse: Arc<AtomicU64>,
    immediate_cancel: Option<Arc<Application>>,
    cancel_requested: Arc<AtomicBool>,
    cancel_at: Arc<AtomicU64>,
}
impl Parser {
    fn begin(&self) {
        let elapsed = self.clock.elapsed().as_micros() as u64;
        if self
            .first_parse
            .compare_exchange(u64::MAX, elapsed, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
            && let Some(app) = &self.immediate_cancel
        {
            self.cancel_at
                .store(self.clock.elapsed().as_micros() as u64, Ordering::Release);
            app.request();
            self.cancel_requested.store(true, Ordering::Release);
        }
    }
}
impl Analyzer for Parser {
    fn analyze(&mut self, bytes: &[u8]) -> Result<Analysis, ModelError> {
        self.begin();
        let mut analysis = self.inner.analyze(bytes)?;
        analysis.context = self.context.clone();
        Ok(analysis)
    }
    fn analyze_cancellable(
        &mut self,
        bytes: &[u8],
        cancelled: &dyn Fn() -> bool,
    ) -> Result<Option<Analysis>, ModelError> {
        self.begin();
        Ok(self
            .inner
            .analyze_cancellable(bytes, cancelled)?
            .map(|mut analysis| {
                analysis.context = self.context.clone();
                analysis
            }))
    }
}

fn qualify(config: Config, emit: &mut impl FnMut(Value)) -> Result<Value, Error> {
    if !(1..=1000).contains(&config.query_interval_ms) {
        return Err("query interval must be in 1..=1000 ms".into());
    }
    if !matches!(config.mode, Mode::Reconcile) && config.context_revision.is_empty() {
        return Err("reparse/cancel needs a fresh context revision".into());
    }
    let app = Arc::new(Application::new("qualification".into())?);
    let baseline_engine = Engine::new(
        Directory::with_policy(
            &config.root,
            config.policy.clone(),
            Arc::new(Languages::builtin()?),
        )?,
        Cache::new(config.cache.clone()),
        config.workers,
    )?;
    emit(json!({"event":"baseline_start"}));
    let ticket = app.request();
    let baseline = baseline_engine
        .refresh(None, &|| false)?
        .ok_or("baseline unexpectedly cancelled")?;
    let old = baseline.snapshot.clone();
    app.publish(ticket, baseline);
    let baseline_status = app.status();
    emit(json!({"event":"baseline_ready", "status":baseline_status}));
    let selected = old
        .documents()
        .iter()
        .find(|file| file.path() == config.query_file)
        .ok_or("query file is outside capture scope")?;
    let query = ViewQuery {
        file: Some(selected.id().as_str().into()),
        ..Default::default()
    };
    let first_parse = Arc::new(AtomicU64::new(u64::MAX));
    let cancel_requested = Arc::new(AtomicBool::new(false));
    let cancel_at = Arc::new(AtomicU64::new(u64::MAX));
    let clock = Instant::now();
    let immediate = matches!(config.mode, Mode::Cancel) && config.cancel_after_ms == 0;
    let revision = if matches!(config.mode, Mode::Reconcile) {
        ""
    } else {
        &config.context_revision
    };
    let inner: Vec<Arc<dyn LanguageAdapter>> = vec![
        Arc::new(RustAdapter),
        Arc::new(PythonAdapter),
        Arc::new(RubyAdapter),
        Arc::new(CAdapter),
        Arc::new(CppAdapter),
    ];
    let factories = inner
        .into_iter()
        .map(|inner| {
            Arc::new(Factory {
                inner,
                revision: revision.into(),
                clock,
                first_parse: first_parse.clone(),
                immediate_cancel: immediate.then(|| app.clone()),
                cancel_requested: cancel_requested.clone(),
                cancel_at: cancel_at.clone(),
            }) as Arc<dyn LanguageAdapter>
        })
        .collect();
    let engine = Engine::new(
        Directory::with_policy(
            &config.root,
            config.policy,
            Arc::new(Languages::new(factories)?),
        )?,
        Cache::new(config.phase_cache.unwrap_or(config.cache)),
        config.workers,
    )?;
    let ticket = app.request();
    emit(json!({"event":"phase_start", "mode":config.mode, "baseline_snapshot":old.id()}));
    let mut queries = Vec::new();
    let phase = std::thread::scope(|scope| -> Result<Value, Error> {
        let worker_app = app.clone();
        let previous = old.clone();
        let worker = scope.spawn(move || -> Result<Value, crate::engine::EngineError> {
            let refresh = engine.refresh(Some(&previous), &|| worker_app.desired() != ticket)?;
            let returned_us = clock.elapsed().as_micros() as u64;
            Ok(match refresh {
                Some(refresh) => {
                    let measurements = refresh.measurements.clone();
                    let outcome = worker_app.publish(ticket, refresh);
                    json!({"refresh":measurements,"outcome":format!("{outcome:?}"),"worker_return_us":returned_us})
                }
                None => json!({"refresh":null,"outcome":"cancelled","worker_return_us":returned_us}),
            })
        });
        let mut parse_seen = false;
        loop {
            let started = parse_time(first_parse.load(Ordering::Acquire));
            emit_parse_start(started, &mut parse_seen, emit);
            if cancel_due(
                config.mode,
                started,
                clock.elapsed(),
                Duration::from_millis(config.cancel_after_ms),
                cancel_requested.load(Ordering::Acquire),
            ) {
                cancel_at.store(clock.elapsed().as_micros() as u64, Ordering::Release);
                app.request();
                cancel_requested.store(true, Ordering::Release);
                emit(json!({"event":"cancel_requested"}));
            }
            let pending = app.status().pending;
            let snapshot = app.snapshot().ok_or("published snapshot disappeared")?;
            let at = Instant::now();
            let view = snapshot.view(&query)?;
            let encoded = serde_json::to_vec(&view)?;
            queries.push(json!({"ms":millis(at.elapsed()), "encoded_bytes":encoded.len(), "pending":pending}));
            validate_query(
                &view,
                snapshot.id(),
                &config.query_file,
                selected.resident_source(),
            )?;
            if worker.is_finished() {
                break;
            }
            std::thread::sleep(Duration::from_millis(config.query_interval_ms));
        }
        let completed = worker.join().map_err(|_| "qualification worker panicked")?;
        emit_parse_start(
            parse_time(first_parse.load(Ordering::Acquire)),
            &mut parse_seen,
            emit,
        );
        completed.map_err(Into::into)
    })?;
    let pinned = app
        .pinned(old.id().as_str())
        .ok_or("previous snapshot was not retained")?;
    let pinned_matches = pinned
        .file(selected.id().as_str())
        .is_some_and(|file| file.resident_source() == selected.resident_source());
    if !pinned_matches {
        return Err("pinned source changed".into());
    }
    let requested = parse_time(cancel_at.load(Ordering::Acquire));
    let returned = phase["worker_return_us"]
        .as_u64()
        .map(Duration::from_micros);
    let report = json!({"mode":config.mode,"baseline":baseline_status,"final":app.status(),"refresh":phase["refresh"],"outcome":phase["outcome"],
        "phase_wall_ms":millis(clock.elapsed()),"queries":queries,"query_file":selected.path(),
        "cancel_requested":cancel_requested.load(Ordering::Acquire),"first_parse_us":parse_time(first_parse.load(Ordering::Acquire)).map(|duration| duration.as_micros()),"pinned_source_matches":pinned_matches,
        "cancel_requested_us":requested.map(|duration| duration.as_micros()),"worker_return_us":phase["worker_return_us"],"cancel_to_return_ms":cancellation_latency(requested, returned)});
    emit(json!({"event":"phase_done", "outcome":phase["outcome"]}));
    Ok(report)
}

fn emit_parse_start(started: Option<Duration>, seen: &mut bool, emit: &mut impl FnMut(Value)) {
    if let Some(started) = started
        && !*seen
    {
        emit(json!({"event":"parse_started","elapsed_us":started.as_micros()}));
        *seen = true;
    }
}

fn millis(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}
fn parse_time(value: u64) -> Option<Duration> {
    (value != u64::MAX).then(|| Duration::from_micros(value))
}
fn cancellation_latency(requested: Option<Duration>, returned: Option<Duration>) -> Option<f64> {
    requested
        .zip(returned)
        .and_then(|(start, end)| end.checked_sub(start))
        .map(millis)
}
fn cancel_due(
    mode: Mode,
    started: Option<Duration>,
    elapsed: Duration,
    delay: Duration,
    requested: bool,
) -> bool {
    matches!(mode, Mode::Cancel)
        && !delay.is_zero()
        && !requested
        && started.is_some_and(|started| elapsed.saturating_sub(started) >= delay)
}
fn validate_query(
    view: &View,
    snapshot: &SnapshotId,
    path: &str,
    text: Option<&str>,
) -> Result<(), Error> {
    if view.snapshot != *snapshot {
        return Err("query mixed snapshot identity".into());
    }
    let source = view.source.as_ref().ok_or("query lost selected source")?;
    if source.path != path {
        return Err("query selected a different file".into());
    }
    if source.text.as_deref() != text {
        return Err("query mixed captured source".into());
    }
    Ok(())
}

/// Runs controlled retained-snapshot qualification from a configuration JSON document.
/// This optional experiment module uses public native interfaces and performs no source edits.
pub fn run(configuration: &[u8], emit: &mut impl FnMut(Value)) -> Result<Value, Error> {
    qualify(serde_json::from_slice(configuration)?, emit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancellation_latency_excludes_poll_cadence_and_requires_ordered_timestamps() {
        assert_eq!(cancellation_latency(None, Some(Duration::ZERO)), None);
        assert_eq!(cancellation_latency(Some(Duration::ZERO), None), None);
        assert_eq!(
            cancellation_latency(
                Some(Duration::from_millis(2)),
                Some(Duration::from_millis(1))
            ),
            None
        );
        assert_eq!(
            cancellation_latency(
                Some(Duration::from_millis(2)),
                Some(Duration::from_millis(2))
            ),
            Some(0.0)
        );
        assert_eq!(
            cancellation_latency(
                Some(Duration::from_micros(1000)),
                Some(Duration::from_micros(2500))
            ),
            Some(1.5)
        );
    }

    #[test]
    fn qualification_wrapper_forwards_interruption_instead_of_completing_the_input() {
        struct Interrupted;
        impl Analyzer for Interrupted {
            fn analyze(&mut self, _: &[u8]) -> Result<Analysis, ModelError> {
                panic!("wrapper lost cancellation");
            }
            fn analyze_cancellable(
                &mut self,
                _: &[u8],
                _: &dyn Fn() -> bool,
            ) -> Result<Option<Analysis>, ModelError> {
                Ok(None)
            }
        }
        let mut parser = Parser {
            inner: Box::new(Interrupted),
            context: RustAdapter.context(),
            clock: Instant::now(),
            first_parse: Arc::new(AtomicU64::new(u64::MAX)),
            immediate_cancel: None,
            cancel_requested: Arc::new(AtomicBool::new(false)),
            cancel_at: Arc::new(AtomicU64::new(u64::MAX)),
        };
        assert!(
            parser
                .analyze_cancellable(b"", &|| false)
                .unwrap()
                .is_none()
        );
        parser.inner = RustAdapter.create().unwrap();
        parser.context.description = "rebound fixture context".into();
        let complete = parser.analyze(b"fn example() {}").unwrap();
        assert_eq!(complete.context, parser.context);
        assert_eq!(
            parser
                .analyze_cancellable(b"fn example() {}", &|| false)
                .unwrap()
                .unwrap(),
            complete
        );
    }

    #[test]
    fn timers_preserve_units_unstarted_state_and_cancel_boundaries() {
        assert_eq!(millis(Duration::ZERO), 0.0);
        assert_eq!(millis(Duration::from_micros(1500)), 1.5);
        assert_eq!(millis(Duration::from_secs(2)), 2000.0);
        assert_eq!(parse_time(u64::MAX), None);
        assert_eq!(parse_time(0), Some(Duration::ZERO));
        assert_eq!(parse_time(1500), Some(Duration::from_micros(1500)));
        let start = Some(Duration::from_millis(50));
        let delay = Duration::from_millis(100);
        assert!(!cancel_due(
            Mode::Cancel,
            None,
            Duration::from_secs(1),
            delay,
            false
        ));
        for elapsed in [0, 49, 50, 149] {
            assert!(!cancel_due(
                Mode::Cancel,
                start,
                Duration::from_millis(elapsed),
                delay,
                false
            ));
        }
        for elapsed in [150, 151, 1000] {
            assert!(cancel_due(
                Mode::Cancel,
                start,
                Duration::from_millis(elapsed),
                delay,
                false
            ));
        }
        assert!(!cancel_due(
            Mode::Cancel,
            start,
            Duration::from_secs(1),
            delay,
            true
        ));
        assert!(!cancel_due(
            Mode::Cancel,
            start,
            Duration::from_secs(1),
            Duration::ZERO,
            false
        ));
        for mode in [Mode::Reconcile, Mode::Reparse] {
            assert!(!cancel_due(
                mode,
                start,
                Duration::from_secs(1),
                delay,
                false
            ));
        }
    }

    #[test]
    fn invalid_probe_configuration_and_inconsistent_queries_are_rejected() {
        let root = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("a.rs"), "fn original() {}").unwrap();
        let mut config = Config {
            root: root.path().into(),
            cache: cache.path().into(),
            phase_cache: None,
            workers: 1,
            policy: PolicyConfig::default(),
            mode: Mode::Reparse,
            context_revision: String::new(),
            query_file: "a.rs".into(),
            cancel_after_ms: 0,
            query_interval_ms: 1,
        };
        for mode in [Mode::Reparse, Mode::Cancel] {
            config.mode = mode;
            assert!(run(&serde_json::to_vec(&config).unwrap(), &mut |_| {}).is_err());
        }
        config.mode = Mode::Reconcile;
        for interval in [0, 1001] {
            config.query_interval_ms = interval;
            assert!(run(&serde_json::to_vec(&config).unwrap(), &mut |_| {}).is_err());
        }
        let engine = Engine::new(
            Directory::open(root.path()).unwrap(),
            Cache::new(cache.path().into()),
            1,
        )
        .unwrap();
        let snapshot = engine.refresh(None, &|| false).unwrap().unwrap().snapshot;
        let view = || snapshot.view(&ViewQuery::default()).unwrap();
        assert!(validate_query(&view(), snapshot.id(), "a.rs", Some("fn original() {}")).is_ok());
        assert!(
            validate_query(
                &view(),
                &SnapshotId::derive(&[b"other"]),
                "a.rs",
                Some("fn original() {}")
            )
            .is_err()
        );
        assert!(validate_query(&view(), snapshot.id(), "b.rs", Some("fn original() {}")).is_err());
        assert!(validate_query(&view(), snapshot.id(), "a.rs", Some("changed")).is_err());
        let mut missing = view();
        missing.source = None;
        assert!(validate_query(&missing, snapshot.id(), "a.rs", Some("fn original() {}")).is_err());
    }

    #[test]
    fn reconciliation_reuses_data_and_context_reparse_retains_pinned_source() {
        for mode in [Mode::Reconcile, Mode::Reparse, Mode::Cancel] {
            let root = tempfile::tempdir().unwrap();
            let cache = tempfile::tempdir().unwrap();
            std::fs::write(root.path().join("a.rs"), "fn rust() {}").unwrap();
            std::fs::write(root.path().join("b.c"), "int c(void) { return 0; }").unwrap();
            let config = Config {
                root: root.path().into(),
                cache: cache.path().into(),
                phase_cache: None,
                workers: 2,
                policy: PolicyConfig {
                    c_family_roots: vec![".".into()],
                    ..Default::default()
                },
                mode,
                context_revision: "fixture-new-context".into(),
                query_file: "b.c".into(),
                cancel_after_ms: 0,
                query_interval_ms: 1,
            };
            let mut events = Vec::new();
            let report = run(&serde_json::to_vec(&config).unwrap(), &mut |event| {
                events.push(event)
            })
            .unwrap();
            assert_eq!(report["query_file"], "b.c");
            assert!(!report["queries"].as_array().unwrap().is_empty());
            assert_eq!(report["baseline"]["measurements"]["files"], 2);
            assert_eq!(report["pinned_source_matches"], true);
            assert!(events.iter().any(|event| event["event"] == "phase_start"));
            match mode {
                Mode::Reconcile => {
                    assert!(report["cancel_requested_us"].is_null());
                    assert!(report["cancel_to_return_ms"].is_null());
                    assert!(report["worker_return_us"].as_u64().is_some());
                    assert_eq!(report["refresh"]["parsed"], 0);
                    assert_eq!(report["refresh"]["reused"], 2);
                    assert_eq!(report["final"]["generation"], 1);
                    assert!(!events.iter().any(|event| event["event"] == "parse_started"));
                    assert_eq!(report["final"]["pending"], false);
                }
                Mode::Reparse => {
                    assert!(report["cancel_to_return_ms"].is_null());
                    assert_eq!(report["refresh"]["parsed"], 2);
                    assert_eq!(report["refresh"]["cache_hits"], 0);
                    assert_eq!(report["final"]["generation"], 2);
                    assert_eq!(report["final"]["pending"], false);
                    assert_eq!(
                        events
                            .iter()
                            .filter(|event| event["event"] == "parse_started")
                            .count(),
                        1
                    );
                }
                Mode::Cancel => {
                    assert!(report["cancel_to_return_ms"].as_f64().unwrap() >= 0.0);
                    assert!(
                        report["first_parse_us"].as_u64().unwrap()
                            <= report["cancel_requested_us"].as_u64().unwrap()
                    );
                    assert!(
                        report["cancel_requested_us"].as_u64().unwrap()
                            <= report["worker_return_us"].as_u64().unwrap()
                    );
                    assert!(report["refresh"].is_null());
                    assert_eq!(report["final"]["generation"], 1);
                    assert_eq!(report["cancel_requested"], true);
                }
            }
        }
    }
    #[test]
    fn a_start_recorded_between_the_last_poll_and_join_is_emitted_once() {
        let mut seen = false;
        let mut events = Vec::new();
        emit_parse_start(None, &mut seen, &mut |event| events.push(event));
        assert!(events.is_empty());
        emit_parse_start(Some(Duration::from_micros(17)), &mut seen, &mut |event| {
            events.push(event)
        });
        emit_parse_start(Some(Duration::from_micros(17)), &mut seen, &mut |event| {
            events.push(event)
        });
        assert_eq!(
            events,
            vec![json!({"event":"parse_started","elapsed_us":17})]
        );
        assert!(seen);
    }
}
