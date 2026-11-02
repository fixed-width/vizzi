//! Optional owned-copy probe for syntax retention across mixed file working sets.

use crate::{
    cache::{Cache, external_path},
    engine::Engine,
    source::{CaptureScope, Directory},
    syntax_cache::{SyntaxCache, SyntaxKey},
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{collections::BTreeSet, path::PathBuf, sync::Arc};
use vizzi_core::{analysis::Languages, model::Snapshot, policy::PolicyConfig};

type Error = Box<dyn std::error::Error>;
#[derive(Deserialize)]
struct Config {
    root: PathBuf,
    cache: PathBuf,
    policy: PolicyConfig,
    workers: usize,
    targets: Vec<String>,
    steps: Vec<Step>,
    budget_mib: usize,
    probe: String,
    owner: String,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum Step {
    Edit { paths: Vec<String> },
    Reconcile,
}
fn engine(config: &Config) -> Result<Engine, Error> {
    Ok(Engine::new(
        Directory::with_policy(
            &config.root,
            config.policy.clone(),
            Arc::new(Languages::builtin()?),
        )?,
        Cache::new(external_path(&config.cache, &config.root)?),
        config.workers,
    )?)
}
fn entries(config: &Config, snapshot: &Snapshot, pool: Option<&SyntaxCache>) -> Value {
    config
        .targets
        .iter()
        .map(|path| {
            let entry = pool.and_then(|pool| {
                snapshot.file_by_path(path).and_then(|file| {
                    file.analyses().first().and_then(|analysis| {
                        pool.entry(&SyntaxKey::new(
                            snapshot.workspace(),
                            path,
                            &analysis.context,
                        ))
                    })
                })
            });
            (path.clone(), json!(entry))
        })
        .collect::<serde_json::Map<_, _>>()
        .into()
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
fn qualify(config: Config, emit: &mut impl FnMut(Value)) -> Result<Value, Error> {
    if config.owner.is_empty()
        || std::fs::read_to_string(config.root.join(".vizzi-working-set-owned"))? != config.owner
    {
        return Err("working-set probe requires an owned-copy marker".into());
    }
    if config.probe.is_empty() || config.targets.is_empty() || config.steps.is_empty() {
        return Err("working-set probe requires targets, steps, and a unique probe id".into());
    }
    let targets: BTreeSet<_> = config.targets.iter().collect();
    if targets.len() != config.targets.len() {
        return Err("duplicate target".into());
    }
    for step in &config.steps {
        if let Step::Edit { paths } = step
            && (paths.is_empty()
                || paths.iter().collect::<BTreeSet<_>>().len() != paths.len()
                || paths.iter().any(|path| !targets.contains(path)))
        {
            return Err("edit steps require distinct configured targets".into());
        }
    }
    let budget = config
        .budget_mib
        .checked_mul(1024 * 1024)
        .ok_or("retention weight overflow")?;
    let pool = if budget == 0 {
        None
    } else {
        Some(Arc::new(SyntaxCache::for_budget(budget)?))
    };
    let mut live = engine(&config)?;
    if let Some(pool) = &pool {
        live = live.with_syntax_cache(pool.clone());
    }
    emit(json!({"event":"baseline_start"}));
    let initial = live.refresh(None, &|| false)?.ok_or("baseline cancelled")?;
    let baseline = initial.snapshot;
    for target in &config.targets {
        let file = baseline
            .file_by_path(target)
            .ok_or("target outside captured scope")?;
        if file.source().is_none() || file.analyses().len() != 1 {
            return Err("targets require UTF-8 and one assigned analysis".into());
        }
    }
    emit(json!({"event":"baseline_ready"}));
    let mut previous = baseline.clone();
    let mut cases = Vec::new();
    for (index, step) in config.steps.iter().enumerate() {
        let before = entries(&config, &baseline, pool.as_deref());
        let stats_before = pool.as_ref().map(|pool| pool.stats());
        let paths = match step {
            Step::Edit { paths } => paths.as_slice(),
            Step::Reconcile => &[],
        };
        for path in paths {
            let file = baseline
                .file_by_path(path)
                .ok_or("validated target disappeared")?;
            let comment = if file.analyses()[0].context.language == "python" {
                "#"
            } else {
                "//"
            };
            let marker = format!(
                "{comment} working set {} step {index} {path}\n",
                config.probe
            );
            let mut source = marker.into_bytes();
            source.extend_from_slice(file.captured().bytes());
            std::fs::write(config.root.join(path), source)?;
        }
        let scope = match step {
            Step::Edit { .. } => CaptureScope::Paths(paths.iter().map(PathBuf::from).collect()),
            Step::Reconcile => CaptureScope::Reconcile,
        };
        emit(json!({"event":"phase_start","step":index,"paths":paths}));
        let result = live
            .refresh_scoped(Some(&previous), &scope, &|| false)?
            .ok_or("working-set refresh cancelled")?;
        let rss_before_verification_kib = rss();
        let after = entries(&config, &baseline, pool.as_deref());
        let stats_after = pool.as_ref().map(|pool| pool.stats());
        let m = &result.measurements;
        if m.parsed != paths.len()
            || m.cache_hits != 0
            || m.reconciled != paths.is_empty()
            || (!paths.is_empty() && m.read_files != paths.len())
        {
            return Err("working-set attempt did not match fresh parse scope".into());
        }
        emit(json!({"event":"verify_start","step":index}));
        let languages = Languages::builtin()?;
        for path in paths {
            let file = result
                .snapshot
                .file_by_path(path)
                .ok_or("edited source disappeared")?;
            let context = &file.analyses()[0].context;
            let fresh = languages
                .select(&context.language)
                .ok_or("target language unavailable")?
                .create()?
                .analyze(file.captured().bytes())?;
            if file.analyses() != [fresh] {
                return Err("working-set facts differ from independent fresh parse".into());
            }
        }
        let clean = engine(&config)?
            .refresh(None, &|| false)?
            .ok_or("clean capture cancelled")?;
        if result.snapshot.id() != clean.snapshot.id() {
            return Err("working-set snapshot differs from clean capture".into());
        }
        cases.push(json!({"step":index,"paths":paths,"reconciliation":paths.is_empty(),"measurements":result.measurements,"entries_before":before,"entries_after":after,
            "syntax_cache_before":stats_before,"syntax_cache_after":stats_after,"rss_before_verification_kib":rss_before_verification_kib,"snapshot":result.snapshot.id(),
            "clean_snapshot_matches":true,"independent_facts_match":true}));
        previous = result.snapshot;
    }
    for path in &config.targets {
        std::fs::write(
            config.root.join(path),
            baseline
                .file_by_path(path)
                .ok_or("validated target disappeared")?
                .captured()
                .bytes(),
        )?;
    }
    Ok(
        json!({"files":baseline.documents().len(),"analyzed":baseline.analyzed_files(),"baseline_snapshot":baseline.id(),"baseline_measurements":initial.measurements,"budget_mib":config.budget_mib,"cases":cases}),
    )
}
/// Runs configured edit batches and verification steps only in a marked owned copy.
/// Inspects target availability without changing LRU order and verifies every result.
pub fn run(configuration: &[u8], emit: &mut impl FnMut(Value)) -> Result<Value, Error> {
    qualify(serde_json::from_slice(configuration)?, emit)
}
