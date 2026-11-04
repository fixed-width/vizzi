//! Diagnostic: compare two supplied files through the portable model, without analysis.

use std::{error::Error, ffi::OsStr, ops::Range, path::Path, sync::Arc};
use vizzi_core::{
    comparison::{
        Comparison, EntryContent, EntryKind, EntryState, InventoryEntry, Observation, ReviewPath,
    },
    model::{CapturePolicyId, Document, Snapshot, WorkspaceId},
    policy::{Classification, Provenance, SourceOnlyReason},
    text_diff::DiffLimits,
};

fn observation(path: &ReviewPath, input: &OsStr) -> Result<Arc<Observation>, Box<dyn Error>> {
    let workspace = WorkspaceId::derive(&[b"text-diff-diagnostic"]);
    let mut documents = vec![];
    let mut entries = vec![];
    if input != "-" {
        let bytes = std::fs::read(Path::new(input))?;
        entries.push(InventoryEntry {
            path: path.clone(),
            state: EntryState::Present {
                kind: EntryKind::Regular { executable: false },
                content: EntryContent::verified(&bytes),
            },
        });
        documents.push(Arc::new(Document::capture(
            workspace.clone(),
            path.document_path()
                .ok_or("unsupported diagnostic document path")?
                .into(),
            &bytes,
            vec![],
            Classification::SourceOnly {
                language: None,
                provenance: Provenance::ExplicitScope { scope: ".".into() },
                reason: SourceOnlyReason::LanguageUnassigned,
            },
        )?));
    }
    let snapshot = Arc::new(Snapshot::new(
        workspace,
        documents,
        CapturePolicyId::derive(&[b"source-only"]),
    )?);
    Ok(Arc::new(Observation::from_inventory(
        snapshot,
        CapturePolicyId::derive(&[b"diagnostic-single-path"]),
        entries,
    )?))
}

fn unified_range(lines: &Range<usize>) -> String {
    format!(
        "{},{}",
        if lines.is_empty() {
            lines.start
        } else {
            lines.start + 1
        },
        lines.len()
    )
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: text_diff LOGICAL_PATH BASE_FILE|- WORKING_FILE|-".into());
    }
    let path = ReviewPath::parse(
        args[0]
            .to_str()
            .ok_or("expected a UTF-8 logical path")?
            .as_bytes()
            .to_vec(),
    )?;
    let pair = Comparison::between(observation(&path, &args[1])?, observation(&path, &args[2])?)?;
    let result = pair.text_diff(&path, DiffLimits::default(), &|| false)?;
    let hunks: Vec<_> = result.hunks.iter().map(|hunk| serde_json::json!({
        "id":hunk.id.as_str(),
        "header":format!("@@ -{} +{} @@", unified_range(&hunk.base.lines), unified_range(&hunk.working.lines)),
        "base_bytes":hunk.base.bytes, "working_bytes":hunk.working.bytes,
        "edits":hunk.edits,
    })).collect();
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "comparison": result.comparison.as_str(), "path":result.path.display(), "work": result.work,
            "base_present": result.base.version.is_some(), "working_present":result.working.version.is_some(),
            "edit_count":result.edits.len(), "hunks":hunks,
        }))?
    );
    Ok(())
}
