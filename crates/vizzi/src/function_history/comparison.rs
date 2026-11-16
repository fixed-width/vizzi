//! Owned, bounded declaration pairs, independent of live publication and review.
use super::*;
use vizzi_core::{
    model::Document,
    text_diff::{DiffInput, DiffLimits, TextEdit, compare_ranges},
};
const PAIRS: usize = 12;
const PAIR_BYTES: usize = 32 * 1024 * 1024;

/// Source selection for an adjacent function comparison.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CompareRequest {
    /// Freeze a current saved declaration and find its previous Git version.
    Current {
        /// Exact current native binding.
        source: Request,
    },
    /// Compare an accepted historical version with its next older distinct version.
    Historical {
        /// Exact accepted source binding.
        source: HistoryBinding,
    },
}
/// A historical version accepted by the native owner.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryBinding {
    /// History lifetime token.
    pub token: String,
    /// Accepted version ordinal.
    pub index: usize,
    /// Exact single-file observation.
    pub snapshot: String,
    /// Native file identity.
    pub file: String,
    /// Immutable document version.
    pub version: String,
}
/// Exact source selection for a comparison window.
#[derive(Deserialize, Serialize)]
pub struct ComparisonWindow {
    /// Pair lifetime token.
    pub token: String,
    /// Before or After capture: older or newer.
    pub side: String,
    /// Immutable observation.
    pub snapshot: String,
    /// Native file identity.
    pub file: String,
    /// Immutable document version.
    pub version: String,
}
pub(super) struct Pair {
    token: String,
    newer: Capture,
    older: Capture,
    newer_history: Option<Value>,
    older_history: Value,
    newer_label: String,
    older_label: String,
    edits: Vec<TextEdit>,
}
impl Pair {
    fn bytes(&self) -> usize {
        self.newer.snapshot.documents()[0].bytes() + self.older.snapshot.documents()[0].bytes()
    }
}
impl FunctionHistory {
    /// Freezes complete adjacent declarations and returns their first changed sections.
    pub fn compare(&self, request: &CompareRequest) -> Result<Value, CompanionError> {
        let (newer, newer_history, older, older_history) = match request {
            CompareRequest::Current { source } => {
                let previous = self.start(source)?;
                let newer = self.current_comparison_capture(source)?;
                let (older, history) = self.comparison_history(
                    previous["token"]
                        .as_str()
                        .ok_or_else(|| unavailable("History token unavailable"))?,
                    0,
                )?;
                (newer, None, older, history)
            }
            CompareRequest::Historical { source } => {
                let (newer, history) = self.comparison_history(&source.token, source.index)?;
                validate(&newer, &source.snapshot, &source.file, &source.version)?;
                let index = source
                    .index
                    .checked_add(1)
                    .ok_or_else(|| invalid("Invalid comparison history position"))?;
                self.step(&Step {
                    token: source.token.clone(),
                    index,
                })?;
                let (older, older_history) = self.comparison_history(&source.token, index)?;
                (newer, Some(history), older, older_history)
            }
        };
        let before = &older.snapshot.documents()[0];
        let after = &newer.snapshot.documents()[0];
        let edits = compare_ranges(
            DiffInput {
                document: before,
                span: older.span.clone(),
            },
            DiffInput {
                document: after,
                span: newer.span.clone(),
            },
            DiffLimits {
                max_bytes: 4 * 1024 * 1024,
                max_lines: 20_000,
                max_hunks: 1024,
                max_work: 2_000_000,
            },
            &|| false,
        )
        .map_err(unavailable)?;
        let mut random = [0; 32];
        getrandom::fill(&mut random).map_err(unavailable)?;
        let token = random.iter().map(|b| format!("{b:02x}")).collect();
        let newer_label = newer_history
            .as_ref()
            .and_then(|h| h["commit"].as_str())
            .unwrap_or("Saved working tree (captured)")
            .to_owned();
        let older_label = older_history["commit"]
            .as_str()
            .ok_or_else(|| unavailable("Historical revision unavailable"))?
            .to_owned();
        let pair = Pair {
            token,
            newer,
            older,
            newer_history,
            older_history,
            newer_label,
            older_label,
            edits,
        };
        let answer = json!({"token":pair.token,"newer":frame(&pair,"newer")?,"older":frame(&pair,"older")?,"edits":pair.edits});
        let mut pairs = self
            .comparisons
            .try_lock()
            .map_err(|_| unavailable("Function comparison is busy; retry"))?;
        while pairs.len() >= PAIRS
            || pairs.iter().map(Pair::bytes).sum::<usize>() + pair.bytes() > PAIR_BYTES
        {
            if pairs.pop_front().is_none() {
                break;
            }
        }
        pairs.push_back(pair);
        Ok(answer)
    }
    /// Pages one owned comparison capture with exact version checks.
    pub fn comparison_window(
        &self,
        binding: &ComparisonWindow,
        query: &WindowQuery,
    ) -> Result<Value, CompanionError> {
        self.with_comparison_capture(binding, |capture, _| {
            super::window(&capture.snapshot, &capture.snapshot.documents()[0], query)
        })
    }
    pub(super) fn with_comparison_capture(
        &self,
        binding: &ComparisonWindow,
        operation: impl FnOnce(&Capture, &str) -> Result<Value, CompanionError>,
    ) -> Result<Value, CompanionError> {
        if !matches!(binding.side.as_str(), "older" | "newer") {
            return Err(invalid("Invalid function comparison side"));
        }
        let pairs = self
            .comparisons
            .try_lock()
            .map_err(|_| unavailable("Function comparison is busy; retry"))?;
        let pair = pairs
            .iter()
            .find(|p| p.token == binding.token)
            .ok_or_else(|| conflict("Function comparison expired; return to live"))?;
        let capture = if binding.side == "older" {
            &pair.older
        } else {
            &pair.newer
        };
        validate(capture, &binding.snapshot, &binding.file, &binding.version)?;
        let label = if binding.side == "older" {
            &pair.older_label
        } else {
            &pair.newer_label
        };
        operation(capture, label)
    }
    fn comparison_history(
        &self,
        token: &str,
        index: usize,
    ) -> Result<(Capture, Value), CompanionError> {
        let sessions = self.lock()?;
        let session = sessions
            .iter()
            .find(|s| s.token == token)
            .ok_or_else(|| conflict("Function history expired; return to live"))?;
        let version = session
            .versions
            .get(index)
            .ok_or_else(|| invalid("Invalid comparison history position"))?;
        let (capture, _) = self.capture(session, version.candidate)?;
        if capture.snapshot.id().as_str() != version.snapshot {
            return Err(conflict("Historical comparison source differs"));
        }
        Ok((
            capture,
            json!({"token":session.token,"tip":session.tip,"commit":session.commits[version.candidate],"index":index}),
        ))
    }
    fn current_comparison_capture(&self, request: &Request) -> Result<Capture, CompanionError> {
        let snapshot = self
            .host
            .application()
            .snapshot()
            .ok_or_else(|| unavailable("Wait for current source"))?;
        if snapshot.id().as_str() != request.snapshot {
            return Err(conflict("Source changed before comparison"));
        }
        let doc = snapshot
            .file(&request.file)
            .filter(|d| d.version().as_str() == request.version)
            .ok_or_else(|| conflict("Comparison source version differs"))?;
        if doc.bytes() > FILE_BYTES {
            return Err(unavailable("Comparison file exceeds 2 MiB"));
        }
        let mut found = doc.analyses().iter().flat_map(|a| {
            a.facts
                .iter()
                .filter(|f| {
                    f.unique
                        && f.kind == "function"
                        && doc.entity_id(a, f).as_str() == request.tracking
                })
                .map(move |f| (a, f))
        });
        let (analysis, fact) = found
            .next()
            .ok_or_else(|| invalid("Comparison needs an unambiguous function"))?;
        if found.next().is_some() {
            return Err(invalid("Comparison declaration is ambiguous"));
        }
        let tracking = doc.entity_id(analysis, fact);
        let span = fact.span.clone();
        let frozen = Arc::new(
            Snapshot::new(
                snapshot.workspace().clone(),
                vec![doc.clone()],
                CapturePolicyId::derive(&[b"function-comparison-current-v1"]),
            )
            .map_err(unavailable)?,
        );
        let symbol = Structure::new(&frozen)
            .resolve(Some(tracking.as_str()), "")
            .cloned()
            .ok_or_else(|| unavailable("Comparison declaration unavailable"))?;
        Ok(Capture {
            snapshot: frozen,
            symbol,
            span,
        })
    }
}
fn validate(
    capture: &Capture,
    snapshot: &str,
    file: &str,
    version: &str,
) -> Result<(), CompanionError> {
    let s = &capture.symbol.source;
    if capture.snapshot.id().as_str() != snapshot || s.file != file || s.version != version {
        return Err(conflict("Function comparison source version differs"));
    }
    Ok(())
}
fn frame(pair: &Pair, side: &str) -> Result<Value, CompanionError> {
    let (capture, history, label, other_label) = if side == "older" {
        (
            &pair.older,
            Some(&pair.older_history),
            &pair.older_label,
            &pair.newer_label,
        )
    } else {
        (
            &pair.newer,
            pair.newer_history.as_ref(),
            &pair.newer_label,
            &pair.older_label,
        )
    };
    let changes: Vec<_> = pair
        .edits
        .iter()
        .map(|e| {
            let r = if side == "older" {
                &e.base.lines
            } else {
                &e.working.lines
            };
            [r.start + 1, r.end + 1]
        })
        .collect();
    let s = &capture.symbol.source;
    let start = changes.first().map_or(s.line, |c| {
        c[0].saturating_sub(3).max(s.line).min(s.end_line)
    });
    let doc: &Document = &capture.snapshot.documents()[0];
    let source = super::window(
        &capture.snapshot,
        doc,
        &WindowQuery {
            line_start: Some(start),
            line_count: Some(96.min(s.end_line - start + 1)),
            outline_count: Some(1),
            reflow: Some(true),
            ..WindowQuery::default()
        },
    )?;
    let origin = super::window(
        &capture.snapshot,
        doc,
        &WindowQuery {
            line_start: Some(s.line),
            line_count: Some(1),
            outline_count: Some(1),
            ..WindowQuery::default()
        },
    )?;
    Ok(
        json!({"snapshot":capture.snapshot.id().as_str(),"symbol":capture.symbol,"history":history,"label":label,"other_label":other_label,"changes":changes,"origin":origin["lines"][0],"window":source}),
    )
}
