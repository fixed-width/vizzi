//! Optional owned-copy probe for admission during foreground edits and recovery.

use crate::{
    admission::Admission,
    cache::{Cache, external_path},
    engine::Engine,
    host::{Host, HostOptions},
    source::{CaptureScope, Directory},
    syntax_cache::SyntaxCache,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use vizzi_core::{analysis::Languages, model::Coverage, policy::PolicyConfig};

type Error = Box<dyn std::error::Error>;
#[derive(Deserialize)]
struct Config {
    root: PathBuf,
    cache: PathBuf,
    policy: PolicyConfig,
    targets: Vec<String>,
    #[serde(default)]
    spool_mib: usize,
    owner: String,
    probe: String,
    input_mib: usize,
    #[serde(default = "default_interval")]
    recovery_ms: u64,
}
fn default_interval() -> u64 {
    2_000
}
fn engine(config: &Config) -> Result<Engine, Error> {
    Ok(Engine::new(
        Directory::with_policy(
            &config.root,
            config.policy.clone(),
            Arc::new(Languages::builtin()?),
        )?,
        Cache::new(external_path(&config.cache, &config.root)?),
        4,
    )?)
}
fn wait(mut predicate: impl FnMut() -> bool) -> Result<(), Error> {
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(60) {
        if predicate() {
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    Err("admission probe timed out waiting for publication/recovery".into())
}
fn marker(language: &str) -> &str {
    if language == "python" { "#" } else { "//" }
}
fn run_probe(config: Config, emit: &mut impl FnMut(Value)) -> Result<Value, Error> {
    if config.owner.is_empty()
        || std::fs::read_to_string(config.root.join(".vizzi-admission-owned"))? != config.owner
        || config.probe.is_empty()
        || config.targets.len() != 4
    {
        return Err(
            "admission probe requires four targets, a unique probe and matching owned-copy marker"
                .into(),
        );
    }
    let admission = if config.input_mib == 0 {
        None
    } else {
        Some(Arc::new(Admission::for_budget(
            config
                .input_mib
                .checked_mul(1024 * 1024)
                .ok_or("input budget overflow")?,
        )?))
    };
    let syntax = Arc::new(SyntaxCache::for_budget(512 * 1024 * 1024)?);
    let mut live = engine(&config)?.with_syntax_cache(syntax.clone());
    if config.spool_mib != 0 {
        let path = config
            .root
            .parent()
            .ok_or("missing owned-copy parent")?
            .join("spool");
        let spool = Arc::new(crate::source_spool::SourceSpool::new(
            &path,
            config
                .spool_mib
                .checked_mul(1024 * 1024)
                .ok_or("spool size overflow")?,
        )?);
        live = Engine::new(
            Directory::with_policy(
                &config.root,
                config.policy.clone(),
                Arc::new(Languages::builtin()?),
            )?
            .with_spool(spool)?,
            Cache::new(external_path(&config.cache, &config.root)?),
            4,
        )?
        .with_syntax_cache(syntax.clone());
    }
    if let Some(budget) = &admission {
        live = live.with_admission(budget.clone());
    }
    emit(json!({"event":"baseline_start"}));
    let host = Host::start_with_options(
        live,
        HostOptions {
            watch: false,
            reconcile_interval: Duration::from_millis(config.recovery_ms),
        },
    )?;
    let app = host.application();
    wait(|| {
        !app.status().pending
            && !app.status().recovery.running
            && app
                .status()
                .measurements
                .as_ref()
                .is_some_and(|m| m.deferred == 0)
    })?;
    let baseline = app.snapshot().ok_or("missing baseline")?;
    for path in &config.targets {
        let file = baseline
            .file_by_path(path)
            .ok_or("target outside captured scope")?;
        if file.encoding() != vizzi_core::model::SourceEncoding::Utf8 || file.analyses().len() != 1
        {
            return Err("targets require UTF-8 and assigned analysis".into());
        }
    }
    emit(json!({"event":"baseline_ready"}));
    let ordinary = &config.targets[0];
    let primary = &config.targets[1];
    let ordinary_file = baseline
        .file_by_path(ordinary)
        .ok_or("missing ordinary input")?;
    let header = baseline.file_by_path(primary).ok_or("missing header")?;
    let ordinary_original = ordinary_file
        .captured()
        .read_all(usize::MAX)?
        .bytes()
        .to_vec();
    let language = &ordinary_file.analyses()[0].context.language;
    let extension = match header.analyses()[0].context.language.as_str() {
        "c" => "c",
        "cpp" => "cpp",
        "python" => "py",
        _ => "rs",
    };
    let unseen = format!("vizzi_admission_probe.{extension}");
    if config.root.join(&unseen).exists() {
        return Err("unseen probe destination already exists".into());
    }
    let mut cases = Vec::new();
    for kind in ["structural", "large_paste", "unseen", "batch"] {
        let mut writes = BTreeMap::new();
        if kind == "structural" {
            let mut bytes =
                format!("{} structural {}\n", marker(language), config.probe).into_bytes();
            bytes.extend_from_slice(ordinary_file.captured().read_all(usize::MAX)?.bytes());
            bytes.push(b'\n');
            for index in 0..1000 {
                let declaration = match language.as_str() {
                    "rust" => format!("fn admission_{index}() {{}}\n"),
                    "python" => format!("def admission_{index}(): pass\n"),
                    _ => format!("int admission_{index}(void) {{ return {index}; }}\n"),
                };
                bytes.extend_from_slice(declaration.as_bytes());
            }
            writes.insert(ordinary.clone(), bytes);
        } else if kind == "large_paste" || kind == "unseen" {
            let mut bytes = format!("{} {kind} {}\n", marker(language), config.probe).into_bytes();
            if kind == "large_paste" {
                bytes.extend_from_slice(ordinary_file.captured().read_all(usize::MAX)?.bytes());
                bytes.push(b'\n');
            }
            bytes.extend_from_slice(header.captured().read_all(usize::MAX)?.bytes());
            if kind == "large_paste" {
                bytes.extend(std::iter::repeat_n(b' ', 2 * 1024 * 1024));
            }
            writes.insert(
                if kind == "unseen" {
                    unseen.clone()
                } else {
                    ordinary.clone()
                },
                bytes,
            );
        } else {
            for path in &config.targets[1..] {
                let file = baseline.file_by_path(path).ok_or("missing batch input")?;
                let mut bytes = format!(
                    "{} batch {}\n",
                    marker(&file.analyses()[0].context.language),
                    config.probe
                )
                .into_bytes();
                bytes.extend_from_slice(file.captured().read_all(usize::MAX)?.bytes());
                writes.insert(path.clone(), bytes);
            }
        }
        wait(|| app.status().recovery.running && !app.status().pending)?;
        let before = app.status();
        emit(json!({"event":"phase_start","kind":kind}));
        let started = Instant::now();
        for (path, bytes) in &writes {
            std::fs::write(config.root.join(path), bytes)?;
        }
        host.request(CaptureScope::Paths(
            writes.keys().map(PathBuf::from).collect(),
        ));
        wait(|| {
            !app.status().pending
                && app.snapshot().is_some_and(|snapshot| {
                    writes.iter().all(|(path, bytes)| {
                        snapshot.file_by_path(path).is_some_and(|file| {
                            file.content() == &vizzi_core::model::ContentId::derive(&[bytes])
                        })
                    })
                })
        })?;
        let visible_ms = started.elapsed().as_secs_f64() * 1000.0;
        let observation = app.snapshot().ok_or("missing observation")?;
        let status = app.status();
        let stats = admission.as_ref().map(|budget| budget.stats());
        emit(json!({"event":"verify_start","kind":kind}));
        let capture = engine(&config)?.directory().capture()?;
        if capture.files.len() != observation.documents().len()
            || capture.files.iter().any(|file| {
                observation.file_by_path(&file.path).is_none_or(|old| {
                    old.content() != file.source.content()
                        || old.classification() != &file.classification
                })
            })
        {
            return Err("captured source/scope differs from independent discovery".into());
        }
        let file_limit = admission
            .as_ref()
            .map_or(usize::MAX, |budget| budget.stats().file_limit_bytes);
        wait(|| {
            !app.status().pending
                && app.snapshot().is_some_and(|snapshot| {
                    writes.iter().all(|(path, bytes)| {
                        snapshot.file_by_path(path).is_some_and(|file| {
                            file.content() == &vizzi_core::model::ContentId::derive(&[bytes])
                                && (bytes.len() > file_limit
                                    || file.analyses()[0].coverage != Coverage::Deferred)
                        })
                    })
                })
        })?;
        let settled = app.snapshot().ok_or("missing settled observation")?;
        let languages = Languages::builtin()?;
        let mut documents = Vec::new();
        let mut settled_documents = Vec::new();
        for path in writes.keys() {
            let initial = observation
                .file_by_path(path)
                .ok_or("edited source disappeared")?;
            let file = settled
                .file_by_path(path)
                .ok_or("settled source disappeared")?;
            let analysis = &file.analyses()[0];
            if analysis.coverage != Coverage::Deferred {
                let fresh = languages
                    .select(&analysis.context.language)
                    .ok_or("language unavailable")?
                    .create()?
                    .analyze(file.captured().read_all(usize::MAX)?.bytes())?;
                if file.analyses() != [fresh] {
                    return Err("completed facts differ from independent fresh parse".into());
                }
            }
            documents.push(json!({"path":path,"bytes":initial.bytes(),"coverage":initial.analyses()[0].coverage,"diagnostics":initial.analyses()[0].diagnostics}));
            settled_documents.push(json!({"path":path,"coverage":analysis.coverage,"diagnostics":analysis.diagnostics}));
        }
        cases.push(json!({"kind":kind,"write_to_published_source_ms":visible_ms,"status_before":before,"status_after":status,
            "documents":documents,"settled_documents":settled_documents,"all_admissible_completed":true,"admission":stats,"syntax_cache":syntax.stats(),"source_scope_matches":true,"completed_facts_match":true}));
    }
    for path in &config.targets {
        std::fs::write(
            config.root.join(path),
            baseline
                .file_by_path(path)
                .ok_or("missing original")?
                .captured()
                .read_all(usize::MAX)?
                .bytes(),
        )?;
    }
    std::fs::remove_file(config.root.join(&unseen))?;
    host.request(CaptureScope::Reconcile);
    wait(|| {
        !app.status().pending
            && app
                .snapshot()
                .is_some_and(|snapshot| snapshot.id() == baseline.id())
    })?;
    let clean = engine(&config)?
        .refresh(None, &|| false)?
        .ok_or("clean capture cancelled")?;
    if clean.snapshot.id() != baseline.id() {
        return Err("restored complete observation differs from clean capture".into());
    }
    drop(host);
    Ok(
        json!({"files":baseline.documents().len(),"analyzed":baseline.analyzed_files(),"baseline_snapshot":baseline.id(),"cases":cases,
        "final_clean_matches":true,"old_pin_source_matches":baseline.file_by_path(ordinary).ok_or("missing pin")?.captured().read_all(usize::MAX)?.bytes()==ordinary_original,
        "admission_final":admission.as_ref().map(|budget|budget.stats())}),
    )
}
/// Measures structural/cold edits with explicit source and completed-fact checks.
/// Edits are restricted to a marked disposable copy; all targets are restored.
pub fn run(configuration: &[u8], emit: &mut impl FnMut(Value)) -> Result<Value, Error> {
    run_probe(serde_json::from_slice(configuration)?, emit)
}
