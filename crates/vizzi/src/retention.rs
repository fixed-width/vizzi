//! Optional owned-copy probe for repeated edits through production analysis.

use crate::{
    cache::Cache,
    engine::{Engine, Measurements},
    source::{CaptureScope, Directory},
    syntax_cache::SyntaxCache,
};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};
use vizzi_core::{analysis::Languages, policy::PolicyConfig};

type Error = Box<dyn std::error::Error>;
#[derive(Deserialize)]
struct Config {
    root: PathBuf,
    cache: PathBuf,
    policy: PolicyConfig,
    workers: usize,
    targets: Vec<String>,
    samples: usize,
    budget_mib: usize,
    probe: String,
    owner: String,
}
fn engine(config: &Config) -> Result<Engine, Error> {
    Ok(Engine::new(
        Directory::with_policy(
            &config.root,
            config.policy.clone(),
            Arc::new(Languages::builtin()?),
        )?,
        Cache::new(config.cache.clone()),
        config.workers,
    )?)
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
fn edit_offset(source: &str, position: &str) -> usize {
    match position {
        "start" => 0,
        "middle" => source.as_bytes()[..source.len() / 2]
            .iter()
            .rposition(|byte| *byte == b'\n')
            .map_or(0, |index| index + 1),
        _ => source.len(),
    }
}
fn validate_attempt(measurements: &Measurements) -> Result<(), Error> {
    if measurements.parsed != 1
        || measurements.read_files != 1
        || measurements.cache_hits != 0
        || measurements.reconciled
    {
        return Err("probe did not measure one fresh targeted parse".into());
    }
    Ok(())
}

fn qualify(config: Config, emit: &mut impl FnMut(Value)) -> Result<Value, Error> {
    if config.owner.is_empty()
        || std::fs::read_to_string(config.root.join(".vizzi-retention-owned"))? != config.owner
    {
        return Err("retention edit probe requires an owned-copy marker".into());
    }
    if !(1..=8).contains(&config.samples) || config.targets.is_empty() || config.probe.is_empty() {
        return Err("retention probe needs targets, a unique probe id, and 1..8 samples".into());
    }
    let budget = config
        .budget_mib
        .checked_mul(1024 * 1024)
        .ok_or("retention weight overflow")?;
    emit(json!({"event":"baseline_start"}));
    let baseline = engine(&config)?
        .refresh(None, &|| false)?
        .ok_or("baseline cancelled")?
        .snapshot;
    emit(json!({"event":"baseline_ready"}));
    let mut cases = Vec::new();
    for target in &config.targets {
        let file = baseline
            .file_by_path(target)
            .ok_or("target is outside captured scope")?;
        let original = file.resident_source().ok_or("target must be UTF-8")?;
        let context = &file
            .analyses()
            .first()
            .ok_or("target requires assigned analysis")?
            .context;
        let path = config.root.join(target);
        for position in ["start", "middle", "end"] {
            let pool = if budget == 0 {
                None
            } else {
                Some(Arc::new(SyntaxCache::for_budget(budget)?))
            };
            let mut live = engine(&config)?;
            if let Some(pool) = &pool {
                live = live.with_syntax_cache(pool.clone());
            }
            let mut previous = baseline.clone();
            let offset = edit_offset(original, position);
            let scope = CaptureScope::Paths([PathBuf::from(target)].into());
            for sample in 0..=config.samples {
                let comment = if context.language == "python" {
                    "#"
                } else {
                    "//"
                };
                let marker = format!(
                    "\n{comment} retained syntax {} {target} {position} {sample}\n",
                    config.probe
                );
                let mut source = Vec::with_capacity(original.len() + marker.len());
                source.extend_from_slice(&original.as_bytes()[..offset]);
                source.extend_from_slice(marker.as_bytes());
                source.extend_from_slice(&original.as_bytes()[offset..]);
                std::fs::write(&path, &source)?;
                emit(
                    json!({"event":"phase_start","path":target,"position":position,"sample":sample}),
                );
                let result = live
                    .refresh_scoped(Some(&previous), &scope, &|| false)?
                    .ok_or("edit cancelled")?;
                let rss_before_verification_kib = rss();
                validate_attempt(&result.measurements)?;
                let stats = pool.as_ref().map(|pool| pool.stats());
                emit(json!({"event":"verify_start"}));
                let languages = Languages::builtin()?;
                let fresh = languages
                    .select(&context.language)
                    .ok_or("target language unavailable")?
                    .create()?
                    .analyze(&source)?;
                if result
                    .snapshot
                    .file_by_path(target)
                    .ok_or("edited source disappeared")?
                    .analyses()
                    != [fresh]
                {
                    return Err("retained facts differ from an independent fresh parse".into());
                }
                let clean = engine(&config)?
                    .refresh(None, &|| false)?
                    .ok_or("clean capture cancelled")?;
                if result.snapshot.id() != clean.snapshot.id() {
                    return Err("retained result differs from clean capture".into());
                }
                cases.push(json!({"path":target,"position":position,"sample":sample,"priming":sample==0,"source_bytes":source.len(),"measurements":result.measurements,
                    "edit_offset":offset,"comment_prefix":comment,"syntax_cache":stats,"rss_before_verification_kib":rss_before_verification_kib,"snapshot":result.snapshot.id(),"clean_snapshot_matches":true,"independent_facts_match":true}));
                previous = result.snapshot;
            }
            std::fs::write(&path, original)?;
        }
    }
    Ok(
        json!({"baseline_snapshot":baseline.id(),"files":baseline.documents().len(),"analyzed":baseline.analyzed_files(),"budget_mib":config.budget_mib,"cases":cases}),
    )
}
/// Runs repeated edits only in a directory bearing the expected ownership marker.
/// Results separate priming from measured edits and verify each with a clean capture.
pub fn run(configuration: &[u8], emit: &mut impl FnMut(Value)) -> Result<Value, Error> {
    qualify(serde_json::from_slice(configuration)?, emit)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn edit_positions_and_rejected_measurements_keep_probe_conditions_distinct() {
        assert_eq!(edit_offset("a\nb\nc\nd\n", "start"), 0);
        assert_eq!(edit_offset("a\nb\nc\nd\n", "middle"), 4);
        assert_eq!(edit_offset("a\nb\nc\nd\n", "end"), 8);
        assert_eq!(edit_offset("abc", "middle"), 0);
        let good = Measurements {
            parsed: 1,
            read_files: 1,
            ..Default::default()
        };
        assert!(validate_attempt(&good).is_ok());
        for bad in [
            Measurements {
                parsed: 0,
                ..good.clone()
            },
            Measurements {
                parsed: 2,
                ..good.clone()
            },
            Measurements {
                read_files: 0,
                ..good.clone()
            },
            Measurements {
                read_files: 2,
                ..good.clone()
            },
            Measurements {
                cache_hits: 1,
                ..good.clone()
            },
            Measurements {
                reconciled: true,
                ..good.clone()
            },
        ] {
            assert!(validate_attempt(&bad).is_err());
        }
    }
}
