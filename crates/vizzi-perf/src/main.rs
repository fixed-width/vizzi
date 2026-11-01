//! Run native content-scan and manifest-update experiments.

use serde_json::json;
use std::{
    collections::BTreeMap,
    error::Error,
    fs,
    hint::black_box,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use vizzi_perf::{
    snapshot::{Fingerprint, Manifest},
    source, timing,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("inventory") => {
            let root = PathBuf::from(args.next().ok_or("inventory requires a source directory")?)
                .canonicalize()?;
            let samples: usize = args.next().as_deref().unwrap_or("7").parse()?;
            if samples == 0 || args.next().is_some() {
                return Err("use inventory ROOT [NONZERO_SAMPLES]".into());
            }
            inventory(&root, samples)
        }
        Some("generate") => {
            let root = PathBuf::from(args.next().ok_or("generate requires a new directory")?);
            let count: usize = args
                .next()
                .ok_or("generate requires a file count")?
                .parse()?;
            let bytes: usize = args.next().as_deref().unwrap_or("1024").parse()?;
            if count == 0 || bytes < 128 || args.next().is_some() {
                return Err("use generate NEW_ROOT NONZERO_FILES [BYTES_AT_LEAST_128]".into());
            }
            generate(&root, count, bytes)
        }
        _ => Err(
            "usage: vizzi-perf inventory ROOT [SAMPLES] | generate NEW_ROOT FILES [BYTES]".into(),
        ),
    }
}

fn inventory(root: &Path, samples: usize) -> Result<()> {
    let started = Instant::now();
    let baseline = source::capture(root)?;
    let first_scan = started.elapsed();
    if baseline.files().is_empty() {
        return Err("source directory has no included regular files".into());
    }
    let paths: Vec<_> = baseline.files().keys().cloned().collect();
    let bytes: u64 = baseline.files().values().map(|file| file.bytes).sum();
    let encoded = serde_json::to_vec(&baseline)?;
    let mut timings: BTreeMap<&str, Vec<Duration>> = BTreeMap::new();
    for _ in 0..samples {
        let start = Instant::now();
        let discovered = source::files(root)?;
        let elapsed = start.elapsed();
        if discovered != paths {
            return Err("source paths changed during measurement".into());
        }
        timings.entry("discovery").or_default().push(elapsed);

        let start = Instant::now();
        for path in &paths {
            let metadata = fs::symlink_metadata(root.join(path))?;
            black_box((metadata.len(), metadata.modified()?));
        }
        timings
            .entry("metadata_for_known_files")
            .or_default()
            .push(start.elapsed());

        let start = Instant::now();
        let refreshed = source::capture(root)?;
        let changes = refreshed.changes_from(&baseline);
        let elapsed = start.elapsed();
        if !changes.is_empty() {
            return Err("source contents changed during measurement".into());
        }
        timings
            .entry("verified_rescan_and_diff")
            .or_default()
            .push(elapsed);

        let start = Instant::now();
        let decoded: Manifest = serde_json::from_slice(&encoded)?;
        let elapsed = start.elapsed();
        if decoded != baseline {
            return Err("manifest round trip changed contents".into());
        }
        timings
            .entry("manifest_json_decode")
            .or_default()
            .push(elapsed);
    }

    let selected = paths
        .iter()
        .find(|p| p.extension().is_some_and(|ext| ext == "rs"))
        .or_else(|| paths.first())
        .ok_or("no file to update")?;
    let original = fs::read(root.join(selected))?;
    let mut modified = original.clone();
    modified.extend_from_slice(b"\n// benchmark edit\n");
    let mut working = baseline.clone();
    for index in 0..samples * 100 {
        let contents = if index % 2 == 0 { &modified } else { &original };
        let start = Instant::now();
        let changed = working.apply(BTreeMap::from([(
            selected.clone(),
            Some(Fingerprint::of(contents)),
        )]));
        let elapsed = start.elapsed();
        if changed.len() != 1 {
            return Err("buffer update did not identify exactly one change".into());
        }
        timings
            .entry("supplied_buffer_hash_and_update")
            .or_default()
            .push(elapsed);
    }
    if working != baseline {
        return Err("reverted updates did not restore the baseline".into());
    }

    let summaries: BTreeMap<_, _> = timings
        .iter()
        .map(|(name, values)| (*name, timing::summarize(values)))
        .collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schema_version": 1,
            "experiment": "content_inventory",
            "source_root": root,
            "files": paths.len(), "content_bytes": bytes,
            "manifest_json_bytes": encoded.len(),
            "first_observed_scan_us": first_scan.as_nanos() as f64 / 1000.0,
            "update_file": selected, "update_file_bytes": original.len(),
            "measurements": summaries,
            "conditions": {
                "build": if cfg!(debug_assertions) { "debug" } else { "release" },
                "os": std::env::consts::OS, "arch": std::env::consts::ARCH,
                "filesystem_cache": "uncontrolled; first observed scan is not a cold-cache measurement",
                "update_input": "supplied in-memory buffer; excludes event detection and disk reads",
                "analysis": "content fingerprints only; no parsing, semantic analysis, or dependency invalidation",
                "manifest_cache": "JSON decoded from memory; disk loading is not measured",
                "excluded_root_directories": [".git", "target", "node_modules"],
                "symlinks": "not followed"
            }
        }))?
    );
    Ok(())
}

fn generate(root: &Path, count: usize, bytes: usize) -> Result<()> {
    fs::create_dir(root)?;
    for index in 0..count {
        let directory = root.join(format!("group-{:05}", index / 100));
        fs::create_dir_all(&directory)?;
        let mut contents = format!("// synthetic performance fixture {index}\npub fn item_{index}() -> usize {{ {index} }}\n//").into_bytes();
        contents.resize(bytes, b'x');
        fs::write(directory.join(format!("item-{index:08}.rs")), contents)?;
    }
    println!(
        "{}",
        json!({"experiment":"synthetic_fixture", "files":count, "bytes_per_file":bytes, "root":root})
    );
    Ok(())
}
