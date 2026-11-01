//! Measure syntax parsing independently of semantic indexing.

use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    error::Error,
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use vizzi_perf::{
    source,
    syntax::{self, SourceLanguage, SyntaxCounts, SyntaxError},
    timing,
};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let language = match args.next().as_deref() {
        Some("rust") => SourceLanguage::Rust,
        Some("python") => SourceLanguage::Python,
        Some("c") => SourceLanguage::C,
        Some("cpp") => SourceLanguage::Cpp,
        _ => return Err("use vizzi-syntax LANGUAGE ROOT [NONZERO_SAMPLES] [MAX_FILES]".into()),
    };
    let root = PathBuf::from(args.next().ok_or("missing source root")?).canonicalize()?;
    let samples: usize = args.next().as_deref().unwrap_or("3").parse()?;
    let limit = args
        .next()
        .map(|value| value.parse::<usize>())
        .transpose()?
        .unwrap_or(usize::MAX);
    if samples == 0 || limit == 0 || args.next().is_some() {
        return Err("samples and file limit must be positive".into());
    }
    let discovered: Vec<_> = source::files(&root)?
        .into_iter()
        .filter(|p| language.accepts(p))
        .collect();
    let matching = discovered.len();
    let selected: Vec<_> = discovered.into_iter().take(limit).collect();
    if selected.is_empty() {
        return Err("no files matched the selected language".into());
    }
    let mut parser = language.parser()?;
    let mut baseline = BTreeMap::new();
    let mut readings = Vec::new();
    let mut parsings = Vec::new();
    let mut inspections = Vec::new();
    let mut totals = SyntaxCounts::default();
    let mut error_files = 0;
    for pass in 0..samples {
        let mut read_time = Duration::ZERO;
        let mut parse_time = Duration::ZERO;
        let mut inspect_time = Duration::ZERO;
        for path in &selected {
            let start = Instant::now();
            let bytes = fs::read(root.join(path))?;
            read_time += start.elapsed();
            let start = Instant::now();
            let tree = parser.parse(&bytes, None).ok_or(SyntaxError::NoTree)?;
            parse_time += start.elapsed();
            let start = Instant::now();
            let facts = syntax::inspect(&tree);
            inspect_time += start.elapsed();
            let fingerprint = blake3::hash(&bytes);
            if pass == 0 {
                error_files += usize::from(facts.errors > 0 || facts.missing > 0);
                totals.nodes += facts.nodes;
                totals.named += facts.named;
                totals.errors += facts.errors;
                totals.missing += facts.missing;
                baseline.insert(path.clone(), (fingerprint, bytes.len(), facts));
            } else if baseline.get(path) != Some(&(fingerprint, bytes.len(), facts)) {
                return Err("source contents or syntax counts changed between passes".into());
            }
        }
        readings.push(read_time);
        parsings.push(parse_time);
        inspections.push(inspect_time);
        eprintln!(
            "syntax pass {}/{}: {} files, {:.3}s parsing",
            pass + 1,
            samples,
            selected.len(),
            parse_time.as_secs_f64()
        );
    }
    let mut by_size: Vec<_> = baseline
        .iter()
        .map(|(p, (_, size, _))| (p.clone(), *size))
        .collect();
    by_size.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| a.0.cmp(&b.0)));
    let mut indices = vec![
        by_size.len() / 2,
        (by_size.len() - 1) * 95 / 100,
        by_size.len() - 1,
    ];
    indices.sort();
    indices.dedup();
    let mut edits = Vec::new();
    for index in indices {
        edits.extend(measure_edits(
            language,
            &root,
            &by_size[index].0,
            samples * 20,
        )?);
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "experiment":"syntax_parsing", "source_root":root, "language":language,
            "matching_files":matching, "measured_files":selected.len(),
            "content_bytes":baseline.values().map(|(_,size,_)| *size as u64).sum::<u64>(),
            "syntax_counts":totals, "files_with_syntax_recovery":error_files,
            "per_pass": {"file_reads":timing::summarize(&readings),"full_parsing":timing::summarize(&parsings),"node_inspection":timing::summarize(&inspections)},
            "incremental_edits":edits,
            "conditions": {
                "build":if cfg!(debug_assertions) {"debug"} else {"release"},
                "selection":"sorted path prefix when a file limit is supplied; otherwise all matching files",
                "parsing":"sequential, raw unpreprocessed source; parser reused; trees dropped between files",
                "inspection":"all nodes counted, including punctuation, error nodes, and missing nodes",
                "coverage":"syntax only; no name resolution, cross-language links, or build configuration",
                "edits":"median, approximately 95th-percentile, and largest files by bytes; duplicates removed",
                "validation":"full and incremental preorder node shapes, fields, flags, depths, and byte ranges compared outside timed phases"
            }
        }))?
    );
    Ok(())
}

fn measure_edits(
    language: SourceLanguage,
    root: &Path,
    path: &Path,
    samples: usize,
) -> Result<Vec<Value>> {
    let bytes = fs::read(root.join(path))?;
    let requested_samples = samples;
    let samples = samples.min((32 * 1024 * 1024 / bytes.len().max(1)).max(3));
    let mut incremental = language.parser()?;
    let mut full = language.parser()?;
    let tree = incremental.parse(&bytes, None).ok_or(SyntaxError::NoTree)?;
    let before = syntax::shape(&tree);
    let comment = match language {
        SourceLanguage::Python => b"\n# benchmark edit\n".as_slice(),
        _ => b"\n// benchmark edit\n".as_slice(),
    };
    let mut cases = vec![
        ("append_comment", bytes.len(), bytes.len(), comment.to_vec()),
        ("prepend_comment", 0, 0, comment.to_vec()),
    ];
    if let Some(index) = (bytes.len() / 2..bytes.len())
        .chain(0..bytes.len() / 2)
        .find(|i| bytes[*i].is_ascii_alphabetic())
    {
        cases.push((
            "replace_middle_ascii_byte",
            index,
            index + 1,
            vec![if bytes[index] == b'x' { b'y' } else { b'x' }],
        ));
    }
    let mut reports = Vec::new();
    for (name, start, end, replacement) in cases {
        eprintln!(
            "incremental {name}: {} ({} bytes, {samples} samples)",
            path.display(),
            bytes.len()
        );
        let mut preparations = Vec::new();
        let mut reparses = Vec::new();
        let mut full_parses = Vec::new();
        let mut recovery = false;
        for iteration in 0..samples {
            let begun = Instant::now();
            let prepared = syntax::prepare_edit(&bytes, start, end, &replacement)?;
            preparations.push(begun.elapsed());
            let ((updated, incremental_time), (fresh, full_time)) = if iteration % 2 == 0 {
                let begun = Instant::now();
                let updated = syntax::reparse(&mut incremental, &tree, &prepared)?;
                let elapsed = begun.elapsed();
                let begun = Instant::now();
                let fresh = full
                    .parse(&prepared.source, None)
                    .ok_or(SyntaxError::NoTree)?;
                let full_time = begun.elapsed();
                ((updated, elapsed), (fresh, full_time))
            } else {
                let begun = Instant::now();
                let fresh = full
                    .parse(&prepared.source, None)
                    .ok_or(SyntaxError::NoTree)?;
                let full_time = begun.elapsed();
                let begun = Instant::now();
                let updated = syntax::reparse(&mut incremental, &tree, &prepared)?;
                let elapsed = begun.elapsed();
                ((updated, elapsed), (fresh, full_time))
            };
            reparses.push(incremental_time);
            full_parses.push(full_time);
            if syntax::shape(&updated) != syntax::shape(&fresh) || syntax::shape(&tree) != before {
                return Err(
                    "incremental parse diverged from full parse or changed the prior tree".into(),
                );
            }
            recovery |= fresh.root_node().has_error();
        }
        reports.push(json!({"path":path,"source_bytes":bytes.len(),"edit":name,"edit_start_byte":start,
            "requested_samples":requested_samples,"sampling":"capped by 32 MiB of source per case, with at least three repetitions",
            "preparation":timing::summarize(&preparations),"incremental_parse":timing::summarize(&reparses),
            "full_parse_of_edited_buffer":timing::summarize(&full_parses),"syntax_recovery_present":recovery,"validation_passed":true}));
    }
    Ok(reports)
}
