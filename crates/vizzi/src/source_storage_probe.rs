//! Optional source-only capture/history ownership probe; no analyser or cache work.

mod maintenance;
use crate::source::{Capture, CaptureScope, Directory};
use maintenance::{Probe, READER_BYTES};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::BTreeSet, io::Write, path::PathBuf, sync::Arc, time::Instant};
use vizzi_core::{
    analysis::Languages,
    model::{CapturedSource, ContentId, Document, Snapshot, SnapshotId},
    policy::PolicyConfig,
    publication::{Outcome, Publication},
};

type Error = Box<dyn std::error::Error>;
#[derive(Deserialize)]
struct Config {
    root: PathBuf,
    policy: PolicyConfig,
    targets: Vec<String>,
    #[serde(default)]
    spool_mib: usize,
    #[serde(default)]
    maintenance_reads: bool,
    owner: String,
    probe: String,
    mode: Mode,
}
#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
enum Mode {
    Headers,
    Workspace,
}
#[derive(Default, Serialize)]
struct Inventory {
    unique_sources: usize,
    source_bytes: usize,
    byte_capacity: usize,
    index_entries: usize,
    index_bytes: usize,
    buffer_bytes: usize,
    integrity_bytes: usize,
}
fn inventory(snapshots: &[Arc<Snapshot>]) -> Result<Inventory, Error> {
    let mut seen = BTreeSet::new();
    let mut result = Inventory::default();
    for snapshot in snapshots {
        for document in snapshot.documents() {
            let source = document.captured();
            if !seen.insert(Arc::as_ptr(source)) {
                continue;
            }
            let storage = source.storage();
            result.source_bytes = result
                .source_bytes
                .checked_add(storage.source_bytes)
                .ok_or("source accounting overflow")?;
            result.byte_capacity = result
                .byte_capacity
                .checked_add(storage.byte_capacity)
                .ok_or("capacity accounting overflow")?;
            result.index_entries = result
                .index_entries
                .checked_add(storage.index_entries)
                .ok_or("entry accounting overflow")?;
            result.integrity_bytes = result
                .integrity_bytes
                .checked_add(storage.integrity_bytes)
                .ok_or("integrity accounting overflow")?;
            result.index_bytes = result
                .index_bytes
                .checked_add(storage.index_bytes)
                .ok_or("index accounting overflow")?;
        }
    }
    result.unique_sources = seen.len();
    result.buffer_bytes = result
        .byte_capacity
        .checked_add(result.index_bytes)
        .and_then(|total| total.checked_add(result.integrity_bytes))
        .ok_or("buffer accounting overflow")?;
    Ok(result)
}
fn rss() -> Option<usize> {
    std::fs::read_to_string("/proc/self/status")
        .ok()?
        .lines()
        .find(|line| line.starts_with("VmRSS:"))?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()
}
fn observation(
    directory: &Directory,
    capture: Capture,
    previous: Option<&Snapshot>,
) -> Result<Arc<Snapshot>, Error> {
    let full = capture.directories.is_some();
    let changed: BTreeSet<_> = capture
        .files
        .iter()
        .map(|file| file.path.as_str())
        .chain(capture.removed.iter().map(String::as_str))
        .collect();
    let mut documents = Vec::new();
    if !full && let Some(previous) = previous {
        documents.extend(
            previous
                .documents()
                .iter()
                .filter(|document| !changed.contains(document.path()))
                .cloned(),
        );
    }
    for file in capture.files {
        documents.push(Arc::new(Document::capture_source(
            directory.workspace().clone(),
            file.path,
            file.source,
            vec![],
            file.classification,
        )?));
    }
    Ok(Arc::new(Snapshot::new(
        directory.workspace().clone(),
        documents,
        directory.policy().id().clone(),
    )?))
}
fn source_manifest(snapshot: &Snapshot, rehash: bool) -> Result<String, Error> {
    let mut digest = blake3::Hasher::new();
    for document in snapshot.documents() {
        digest.update(&(document.path().len() as u64).to_le_bytes());
        digest.update(document.path().as_bytes());
        let actual = if rehash {
            ContentId::derive(&[document.captured().read_all(usize::MAX)?.bytes()])
        } else {
            document.content().clone()
        };
        digest.update(actual.as_str().as_bytes());
    }
    Ok(digest.finalize().to_hex().to_string())
}
fn record(
    phase: &str,
    publication: &Publication,
    ids: &[SnapshotId],
    external: Option<&Arc<Snapshot>>,
    elapsed_ms: Option<f64>,
    shared: usize,
    probe: &Probe<'_>,
) -> Result<Value, Error> {
    let (reclaim_ms, maintenance_reads) = probe.measure()?;
    let current = publication.current().into_iter().collect::<Vec<_>>();
    let retained = ids
        .iter()
        .filter_map(|id| publication.pinned(id.as_str()))
        .collect::<Vec<_>>();
    let mut union = retained.clone();
    if let Some(external) = external {
        union.push(external.clone());
    }
    let current_inventory = inventory(&current)?;
    let managed = inventory(&retained)?;
    let including_external = inventory(&union)?;
    Ok(
        json!({"phase":phase,"elapsed_ms":elapsed_ms,"shared_reopened_sources":shared,"retained_snapshots":retained.len(),
        "current":current_inventory,"publication":managed,"including_external":including_external,"rss_kib":rss(),"reclaim_ms":reclaim_ms,"spool":probe.spool.map(|spool|spool.stats()),"maintenance_reads":maintenance_reads}),
    )
}
fn run_probe(config: Config, emit: &mut impl FnMut(Value)) -> Result<Value, Error> {
    if config.owner.is_empty()
        || std::fs::read_to_string(config.root.join(".vizzi-source-owned"))? != config.owner
        || config.probe.is_empty()
        || config.targets.is_empty()
    {
        return Err(
            "source-storage probe requires targets, a unique probe and matching ownership marker"
                .into(),
        );
    }
    let mut directory =
        Directory::with_policy(&config.root, config.policy, Arc::new(Languages::builtin()?))?;
    let spool = if config.spool_mib == 0 {
        None
    } else {
        let path = config
            .root
            .parent()
            .ok_or("missing owned-copy parent")?
            .join("spool");
        Some(Arc::new(crate::source_spool::SourceSpool::new(
            &path,
            config
                .spool_mib
                .checked_mul(1024 * 1024)
                .ok_or("spool size overflow")?,
        )?))
    };
    if let Some(spool) = &spool {
        directory = directory.with_spool(spool.clone())?;
    }
    let reader = if config.maintenance_reads {
        Some(
            spool
                .as_ref()
                .ok_or("maintenance reads require spooling")?
                .persist(CapturedSource::capture(vec![b'p'; READER_BYTES], None))?,
        )
    } else {
        None
    };
    let reader_weak = reader.as_ref().map(Arc::downgrade);
    let mut probe = Probe {
        spool: spool.as_deref(),
        reader,
    };
    let mut publication = Publication::default();
    let mut ids = Vec::new();
    let mut weak_snapshots = Vec::new();
    let mut weak_sources = Vec::new();
    let mut phases = Vec::new();
    emit(json!({"event":"baseline_start"}));
    let started = Instant::now();
    let initial = observation(&directory, directory.capture()?, None)?;
    for path in &config.targets {
        if initial.file_by_path(path).is_none() {
            return Err("target outside captured scope".into());
        }
    }
    let initial_id = initial.id().clone();
    let original_manifest = source_manifest(&initial, false)?;
    let files = initial.documents().len();
    let changed: BTreeSet<String> = match config.mode {
        Mode::Headers => config.targets.iter().cloned().collect(),
        Mode::Workspace => initial
            .documents()
            .iter()
            .map(|document| document.path().to_string())
            .collect(),
    };
    weak_snapshots.push(Arc::downgrade(&initial));
    for path in &config.targets {
        weak_sources.push(Arc::downgrade(
            initial
                .file_by_path(path)
                .ok_or("missing initial source")?
                .captured(),
        ));
    }
    let mut external = Some(initial.clone());
    ids.push(initial_id.clone());
    let ticket = publication.request();
    publication.publish(ticket, initial);
    phases.push(record(
        "baseline",
        &publication,
        &ids,
        external.as_ref(),
        Some(started.elapsed().as_secs_f64() * 1000.0),
        0,
        &probe,
    )?);
    emit(json!({"event":"baseline_ready"}));
    {
        let previous = publication.current().ok_or("missing baseline")?;
        let started = Instant::now();
        let capture = directory
            .capture_reusing(Some(&previous), &CaptureScope::Reconcile, &|| false)?
            .ok_or("capture cancelled")?;
        let shared = capture
            .files
            .iter()
            .filter(|file| {
                previous
                    .file_by_path(&file.path)
                    .is_some_and(|old| Arc::ptr_eq(old.captured(), &file.source))
            })
            .count();
        let next = observation(&directory, capture, Some(&previous))?;
        let ticket = publication.request();
        if publication.publish(ticket, next) != Outcome::Unchanged || shared != files {
            return Err("unchanged capture did not preserve source sharing".into());
        }
        phases.push(record(
            "unchanged",
            &publication,
            &ids,
            external.as_ref(),
            Some(started.elapsed().as_secs_f64() * 1000.0),
            shared,
            &probe,
        )?);
    }
    let scope = match config.mode {
        Mode::Headers => CaptureScope::Paths(changed.iter().map(PathBuf::from).collect()),
        Mode::Workspace => CaptureScope::Reconcile,
    };
    let mut prefix_bytes = 0usize;
    for generation in 1..=4 {
        let previous = publication
            .current()
            .ok_or("missing previous observation")?;
        let prefix = format!(
            "// source-storage {} generation {generation}\n",
            config.probe
        );
        for path in &changed {
            let source = previous
                .file_by_path(path)
                .ok_or("changed input disappeared")?;
            let mut file = std::fs::File::create(config.root.join(path))?;
            file.write_all(prefix.as_bytes())?;
            file.write_all(source.captured().read_all(usize::MAX)?.bytes())?;
        }
        prefix_bytes += prefix.len();
        emit(json!({"event":"phase_start","generation":generation}));
        let started = Instant::now();
        let capture = directory
            .capture_reusing(Some(&previous), &scope, &|| false)?
            .ok_or("capture cancelled")?;
        for file in &capture.files {
            if let Some(old) = previous.file_by_path(&file.path) {
                if changed.contains(&file.path) {
                    if !file
                        .source
                        .read_all(usize::MAX)?
                        .bytes()
                        .starts_with(prefix.as_bytes())
                        || &file.source.read_all(usize::MAX)?.bytes()[prefix.len()..]
                            != old.captured().read_all(usize::MAX)?.bytes()
                    {
                        return Err("changed source bytes differ from expected prefix edit".into());
                    }
                } else if !Arc::ptr_eq(&file.source, old.captured()) {
                    return Err("unchanged source lost sharing".into());
                }
            }
        }
        let shared = capture
            .files
            .iter()
            .filter(|file| {
                previous
                    .file_by_path(&file.path)
                    .is_some_and(|old| Arc::ptr_eq(old.captured(), &file.source))
            })
            .count();
        let next = observation(&directory, capture, Some(&previous))?;
        weak_snapshots.push(Arc::downgrade(&next));
        for path in &config.targets {
            weak_sources.push(Arc::downgrade(
                next.file_by_path(path)
                    .ok_or("missing changed source")?
                    .captured(),
            ));
        }
        ids.push(next.id().clone());
        let ticket = publication.request();
        publication.publish(ticket, next);
        phases.push(record(
            &format!("edit_{generation}"),
            &publication,
            &ids,
            external.as_ref(),
            Some(started.elapsed().as_secs_f64() * 1000.0),
            shared,
            &probe,
        )?);
        emit(json!({"event":"phase_done","generation":generation}));
    }
    if publication.pinned(initial_id.as_str()).is_some() {
        return Err("initial pin did not expire from bounded publication history".into());
    }
    if source_manifest(external.as_ref().ok_or("external pin missing")?, true)? != original_manifest
    {
        return Err("external pin source bytes changed".into());
    }
    drop(external.take());
    phases.push(record(
        "external_released",
        &publication,
        &ids,
        None,
        None,
        0,
        &probe,
    )?);
    {
        let previous = publication.current().ok_or("missing current observation")?;
        for path in &changed {
            let source = previous
                .file_by_path(path)
                .ok_or("missing restoration source")?;
            let input = source.captured().read_all(usize::MAX)?;
            let bytes = input
                .bytes()
                .get(prefix_bytes..)
                .ok_or("invalid restoration prefix")?;
            std::fs::write(config.root.join(path), bytes)?;
        }
        let started = Instant::now();
        let restored = observation(
            &directory,
            directory
                .capture_reusing(Some(&previous), &CaptureScope::Reconcile, &|| false)?
                .ok_or("restoration cancelled")?,
            Some(&previous),
        )?;
        if restored.id() != &initial_id || source_manifest(&restored, false)? != original_manifest {
            return Err("restored source differs from baseline".into());
        }
        weak_snapshots.push(Arc::downgrade(&restored));
        for path in &config.targets {
            weak_sources.push(Arc::downgrade(
                restored
                    .file_by_path(path)
                    .ok_or("missing restored source")?
                    .captured(),
            ));
        }
        ids.push(restored.id().clone());
        let ticket = publication.request();
        publication.publish(ticket, restored);
        // The restored identity replaces an expired identity in the known-id list.
        ids.remove(0);
        phases.push(record(
            "restored",
            &publication,
            &ids,
            None,
            Some(started.elapsed().as_secs_f64() * 1000.0),
            0,
            &probe,
        )?);
    }
    publication = Publication::default();
    if weak_snapshots.iter().any(|weak| weak.upgrade().is_some())
        || weak_sources.iter().any(|weak| weak.upgrade().is_some())
    {
        return Err("source or snapshot handles remain alive after releasing ownership".into());
    }
    phases.push(record(
        "released",
        &publication,
        &ids,
        None,
        None,
        0,
        &probe,
    )?);
    if config.maintenance_reads {
        drop(probe.reader.take());
        phases.push(record(
            "reader_released",
            &publication,
            &ids,
            None,
            None,
            0,
            &probe,
        )?);
    }
    let reader_released = reader_weak.is_none_or(|weak| weak.upgrade().is_none());
    if !reader_released {
        return Err("maintenance reader remains alive".into());
    }
    Ok(
        json!({"reader_probe_bytes":if config.maintenance_reads { READER_BYTES } else { 0 },"reader_released":reader_released,"spool":spool.as_ref().map(|spool|spool.stats()),"mode":"unanalysed_source_storage","files":files,"changed_files":changed.len(),"initial_snapshot":initial_id,"original_source_manifest":original_manifest,
        "phases":phases,"expired_from_publication":true,"external_pin_bytes_match":true,"restored_snapshot_matches":true,"released_handles_dead":true}),
    )
}
/// Measures unique source buffer ownership in a marked disposable copy.
/// Analysis facts are intentionally omitted; source selection and publication are real.
pub fn run(configuration: &[u8], emit: &mut impl FnMut(Value)) -> Result<Value, Error> {
    run_probe(serde_json::from_slice(configuration)?, emit)
}
