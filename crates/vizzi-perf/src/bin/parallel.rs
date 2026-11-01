//! Compare bounded native parser pools with a sequential corpus baseline.

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    error::Error,
    fs,
    io::{BufReader, BufWriter},
    path::PathBuf,
    time::{Duration, Instant},
};
use vizzi_perf::{
    parallel::{self, ParsedFile},
    source,
    syntax::{SourceLanguage, SyntaxCounts},
    timing,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Serialize, Deserialize)]
struct Baseline {
    schema_version: u32,
    language: SourceLanguage,
    dependency_fingerprint: String,
    files: Vec<ParsedFile>,
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let language = match args.next().as_deref() {
        Some("rust") => SourceLanguage::Rust,
        Some("python") => SourceLanguage::Python,
        Some("c") => SourceLanguage::C,
        Some("cpp") => SourceLanguage::Cpp,
        _ => return Err(
            "use vizzi-parallel LANGUAGE ROOT WORKERS PASSES --baseline PATH|--write-baseline PATH"
                .into(),
        ),
    };
    let root = PathBuf::from(args.next().ok_or("missing source root")?).canonicalize()?;
    let workers: usize = args.next().ok_or("missing worker count")?.parse()?;
    let passes: usize = args.next().ok_or("missing pass count")?.parse()?;
    let mode = args.next().ok_or("a baseline mode is required")?;
    let baseline_path = PathBuf::from(args.next().ok_or("missing baseline path")?);
    if workers == 0 || workers > 128 || passes == 0 || args.next().is_some() {
        return Err("workers must be 1..=128 and passes must be positive".into());
    }
    let dependency_fingerprint = blake3::hash(include_bytes!("../../../../Cargo.lock"))
        .to_hex()
        .to_string();
    let mut expected = match mode.as_str() {
        "--baseline" => {
            let baseline: Baseline =
                serde_json::from_reader(BufReader::new(fs::File::open(&baseline_path)?))?;
            if baseline.schema_version != 1
                || baseline.language != language
                || baseline.dependency_fingerprint != dependency_fingerprint
            {
                return Err("baseline language or dependency context differs".into());
            }
            Some(baseline.files)
        }
        "--write-baseline" => {
            if workers != 1 {
                return Err("create a baseline with exactly one worker".into());
            }
            if baseline_path.exists() {
                return Err("baseline destination already exists".into());
            }
            None
        }
        _ => return Err("unknown baseline mode".into()),
    };

    let selection_started = Instant::now();
    let mut jobs = Vec::new();
    for path in source::files(&root)?
        .into_iter()
        .filter(|p| language.accepts(p))
    {
        let size = fs::metadata(root.join(&path))?.len();
        jobs.push((path, size));
    }
    jobs.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let paths: Vec<_> = jobs.into_iter().map(|(p, _)| p).collect();
    if paths.is_empty() {
        return Err("no files matched the language".into());
    }
    let selection_us = selection_started.elapsed().as_nanos() as f64 / 1000.0;
    let mut walls = Vec::new();
    let mut reports = Vec::new();
    for pass in 0..passes {
        let start = Instant::now();
        let batch = parallel::parse_files(&root, language, &paths, workers)?;
        let wall = start.elapsed();
        if batch.files.len() != paths.len() {
            return Err("pool returned an incomplete corpus".into());
        }
        if let Some(expected) = &expected
            && batch.files != *expected
        {
            return Err("corpus outputs differ from baseline".into());
        }
        let worker_stats: Vec<_> = batch
            .workers
            .iter()
            .enumerate()
            .map(|(id, w)| {
                json!({
                    "worker":id,"files":w.files,"bytes":w.bytes,
                    "read_us":w.reads.as_nanos() as f64/1000.0,
                    "parse_us":w.parsing.as_nanos() as f64/1000.0,
                    "inspect_us":w.inspection.as_nanos() as f64/1000.0,
                })
            })
            .collect();
        let sum = |select: fn(&parallel::WorkerStats) -> Duration| {
            batch
                .workers
                .iter()
                .map(select)
                .sum::<Duration>()
                .as_nanos() as f64
                / 1000.0
        };
        reports.push(json!({"wall_us":wall.as_nanos() as f64/1000.0,"actual_workers":batch.workers.len(),"workers":worker_stats,
            "summed_worker_phase_us":{"read":sum(|w|w.reads),"parse":sum(|w|w.parsing),"inspect":sum(|w|w.inspection)},"validation_passed":true}));
        if expected.is_none() {
            expected = Some(batch.files);
        }
        walls.push(wall);
        eprintln!(
            "pool pass {}/{}: {} workers, {} files, {:.3}s wall",
            pass + 1,
            passes,
            workers,
            paths.len(),
            wall.as_secs_f64()
        );
    }
    let expected = expected.ok_or("no successful pass")?;
    let fingerprint = blake3::hash(&serde_json::to_vec(&expected)?)
        .to_hex()
        .to_string();
    let mut totals = SyntaxCounts::default();
    let mut bytes = 0;
    let mut recovery = 0;
    for file in &expected {
        bytes += file.bytes;
        totals.nodes += file.counts.nodes;
        totals.named += file.counts.named;
        totals.errors += file.counts.errors;
        totals.missing += file.counts.missing;
        recovery += usize::from(file.counts.errors > 0 || file.counts.missing > 0);
    }
    let output = json!({"experiment":"parallel_syntax","language":language,"source_root":root,
    "requested_workers":workers,"actual_workers":workers.min(paths.len()),"files":paths.len(),"content_bytes":bytes,
    "syntax_counts":totals,"files_with_syntax_recovery":recovery,"result_fingerprint":fingerprint,
    "validation":if mode=="--write-baseline" {"created_sequential_baseline"} else {"sequential_baseline"},
    "selection_and_sizing_us":selection_us,"pass_wall":timing::summarize(&walls),"passes":reports,
    "conditions":{"build":if cfg!(debug_assertions){"debug"}else{"release"},
        "scope":"raw syntax only; no semantic analysis or retained whole-project trees",
        "pool":"new bounded scoped threads each pass; one parser per worker, reused across files",
        "schedule":"dynamic atomic queue, descending source byte size with path tie-breaker",
        "wall":"includes pool startup, parser setup, reads, parse, inspection, hashing, tree disposal, joins, and result sorting",
        "excluded":"file discovery and sizing, baseline loading/writing, result comparison and JSON digest",
        "phases":"summed worker phase wall durations, not CPU time or latency; may exceed pass wall time",
        "memory":"process high-water RSS also includes baseline/result vectors and their serialization",
        "validation":"every file's path, content hash, byte count, and syntax counts match the sequential baseline; full AST shapes are not compared in this experiment",
        "dependency_fingerprint":dependency_fingerprint
    }});
    if mode == "--write-baseline" {
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(baseline_path)?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer(
            &mut writer,
            &Baseline {
                schema_version: 1,
                language,
                dependency_fingerprint,
                files: expected,
            },
        )?;
        std::io::Write::flush(&mut writer)?;
    }
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
