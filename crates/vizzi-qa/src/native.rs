//! Explicit source/documentation capture through the existing no-follow directory seam.
use crate::{Error, SourceFile};
use serde::Serialize;
use std::path::Path;
use vizzi::review_source::{CaptureLimits, DirectoryInput, ReadEntry, ReviewInput};
use vizzi_core::comparison::EntryKind;
/// Acquisition counts, including omissions outside the experiment's source scope.
#[derive(Debug, Serialize)]
pub struct CaptureStats {
    /// Enumerated paths before applying the documented experiment scope.
    pub candidates: usize,
    /// Paths omitted by scope, encoding, or nonregular type.
    pub omitted: usize,
    /// UTF-8 regular files selected for the corpus.
    pub selected: usize,
}
/// Captures configured text source and documentation without project execution.
pub fn capture(root: &Path) -> Result<(Vec<SourceFile>, CaptureStats), Error> {
    capture_with_limits(root, pilot_limits())
}
/// Published pilot acquisition ceilings, independent of model/context budgets.
pub fn pilot_limits() -> CaptureLimits {
    CaptureLimits {
        max_paths: 100_000,
        max_file_bytes: 2 * 1024 * 1024,
        max_bytes: 32 * 1024 * 1024,
        ..Default::default()
    }
}
/// Captures the same scope under explicit acquisition ceilings, useful for resource-bound checks.
pub fn capture_with_limits(
    root: &Path,
    limits: CaptureLimits,
) -> Result<(Vec<SourceFile>, CaptureStats), Error> {
    let mut input = DirectoryInput::open(root).map_err(|e| Error(e.to_string()))?;
    let paths = input
        .paths(limits, &|| false)
        .map_err(|e| Error(e.to_string()))?;
    let mut stats = CaptureStats {
        candidates: paths.len(),
        omitted: 0,
        selected: 0,
    };
    let mut files = vec![];
    let mut total_bytes = 0;
    for path in paths {
        let Some(name) = path.document_path() else {
            stats.omitted += 1;
            continue;
        };
        let components: Vec<_> = name.split('/').collect();
        let excluded = components.iter().any(|p| {
            [
                "target",
                "node_modules",
                ".git",
                "__pycache__",
                "results",
                "facilitator",
            ]
            .contains(p)
        }) || name.starts_with("experiments/qa/");
        let extension = Path::new(name)
            .extension()
            .and_then(|x| x.to_str())
            .unwrap_or("");
        let selected = [
            "rs", "py", "js", "mjs", "ts", "tsx", "html", "css", "md", "toml", "yaml", "yml",
        ]
        .contains(&extension)
            || name == "LICENSE"
            || name == "LICENSE-MIT";
        if excluded || !selected {
            stats.omitted += 1;
            continue;
        }
        let entry = input
            .read(&path, limits.max_file_bytes, &|| false)
            .map_err(|e| Error(e.to_string()))?;
        match entry {
            ReadEntry::Present {
                kind: EntryKind::Regular { .. },
                bytes,
            } => match String::from_utf8(bytes) {
                Ok(text) => {
                    total_bytes += text.len();
                    if total_bytes > limits.max_bytes {
                        return Err(Error("QA capture exceeds 32 MiB".into()));
                    }
                    files.push(SourceFile {
                        path: name.into(),
                        text,
                    });
                    stats.selected += 1;
                }
                Err(_) => stats.omitted += 1,
            },
            ReadEntry::Present { .. } => stats.omitted += 1,
            _ => {
                return Err(Error(format!(
                    "Selected source unavailable during capture: {name}"
                )));
            }
        }
    }
    Ok((files, stats))
}
