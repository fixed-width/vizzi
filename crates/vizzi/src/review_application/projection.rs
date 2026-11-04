//! Renderer-independent finite review projections over a held lease.

use super::*;
use vizzi_core::{
    comparison::{
        ChangeKind, ComparedEntry, EntryContent, EntryKind, EntryState, Side, Unavailability,
    },
    model::Coverage,
    policy::{Classification, Provenance},
    projection::{Window, WindowQuery},
    text_diff::{TextEdit, TextHunk},
};

/// A held pair plus labels acquired atomically with its publication token.
pub struct ComparisonQuery {
    pub(super) lease: ReviewLease,
    pub(super) metadata: ComparisonMetadata,
}

/// Stable page/filter request. A projection filter never changes review capture scope.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct ReviewPageQuery {
    /// Optional opaque selection, resolved independently of filtering and paging.
    pub file: Option<String>,
    /// Zero-based matched-row offset.
    pub offset: Option<usize>,
    /// Requested rows, clamped to 1..200.
    pub count: Option<usize>,
    /// Case-insensitive literal display-path substring, at most 256 bytes.
    pub filter: Option<String>,
    /// Whether unchanged context is included; default false.
    pub unchanged: Option<bool>,
}

/// Finite changed-file/context inventory result.
#[derive(Debug, Serialize)]
pub struct ReviewFiles {
    /// Wire schema.
    pub schema: u8,
    /// Pair labels.
    #[serde(flatten)]
    pub comparison: ComparisonMetadata,
    /// Bounded rows.
    pub files: Vec<ReviewFile>,
    /// Requested inventory row, including outside the page/filter; null when absent.
    pub selected: Option<ReviewFile>,
    /// Matched rows across the entire inventory.
    pub total: usize,
    /// Next offset, absent at the end.
    pub next: Option<usize>,
    /// Returned matched-row offset.
    pub offset: usize,
    /// Matched rows after this page.
    pub remaining: usize,
    /// Full-scope counts, unaffected by the projection filter.
    pub totals: ReviewTotals,
}

/// Full inventory status counts; unavailable content overlaps path statuses.
#[derive(Debug, Default, Serialize)]
pub struct ReviewTotals {
    /// All in-scope paths in either observation.
    pub paths: usize,
    /// Verified additions.
    pub added: usize,
    /// Verified removals.
    pub removed: usize,
    /// Content/type/mode modifications.
    pub modified: usize,
    /// Verified equal paths.
    pub unchanged: usize,
    /// Paths whose equality/change cannot be established.
    pub unassessed: usize,
    /// Paths with an unavailable content or presence observation on either side.
    pub unavailable: usize,
}

/// One source-control-independent inventory row.
#[derive(Debug, Serialize)]
pub struct ReviewFile {
    /// Opaque workspace file identity.
    pub file: String,
    /// Escaped display path, at most 512 UTF-8 bytes.
    pub path: String,
    /// Whether the displayed path has further bytes.
    pub path_truncated: bool,
    /// Added/removed/modified/unchanged/unassessed.
    pub change: String,
    /// Base document pin, absent when there is no retained document.
    pub base_version: Option<String>,
    /// Working document pin, absent when there is no retained document.
    pub working_version: Option<String>,
    /// Base presence/type/content summary; absent side is null.
    pub base: Option<ReviewEntry>,
    /// Working presence/type/content summary; absent side is null.
    pub working: Option<ReviewEntry>,
}

/// Bounded inventory entry evidence independent of parser availability.
#[derive(Debug, Serialize)]
pub struct ReviewEntry {
    /// `present` or `unobserved`.
    pub state: &'static str,
    /// Observed entry kind.
    pub kind: Option<&'static str>,
    /// Executable bit for regular files.
    pub executable: Option<bool>,
    /// Verified byte length.
    pub bytes: Option<usize>,
    /// Verified content identity.
    pub content: Option<String>,
    /// Captured availability reason.
    pub unavailable: Option<&'static str>,
    /// Bounded escaped symlink target/reference bytes, never followed.
    pub literal: Option<String>,
    /// Whether the literal display has further bytes.
    pub literal_truncated: bool,
}

/// One bounded analysis descriptor, without unbounded facts.
#[derive(Debug, Serialize)]
pub struct ReviewContext {
    /// Registered context identity.
    pub context: String,
    /// Bounded language label.
    pub language: String,
    /// Bounded descriptor.
    pub description: String,
    /// Syntax/deferred/unavailable coverage.
    pub coverage: Coverage,
    /// Complete declaration count, separately paged through the window.
    pub declarations: usize,
    /// Up to three bounded diagnostic strings.
    pub diagnostics: Vec<String>,
    /// Additional diagnostic count.
    pub hidden_diagnostics: usize,
    /// Whether any displayed descriptor/diagnostic was clipped.
    pub truncated: bool,
}

/// A source/outline window bound to one publication, side and document version.
#[derive(Debug, Serialize)]
pub struct ReviewWindow {
    /// Wire schema.
    pub schema: u8,
    /// Publication token.
    pub token: String,
    /// Portable pair identity.
    pub comparison: String,
    /// Explicit immutable side.
    pub side: String,
    /// Existing finite document projection.
    pub window: Window,
    /// Source assignment with bounded text fields.
    pub classification: Classification,
    /// Whether assignment text was clipped.
    pub classification_truncated: bool,
    /// Up to four context descriptors.
    pub contexts: Vec<ReviewContext>,
    /// Additional context count.
    pub hidden_contexts: usize,
}

/// Independent bounded hunk/edit pages for a selected immutable file pair.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct ReviewDiffQuery {
    /// Zero-based first hunk.
    pub hunk_start: Option<usize>,
    /// Hunk count, clamped to 1..200.
    pub hunk_count: Option<usize>,
    /// Zero-based first edit.
    pub edit_start: Option<usize>,
    /// Edit count, clamped to 1..200.
    pub edit_count: Option<usize>,
}

/// Complete diff counts with finite metadata pages; source remains in pinned windows.
#[derive(Debug, Serialize)]
pub struct ReviewDiff {
    /// Wire schema.
    pub schema: u8,
    /// Pair labels.
    #[serde(flatten)]
    pub comparison: ComparisonMetadata,
    /// Opaque selected file.
    pub file: String,
    /// Base pin, null only for absent/unretained sides.
    pub base_version: Option<String>,
    /// Working pin, null only for absent/unretained sides.
    pub working_version: Option<String>,
    /// All computed hunks.
    pub hunk_total: usize,
    /// All computed edits.
    pub edit_total: usize,
    /// At most 200 contextual hunks.
    pub hunks: Vec<TextHunk>,
    /// At most 200 exact edits.
    pub edits: Vec<TextEdit>,
    /// Next hunk offset.
    pub next_hunk: Option<usize>,
    /// Next edit offset.
    pub next_edit: Option<usize>,
    /// Hidden hunks after this page.
    pub remaining_hunks: usize,
    /// Hidden edits after this page.
    pub remaining_edits: usize,
}

impl ComparisonQuery {
    /// Returns fixed labels without querying the live workspace.
    pub fn metadata(&self) -> &ComparisonMetadata {
        &self.metadata
    }
    /// Projects a stable page with explicit hidden/remaining metadata.
    pub fn files(&self, query: &ReviewPageQuery) -> Result<ReviewFiles, ReviewApiError> {
        let filter = query.filter.as_deref().unwrap_or("");
        if filter.len() > 256 {
            return Err(ReviewApiError::Invalid("filter exceeds 256 bytes".into()));
        }
        let filter = filter.to_lowercase();
        let offset = query.offset.unwrap_or(0);
        let count = query.count.unwrap_or(200).clamp(1, 200);
        let mut totals = ReviewTotals::default();
        let mut total = 0;
        let mut files = vec![];
        for entry in self.lease.entries() {
            totals.paths += 1;
            match entry.change {
                ChangeKind::Added => totals.added += 1,
                ChangeKind::Removed => totals.removed += 1,
                ChangeKind::Modified => totals.modified += 1,
                ChangeKind::Unchanged => totals.unchanged += 1,
                ChangeKind::Unassessed => totals.unassessed += 1,
            };
            totals.unavailable += usize::from(
                [entry.base, entry.working]
                    .into_iter()
                    .flatten()
                    .any(unavailable),
            );
            if (!query.unchanged.unwrap_or(false) && entry.change == ChangeKind::Unchanged)
                || (!filter.is_empty() && !entry.path.display().to_lowercase().contains(&filter))
            {
                continue;
            }
            if total >= offset && files.len() < count {
                files.push(self.file(entry));
            }
            total += 1;
        }
        let end = offset.saturating_add(files.len()).min(total);
        Ok(ReviewFiles {
            schema: 1,
            comparison: self.metadata.clone(),
            files,
            selected: query
                .file
                .as_deref()
                .and_then(|file| self.lease.entry(file))
                .map(|entry| self.file(entry)),
            total,
            next: (end < total).then_some(end),
            offset: offset.min(total),
            remaining: total - end,
            totals,
        })
    }
    fn file(&self, entry: ComparedEntry<'_>) -> ReviewFile {
        let (path, path_truncated) = clip(&entry.path.display(), 512);
        ReviewFile {
            file: self.lease.file_id(entry.path).as_str().into(),
            path,
            path_truncated,
            change: change(entry.change).into(),
            base_version: self
                .lease
                .document_version(Side::Base, entry.path)
                .map(|v| v.as_str().into()),
            working_version: self
                .lease
                .document_version(Side::Working, entry.path)
                .map(|v| v.as_str().into()),
            base: entry
                .base
                .map(|state| summary(state, self.lease.literal(Side::Base, entry.path))),
            working: entry
                .working
                .map(|state| summary(state, self.lease.literal(Side::Working, entry.path))),
        }
    }
    /// Validates token-derived file, side and document version before source IO.
    pub fn window(
        &self,
        file: &str,
        side: Side,
        version: &str,
        query: &WindowQuery,
    ) -> Result<ReviewWindow, ReviewApiError> {
        let path = self.lease.path(file).ok_or(ReviewApiError::NotFound)?;
        let actual = self
            .lease
            .document_version(side, path)
            .ok_or(ReviewApiError::NotFound)?;
        if actual.as_str() != version {
            return Err(ReviewApiError::Version);
        }
        let (classification, classification_truncated) =
            classification(self.lease.classification(side, path)?.clone());
        let analyses = self.lease.analyses(side, path)?;
        let mut contexts = vec![];
        for analysis in analyses.iter().take(4) {
            let (language, a) = clip(&analysis.context.language, 32);
            let (description, b) = clip(&analysis.context.description, 512);
            let mut truncated = a || b;
            let diagnostics = analysis
                .diagnostics
                .iter()
                .take(3)
                .map(|text| {
                    let (text, clipped) = clip(text, 512);
                    truncated |= clipped;
                    text
                })
                .collect();
            contexts.push(ReviewContext {
                context: analysis.context.id.as_str().into(),
                language,
                description,
                coverage: analysis.coverage.clone(),
                declarations: analysis.facts.len(),
                diagnostics,
                hidden_diagnostics: analysis.diagnostics.len().saturating_sub(3),
                truncated,
            });
        }
        Ok(ReviewWindow {
            schema: 1,
            token: self.metadata.token.clone(),
            comparison: self.metadata.comparison.clone(),
            side: side_name(side).into(),
            window: self.lease.window(side, path, query)?,
            classification,
            classification_truncated,
            contexts,
            hidden_contexts: analyses.len().saturating_sub(4),
        })
    }
    /// Validates both side versions before computing an immutable diff.
    pub fn diff(
        &self,
        file: &str,
        base: Option<&str>,
        working: Option<&str>,
        query: &ReviewDiffQuery,
    ) -> Result<ReviewDiff, ReviewApiError> {
        let path = self.lease.path(file).ok_or(ReviewApiError::NotFound)?;
        let base_version = self
            .lease
            .document_version(Side::Base, path)
            .map(|v| v.as_str());
        let working_version = self
            .lease
            .document_version(Side::Working, path)
            .map(|v| v.as_str());
        if base != base_version || working != working_version {
            return Err(ReviewApiError::Version);
        }
        let result =
            self.lease
                .text_diff(path, vizzi_core::text_diff::DiffLimits::default(), &|| {
                    false
                })?;
        let (hunk_start, hunk_end) = page(query.hunk_start, query.hunk_count, result.hunks.len());
        let (edit_start, edit_end) = page(query.edit_start, query.edit_count, result.edits.len());
        Ok(ReviewDiff {
            schema: 1,
            comparison: self.metadata.clone(),
            file: file.into(),
            base_version: base_version.map(str::to_owned),
            working_version: working_version.map(str::to_owned),
            hunk_total: result.hunks.len(),
            edit_total: result.edits.len(),
            hunks: result.hunks[hunk_start..hunk_end].to_vec(),
            edits: result.edits[edit_start..edit_end].to_vec(),
            next_hunk: (hunk_end < result.hunks.len()).then_some(hunk_end),
            next_edit: (edit_end < result.edits.len()).then_some(edit_end),
            remaining_hunks: result.hunks.len() - hunk_end,
            remaining_edits: result.edits.len() - edit_end,
        })
    }
}

fn page(start: Option<usize>, count: Option<usize>, total: usize) -> (usize, usize) {
    let start = start.unwrap_or(0).min(total);
    (
        start,
        start
            .saturating_add(count.unwrap_or(200).clamp(1, 200))
            .min(total),
    )
}
fn side_name(side: Side) -> &'static str {
    match side {
        Side::Base => "base",
        Side::Working => "working",
    }
}
fn change(kind: ChangeKind) -> &'static str {
    match kind {
        ChangeKind::Added => "added",
        ChangeKind::Removed => "removed",
        ChangeKind::Modified => "modified",
        ChangeKind::Unchanged => "unchanged",
        ChangeKind::Unassessed => "unassessed",
    }
}
fn reason(reason: Unavailability) -> &'static str {
    match reason {
        Unavailability::Excluded => "excluded",
        Unavailability::Unreadable => "unreadable",
        Unavailability::Sparse => "sparse",
        Unavailability::UnsupportedPath => "unsupported_path",
        Unavailability::UnsupportedKind => "unsupported_kind",
        Unavailability::Changing => "changing",
    }
}
fn unavailable(state: &EntryState) -> bool {
    matches!(
        state,
        EntryState::Unobserved(_)
            | EntryState::Present {
                content: EntryContent::Unavailable(_),
                ..
            }
    )
}
fn summary(state: &EntryState, literal: Option<&[u8]>) -> ReviewEntry {
    let literal = literal.map(|bytes| {
        bytes
            .escape_ascii()
            .take(513)
            .map(char::from)
            .collect::<String>()
    });
    let (literal, literal_truncated) = literal
        .map(|value| {
            let (value, truncated) = clip(&value, 512);
            (Some(value), truncated)
        })
        .unwrap_or((None, false));
    let mut row = ReviewEntry {
        state: "unobserved",
        kind: None,
        executable: None,
        bytes: None,
        content: None,
        unavailable: None,
        literal,
        literal_truncated,
    };
    match state {
        EntryState::Unobserved(why) => row.unavailable = Some(reason(*why)),
        EntryState::Present { kind, content } => {
            row.state = "present";
            row.kind = Some(match kind {
                EntryKind::Regular { executable } => {
                    row.executable = Some(*executable);
                    "regular"
                }
                EntryKind::Symlink => "symlink",
                EntryKind::Reference => "reference",
                EntryKind::Special => "special",
            });
            match content {
                EntryContent::Verified { id, bytes } => {
                    row.bytes = Some(*bytes);
                    row.content = Some(id.as_str().into());
                }
                EntryContent::Unavailable(why) => row.unavailable = Some(reason(*why)),
            }
        }
    };
    row
}
fn classification(mut value: Classification) -> (Classification, bool) {
    let mut truncated = false;
    let provenance = match &mut value {
        Classification::Assigned {
            language,
            provenance,
        } => {
            let (text, clipped) = clip(language, 32);
            *language = text;
            truncated |= clipped;
            provenance
        }
        Classification::SourceOnly {
            language,
            provenance,
            ..
        } => {
            if let Some(value) = language {
                let (text, clipped) = clip(value, 32);
                *value = text;
                truncated |= clipped;
            }
            provenance
        }
    };
    let text = match provenance {
        Provenance::Extension { extension } => extension,
        Provenance::ExplicitScope { scope } => scope,
    };
    let (bounded, clipped) = clip(text, 256);
    *text = bounded;
    truncated |= clipped;
    (value, truncated)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn assignment_text_limits_preserve_structure_and_utf8_boundaries() {
        let (value, truncated) = classification(Classification::Assigned {
            language: "λ".repeat(100),
            provenance: Provenance::ExplicitScope {
                scope: "🙂".repeat(1000),
            },
        });
        assert!(truncated);
        match value {
            Classification::Assigned {
                language,
                provenance: Provenance::ExplicitScope { scope },
            } => {
                assert_eq!(language.len(), 32);
                assert_eq!(scope.len(), 256);
            }
            _ => panic!("classification changed kind"),
        }
        let original = Classification::SourceOnly {
            language: Some("cpp".into()),
            provenance: Provenance::Extension {
                extension: "h".into(),
            },
            reason: vizzi_core::policy::SourceOnlyReason::LanguageUnassigned,
        };
        let (bounded, truncated) = classification(original.clone());
        assert!(!truncated);
        assert_eq!(bounded, original);
        for value in [
            Classification::Assigned {
                language: "λ".repeat(100),
                provenance: Provenance::Extension {
                    extension: "rs".into(),
                },
            },
            Classification::SourceOnly {
                language: Some("λ".repeat(100)),
                provenance: Provenance::Extension {
                    extension: "h".into(),
                },
                reason: vizzi_core::policy::SourceOnlyReason::LanguageUnassigned,
            },
            Classification::SourceOnly {
                language: None,
                provenance: Provenance::ExplicitScope {
                    scope: "🙂".repeat(1000),
                },
                reason: vizzi_core::policy::SourceOnlyReason::OutsideAnalysisScope,
            },
        ] {
            assert!(
                classification(value).1,
                "each clipped field must report truncation independently"
            );
        }
    }
    #[test]
    fn entry_summaries_distinguish_unavailable_from_absent_and_clip_literal_displays() {
        let value = summary(&EntryState::Unobserved(Unavailability::Excluded), None);
        assert_eq!(value.state, "unobserved");
        assert_eq!(value.unavailable, Some("excluded"));
        assert_eq!(value.kind, None);
        let value = summary(
            &EntryState::Present {
                kind: EntryKind::Reference,
                content: EntryContent::Unavailable(Unavailability::Unreadable),
            },
            None,
        );
        assert_eq!(value.state, "present");
        assert_eq!(value.kind, Some("reference"));
        assert_eq!(value.unavailable, Some("unreadable"));
        assert_eq!(value.bytes, None);
        let bytes = vec![0xff; 200];
        let value = summary(
            &EntryState::Present {
                kind: EntryKind::Symlink,
                content: EntryContent::verified(&bytes),
            },
            Some(&bytes),
        );
        assert_eq!(value.literal.as_ref().unwrap().len(), 512);
        assert!(value.literal_truncated);
        assert_eq!(value.bytes, Some(200));
    }
}
