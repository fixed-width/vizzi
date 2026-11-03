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
use serde_json::{Value, json};
use std::{
    io::Read,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use vizzi_core::{analysis::Languages, model::ContentId};
type Error = Box<dyn std::error::Error>;
#[derive(Default)]
struct Observations {
    entered: bool,
    span: Option<ReclaimSpan>,
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
/// Measures explicit-hint foreground publication around host-scheduled real cleanup.
/// Only marked owned source is edited; pressure is generated outside project code.
pub fn run(configuration: &[u8], emit: &mut impl FnMut(Value)) -> Result<Value, Error> {
    let config: Config = serde_json::from_slice(configuration)?;
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
            watch: false,
            reconcile_interval: Duration::from_secs(3600),
            idle_maintenance: true,
        },
        Some(observed.clone()),
    )?;
    let app = host.application();
    wait(&app, Duration::from_secs(120), || {
        initial_ready(&app.status())
    })?;
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
            spool.trace_capture(Some(trace.clone()));
            let clock = Instant::now();
            let retired = Instant::now();
            drop(tail);
            wait(&app, Duration::from_secs(120), || {
                observed
                    .observations
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .entered
                    || host.maintenance_status().failed > before.failed
            })?;
            if host.maintenance_status().failed > before.failed {
                return Err("scheduled cleanup failed".into());
            }
            if !overlap {
                wait(&app, Duration::from_secs(120), || {
                    observed
                        .observations
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner)
                        .span
                        .is_some()
                })?;
            }
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
            let mark = capture_mark(
                &marks,
                status
                    .measurements
                    .as_ref()
                    .ok_or("missing measurements")?
                    .read_files,
            )?;
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
                "capture_marks":[{"entered_ms":millis(clock,mark.0),"ready_ms":millis(clock,mark.1),"gate_ms":(mark.1-mark.0).as_secs_f64()*1000.0}],
                "write_overlaps_cleanup":write_started>=span.started&&write_started<span.finished,
                "capture_overlaps_cleanup":mark.0>=span.started&&mark.0<span.finished,"published_during_cleanup":published<span.finished,
                "maintenance_before":before,"maintenance_observed":host.maintenance_status(),"measurements":status.measurements,
                "source_matches":true,"fresh_facts_match":true,"expected_declaration":true,"old_pin_matches":true}));
            drop(observations);
            emit(json!({"event":"phase_done","sample":sample,"condition":condition}));
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
        .ok_or("clean cancelled")?;
    if clean.snapshot.id() != &baseline_id {
        return Err("clean observation differs".into());
    }
    let files = baseline.documents().len();
    let maintenance = host.maintenance_status();
    let started = Instant::now();
    drop(host);
    let shutdown_ms = started.elapsed().as_secs_f64() * 1000.0;
    drop(clean);
    drop(baseline);
    drop(app);
    spool.reclaim()?;
    Ok(
        json!({"files":files,"cases":cases,"maintenance":maintenance,"shutdown_ms":shutdown_ms,"baseline_snapshot":baseline_id,
        "restored_matches":true,"final_clean_matches":true,"spool":spool.stats()}),
    )
}
