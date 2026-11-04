//! Diagnostic native source-only Git capture; no host publication or language analysis.

use std::{error::Error, path::Path};
use vizzi::review_source::{CaptureLimits, CapturePolicy, GitRepository, acquire};
use vizzi_core::{
    comparison::{ChangeKind, Comparison, ReviewPath},
    text_diff::DiffLimits,
};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: review_capture ROOT REVISION LOGICAL_FILE".into());
    }
    let revision = args[1].to_str().ok_or("expected a UTF-8 revision")?;
    let path = ReviewPath::parse(
        args[2]
            .to_str()
            .ok_or("expected a UTF-8 logical path")?
            .as_bytes()
            .to_vec(),
    )?;
    let repository = GitRepository::open(Path::new(&args[0]))?;
    let limits = CaptureLimits::default();
    let policy = CapturePolicy::scope(None, vec![]);
    let mut baseline = repository.pin(revision, limits, &|| false)?;
    let base = acquire(&mut baseline, &policy, limits, &|| false)?;
    let working = acquire(
        &mut repository.working(&baseline)?,
        &policy,
        CaptureLimits {
            max_bytes: limits.max_bytes - base.bytes,
            ..limits
        },
        &|| false,
    )?;
    let notices:Vec<_>=working.notices.iter().map(|notice|serde_json::json!({"path":notice.path.display(),"kind":format!("{:?}",notice.kind)})).collect();
    let pair = Comparison::between(base.observation, working.observation)?;
    let changed = pair
        .entries()
        .filter(|entry| entry.change != ChangeKind::Unchanged)
        .count();
    let changes:Vec<_>=pair.entries().filter(|entry|entry.change!=ChangeKind::Unchanged).take(200).map(|entry|serde_json::json!({"path":entry.path.display(),"change":format!("{:?}",entry.change)})).collect();
    let diff = pair.text_diff(&path, DiffLimits::default(), &|| false)?;
    let range = |lines: &std::ops::Range<usize>| {
        format!(
            "{},{}",
            if lines.is_empty() {
                lines.start
            } else {
                lines.start + 1
            },
            lines.len()
        )
    };
    let hunks:Vec<_>=diff.hunks.iter().map(|hunk|serde_json::json!({"id":hunk.id.as_str(),"header":format!("@@ -{} +{} @@",range(&hunk.base.lines),range(&hunk.working.lines))})).collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "schema":1,"baseline":baseline.resolved(),"comparison":pair.id().as_str(),"workspace":repository.workspace().as_str(),
            "analysis":"not_requested","base_paths":pair.observation(vizzi_core::comparison::Side::Base).inventory().len(),
            "working_paths":pair.observation(vizzi_core::comparison::Side::Working).inventory().len(),
            "acquired_bytes":base.bytes+working.bytes,"changes":changes,"hidden_changes":changed.saturating_sub(200),
            "notices":notices,"selected_path":path.display(),"base_present":diff.base.version.is_some(),"working_present":diff.working.version.is_some(),"hunks":hunks,
        }))?
    );
    Ok(())
}
