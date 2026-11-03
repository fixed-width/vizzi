//! Optional automatic idle-cleanup measurements through the native host scheduler.
use crate::{
    foreground_maintenance::{
        Config, budgets, capture_mark, edit, engine, initial_ready, millis, settled, validate_tail,
        verify_publication, wait,
    },
    host::{Host, HostOptions},
    maintenance::SourceMaintenance,
    source::CaptureScope,
    source_spool::{CaptureTrace, ReclaimSpan, SourceSpool},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    io::Read,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use vizzi_core::{
    analysis::Languages,
    model::{ContentId, SnapshotId},
};
type Error = Box<dyn std::error::Error>;
#[derive(Deserialize)]
struct Mode {
    #[serde(default)]
    watch: bool,
    #[serde(default = "default_recovery")]
    recovery_ms: u64,
    #[serde(default)]
    recovery_samples: usize,
}
fn default_recovery() -> u64 {
    3_600_000
}
impl Mode {
    fn validate(&self) -> Result<(), Error> {
        if self.recovery_samples > 8 {
            return Err("at most eight recovery samples".into());
        }
        Ok(())
    }
    fn combined(&self) -> bool {
        self.watch || self.recovery_ms != default_recovery() || self.recovery_samples != 0
    }
}
fn require_native(observation: &str) -> Result<(), Error> {
    if observation != "native directory watches + periodic reconciliation" {
        return Err(format!("native watching unavailable: {observation}").into());
    }
    Ok(())
}
fn hint_scope(watch: bool, target: &str) -> Option<CaptureScope> {
    if watch {
        None
    } else {
        Some(CaptureScope::Paths(
            [PathBuf::from(target)].into_iter().collect(),
        ))
    }
}
fn submit(host: &Host, watch: bool, target: &str) {
    if let Some(scope) = hint_scope(watch, target) {
        host.request(scope);
    }
}
fn failed_since(before: u64, observed: u64) -> bool {
    observed > before
}
fn restored(pending: bool, actual: Option<&SnapshotId>, expected: &SnapshotId) -> bool {
    settled(pending, || actual == Some(expected))
}
#[derive(Default)]
struct Observations {
    entered: bool,
    span: Option<ReclaimSpan>,
}
impl Observations {
    fn ready(&self, overlap: bool) -> bool {
        if overlap {
            self.entered
        } else {
            self.span.is_some()
        }
    }
}
struct Observed {
    spool: Arc<SourceSpool>,
    minimum: u64,
    observations: Mutex<Observations>,
}
impl SourceMaintenance for Observed {
    fn pending(&self) -> bool {
        SourceMaintenance::pending(&*self.spool)
    }
    fn run(&self) -> Result<(), String> {
        let span = self
            .spool
            .observe_reclaim_minimum(self.minimum, |_| {
                self.observations
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .entered = true;
            })
            .map_err(|error| error.to_string())?;
        if let Some(span) = span {
            self.observations
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .span = Some(span);
        }
        Ok(())
    }
}
/// Measures foreground publication around host-scheduled cleanup and optional recovery.
/// Only marked owned source is edited; pressure is generated outside project code.
pub fn run(configuration: &[u8], emit: &mut impl FnMut(Value)) -> Result<Value, Error> {
    let config: Config = serde_json::from_slice(configuration)?;
    let mode: Mode = serde_json::from_slice(configuration)?;
    mode.validate()?;
    let (capacity, tail_bytes) = budgets(&config)?;
    let spool = Arc::new(SourceSpool::new(
        &config.root.parent().ok_or("missing parent")?.join("spool"),
        capacity,
    )?);
    let observed = Arc::new(Observed {
        spool: spool.clone(),
        minimum: tail_bytes as u64,
        observations: Mutex::new(Observations::default()),
    });
    emit(json!({"event":"baseline_start"}));
    let host = Host::start_with_maintenance(
        engine(&config, Some(spool.clone()))?,
        HostOptions {
            watch: mode.watch,
            reconcile_interval: Duration::from_millis(mode.recovery_ms),
            idle_maintenance: true,
        },
        Some(observed.clone()),
    )?;
    let app = host.application();
    wait(&app, Duration::from_secs(120), || {
        initial_ready(&app.status())
    })?;
    if mode.watch {
        require_native(&app.status().observation)?;
    }
    let baseline = app.snapshot().ok_or("missing baseline")?;
    let baseline_id = baseline.id().clone();
    let file = baseline
        .file_by_path(&config.target)
        .ok_or("target missing")?;
    if file.analyses().len() != 1 {
        return Err("target needs one analyser".into());
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
            wait(&app, Duration::from_secs(120), || {
                !host.maintenance_status().running
            })?;
            *observed
                .observations
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = Observations::default();
            let before = host.maintenance_status();
            let trace = Arc::new(CaptureTrace::default());
            if !mode.combined() {
                spool.trace_capture(Some(trace.clone()));
            }
            let clock = Instant::now();
            let retired = Instant::now();
            drop(tail);
            wait(&app, Duration::from_secs(120), || {
                observed
                    .observations
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .ready(overlap)
                    || failed_since(before.failed, host.maintenance_status().failed)
            })?;
            if failed_since(before.failed, host.maintenance_status().failed) {
                return Err("scheduled cleanup failed".into());
            }
            let (bytes, name) = edit(&language, &original, &config.probe, sample);
            let expected = ContentId::derive(&[&bytes]);
            emit(json!({"event":"phase_start","sample":sample,"condition":condition}));
            let status_before = app.status();
            let write_started = Instant::now();
            std::fs::write(&path, &bytes)?;
            let hinted = Instant::now();
            submit(&host, mode.watch, &config.target);
            wait(&app, Duration::from_secs(120), || {
                settled(app.status().pending, || {
                    app.snapshot().is_some_and(|s| {
                        s.file_by_path(&config.target)
                            .is_some_and(|f| f.content() == &expected)
                    })
                })
            })?;
            let published = Instant::now();
            let visible = app.snapshot().ok_or("missing publication")?;
            let status = app.status();
            spool.trace_capture(None);
            wait(&app, Duration::from_secs(120), || {
                observed
                    .observations
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .span
                    .is_some()
            })?;
            let observations = observed
                .observations
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let span = observations.span.as_ref().ok_or("missing real cut")?;
            let marks = trace
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .clone();
            let mark = if mode.combined() {
                None
            } else {
                Some(capture_mark(
                    &marks,
                    status
                        .measurements
                        .as_ref()
                        .ok_or("missing measurements")?
                        .read_files,
                )?)
            };
            let source = visible
                .file_by_path(&config.target)
                .ok_or("source missing")?;
            let fresh = Languages::builtin()?
                .select(&language)
                .ok_or("analyser unavailable")?
                .create()?
                .analyze(&bytes)?;
            verify_publication(source, &bytes, &fresh, &name, file.captured(), &original)?;
            cases.push(json!({"pair":pair,"sample":sample,"condition":condition,"tail_bytes":tail_bytes,
                "retired_ms":millis(clock,retired),"cleanup_start_ms":millis(clock,span.started),"cleanup_end_ms":millis(clock,span.finished),
                "cleanup_ms":(span.finished-span.started).as_secs_f64()*1000.0,"admission_ms":(span.started-retired).as_secs_f64()*1000.0,
                "write_start_ms":millis(clock,write_started),"hint_ms":millis(clock,hinted),"published_ms":millis(clock,published),
                "write_to_published_ms":(published-write_started).as_secs_f64()*1000.0,
                "capture_marks":mark.map(|mark|vec![json!({"entered_ms":millis(clock,mark.0),"ready_ms":millis(clock,mark.1),"gate_ms":(mark.1-mark.0).as_secs_f64()*1000.0})]).unwrap_or_default(),
                "write_overlaps_cleanup":write_started>=span.started&&write_started<span.finished,
                "capture_overlaps_cleanup":mark.map(|m|m.0>=span.started&&m.0<span.finished),"published_during_cleanup":published<span.finished,
                "status_before":status_before,"status_after":status,"explicit_hint":!mode.watch,
                "maintenance_before":before,"maintenance_observed":host.maintenance_status(),"measurements":status.measurements,
                "source_matches":true,"fresh_facts_match":true,"expected_declaration":true,"old_pin_matches":true}));
            drop(observations);
            emit(json!({"event":"phase_done","sample":sample,"condition":condition}));
            std::fs::write(&path, &original)?;
            submit(&host, mode.watch, &config.target);
            wait(&app, Duration::from_secs(120), || {
                restored(
                    app.status().pending,
                    app.snapshot().as_ref().map(|s| s.id()),
                    &baseline_id,
                )
            })?;
        }
    }
    for _ in 0..mode.recovery_samples {
        let mut status_before = app.status();
        wait(&app, Duration::from_secs(120), || {
            status_before = app.status();
            status_before.recovery.running && !status_before.pending
        })?;
        let sample = cases.len();
        let (bytes, name) = edit(&language, &original, &config.probe, sample);
        let expected = ContentId::derive(&[&bytes]);
        let clock = Instant::now();
        emit(json!({"event":"phase_start","sample":sample,"condition":"recovery_observed"}));
        let write_started = Instant::now();
        std::fs::write(&path, &bytes)?;
        let write_finished = Instant::now();
        submit(&host, mode.watch, &config.target);
        wait(&app, Duration::from_secs(120), || {
            settled(app.status().pending, || {
                app.snapshot().is_some_and(|s| {
                    s.file_by_path(&config.target)
                        .is_some_and(|f| f.content() == &expected)
                })
            })
        })?;
        let published = Instant::now();
        let visible = app.snapshot().ok_or("missing recovery-write publication")?;
        let status_after = app.status();
        let source = visible
            .file_by_path(&config.target)
            .ok_or("source missing")?;
        let fresh = Languages::builtin()?
            .select(&language)
            .ok_or("analyser unavailable")?
            .create()?
            .analyze(&bytes)?;
        verify_publication(source, &bytes, &fresh, &name, file.captured(), &original)?;
        cases.push(json!({"sample":sample,"condition":"recovery_observed","status_before":status_before,"status_after":status_after,
            "write_start_ms":millis(clock,write_started),"write_finished_ms":millis(clock,write_finished),"published_ms":millis(clock,published),
            "write_to_published_ms":(published-write_started).as_secs_f64()*1000.0,"explicit_hint":!mode.watch,
            "maintenance_observed":host.maintenance_status(),"source_matches":true,"fresh_facts_match":true,"expected_declaration":true,"old_pin_matches":true}));
        emit(json!({"event":"phase_done","sample":sample,"condition":"recovery_observed"}));
        std::fs::write(&path, &original)?;
        submit(&host, mode.watch, &config.target);
        wait(&app, Duration::from_secs(120), || {
            restored(
                app.status().pending,
                app.snapshot().as_ref().map(|s| s.id()),
                &baseline_id,
            )
        })?;
    }
    let clean = engine(&config, None)?
        .refresh(None, &|| false)?
        .ok_or("clean cancelled")?;
    if clean.snapshot.id() != &baseline_id {
        return Err("clean observation differs".into());
    }
    let files = baseline.documents().len();
    let maintenance = host.maintenance_status();
    let started = Instant::now();
    drop(host);
    let shutdown_ms = millis(started, Instant::now());
    // Native watcher teardown may release its callback after coordinator shutdown.
    wait(&app, Duration::from_secs(120), || {
        Arc::strong_count(&app) == 1
    })?;
    let released_ms = millis(started, Instant::now());
    drop(clean);
    drop(baseline);
    drop(app);
    spool.reclaim()?;
    Ok(
        json!({"files":files,"cases":cases,"watching":mode.watch,"explicit_hints":!mode.watch,"recovery_ms":mode.recovery_ms,"maintenance":maintenance,"shutdown_ms":shutdown_ms,"released_ms":released_ms,"baseline_snapshot":baseline_id,
        "restored_matches":true,"final_clean_matches":true,"spool":spool.stats()}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restoration_requires_the_baseline_and_completed_foreground_work() {
        let baseline = SnapshotId::derive(&[b"baseline"]);
        let changed = SnapshotId::derive(&[b"changed"]);
        assert!(!restored(false, None, &baseline));
        assert!(!restored(false, Some(&changed), &baseline));
        assert!(!restored(true, Some(&baseline), &baseline));
        assert!(restored(false, Some(&baseline), &baseline));
        let now = Instant::now();
        assert_eq!(millis(now, now + Duration::from_micros(1500)), 1.5);
    }
    #[test]
    fn control_waits_for_completion_while_overlap_waits_for_real_entry() {
        let mut observations = Observations::default();
        assert!(!observations.ready(false));
        assert!(!observations.ready(true));
        observations.entered = true;
        assert!(!observations.ready(false));
        assert!(observations.ready(true));
        let now = Instant::now();
        observations.span = Some(ReclaimSpan {
            started: now,
            finished: now,
            cut_bytes: 0,
        });
        assert!(observations.ready(false));
        assert!(observations.ready(true));
    }
    #[test]
    fn cleanup_failure_detection_distinguishes_existing_from_new_failures() {
        assert!(!failed_since(0, 0));
        assert!(!failed_since(5, 5));
        assert!(failed_since(0, 1));
        assert!(failed_since(5, 6));
    }
    #[test]
    fn combined_mode_and_native_requirement_do_not_claim_polling_as_watching() {
        let mut mode: Mode = serde_json::from_str("{}").unwrap();
        assert_eq!(mode.recovery_ms, 3_600_000);
        assert!(!mode.combined());
        assert!(hint_scope(true, "a.rs").is_none());
        assert_eq!(
            hint_scope(false, "a.rs"),
            Some(CaptureScope::Paths(
                [PathBuf::from("a.rs")].into_iter().collect()
            ))
        );
        mode.recovery_samples = 8;
        mode.validate().unwrap();
        mode.recovery_samples = 9;
        assert!(mode.validate().is_err());
        mode.recovery_samples = 0;
        mode.watch = true;
        assert!(mode.combined());
        mode.watch = false;
        mode.recovery_ms = 2000;
        assert!(mode.combined());
        mode.recovery_ms = default_recovery();
        mode.recovery_samples = 1;
        assert!(mode.combined());
        require_native("native directory watches + periodic reconciliation").unwrap();
        for value in [
            "polling; native watches disabled or unavailable",
            "periodic reconciliation; 1 directory watches unavailable",
            "",
        ] {
            assert!(require_native(value).is_err());
        }
    }
    #[test]
    fn observed_provider_reports_only_real_qualifying_cuts() {
        let owned = tempfile::tempdir().unwrap();
        let spool = Arc::new(SourceSpool::new(owned.path(), 1024).unwrap());
        let provider = Observed {
            spool: spool.clone(),
            minimum: 128,
            observations: Mutex::new(Observations::default()),
        };
        assert!(!provider.pending());
        provider.run().unwrap();
        assert!(!provider.observations.lock().unwrap().entered);
        let small = spool
            .capture(&mut std::io::repeat(b'x').take(16), None, &|| false)
            .unwrap()
            .unwrap();
        drop(small);
        provider.run().unwrap();
        assert_eq!(spool.stats().file_bytes, Some(0));
        assert!(!provider.observations.lock().unwrap().entered);
        assert!(provider.observations.lock().unwrap().span.is_none());
        let tail = spool
            .capture(&mut std::io::repeat(b'x').take(128), None, &|| false)
            .unwrap()
            .unwrap();
        assert!(!provider.pending());
        drop(tail);
        assert!(provider.pending());
        provider.run().unwrap();
        assert!(!provider.pending());
        let observations = provider.observations.lock().unwrap();
        assert!(observations.entered);
        let span = observations.span.as_ref().unwrap();
        assert_eq!(span.cut_bytes, 0);
        assert!(span.started <= span.finished);
        assert_eq!(spool.stats().file_bytes, Some(0));
    }
}
