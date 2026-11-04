//! Diagnostic native source-only Git capture; no host publication or language analysis.

use std::{error::Error, path::Path};
use vizzi::{
    review::ReviewStore,
    review_source::{CaptureLimits, CapturePolicy, GitRepository},
};
use vizzi_core::{
    comparison::{ChangeKind, EntryContent, EntryState, ReviewPath, Side},
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
    let store = ReviewStore::open(limits.max_bytes)?;
    let ticket = store.request()?;
    let mut candidate = store.begin(&ticket, None)?;
    candidate.capture_base(&mut baseline, &policy, limits, &|| false)?;
    candidate.capture_working(&mut repository.working(&baseline)?, limits, &|| false)?;
    let token = candidate.publish()?;
    let pair = store.query(&token)?;
    let notices:Vec<_>=pair.notices(Side::Working).iter().map(|notice|serde_json::json!({"path":notice.path.display(),"kind":format!("{:?}",notice.kind)})).collect();
    let mut base_paths = 0;
    let mut working_paths = 0;
    let mut acquired_bytes = 0u64;
    for entry in pair.entries() {
        base_paths += usize::from(entry.base.is_some());
        working_paths += usize::from(entry.working.is_some());
        for state in [entry.base, entry.working].into_iter().flatten() {
            if let EntryState::Present {
                content: EntryContent::Verified { bytes, .. },
                ..
            } = state
            {
                acquired_bytes += *bytes as u64;
            }
        }
    }
    let accounting = store.accounting().stats();
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
            "schema":1,"baseline":baseline.resolved(),"comparison":pair.comparison_id().as_str(),"workspace":repository.workspace().as_str(),"token":token,
            "analysis":"not_requested","base_paths":base_paths,"working_paths":working_paths,
            "acquired_bytes":acquired_bytes,"retained_source_bytes":accounting.retained_bytes,"charged_source_bytes":accounting.charged_bytes,"pair_slots":accounting.pair_slots,"changes":changes,"hidden_changes":changed.saturating_sub(200),
            "notices":notices,"selected_path":path.display(),"base_present":diff.base.version.is_some(),"working_present":diff.working.version.is_some(),"hunks":hunks,
        }))?
    );
    Ok(())
}
