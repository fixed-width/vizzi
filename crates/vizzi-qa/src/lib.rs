//! Experimental CPU repository QA: retrieval and source binding, separate from inference.
mod compact;
mod curated;
mod discovery;
mod metadata;
pub mod native;
mod ranking;
pub use curated::SourceRange;
pub use metadata::{SourceRole, SourceScope};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    ops::Range,
    sync::Arc,
};
use vizzi_core::{analysis::Languages, comparison::ReviewPath};

const CHUNK: usize = 1800;
const STRIDE: usize = 1200;
const FILE_BYTES: usize = 2 * 1024 * 1024;
const CORPUS_BYTES: usize = 32 * 1024 * 1024;
/// Validated source input; acquisition and revision provenance belong to the native host.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceFile {
    /// Normalised repository-relative path.
    pub path: String,
    /// Literal captured UTF-8 source or documentation.
    pub text: String,
}
/// Rejected input, exhausted allowance or invalid model/source binding.
#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct Error(pub String);
/// Actual replacement work, independent of generation time.
#[derive(Debug, Serialize)]
pub struct Update {
    /// Rechunked new or content-changed files.
    pub changed: usize,
    /// Verified unchanged files whose chunks were reused.
    pub reused: usize,
    /// Removed paths.
    pub removed: usize,
    /// Complete selected file count.
    pub files: usize,
    /// Complete selected source bytes.
    pub bytes: usize,
    /// Complete derived chunk count; postings are rebuilt after replacement.
    pub chunks: usize,
}
struct IndexedFile {
    path: String,
    content: blake3::Hash,
    text: String,
    ranges: Vec<Range<usize>>,
    metadata: metadata::Metadata,
}
struct Window {
    file: Arc<IndexedFile>,
    range: Range<usize>,
    length: usize,
}
/// A bounded source excerpt with a prompt-local label and complete source identity.
#[derive(Debug, Serialize)]
pub struct Evidence {
    /// One-based prompt-local citation label.
    pub label: usize,
    /// Content/path/range-derived identity.
    pub id: String,
    /// Repository-relative path.
    pub path: String,
    /// Full captured file identity.
    pub content: String,
    /// One-based first source line.
    pub start_line: usize,
    /// Inclusive last represented line.
    pub end_line: usize,
    /// Exact literal excerpt, never normalized for matching quotes.
    pub text: String,
    /// Source-role hint from path or raw lexical scope, not a resolved semantic guarantee.
    pub role: SourceRole,
    /// Smallest captured lexical scope at the excerpt start, when available.
    pub scope: Option<SourceScope>,
    /// Basis for the source-role hint.
    pub role_basis: &'static str,
}
/// A source-bound context; callers cannot deserialize or construct unchecked contexts.
#[derive(Debug, Serialize)]
pub struct Context {
    snapshot: String,
    evidence: Vec<Evidence>,
}
impl Context {
    /// Exact source/index observation used for this context.
    pub fn snapshot(&self) -> &str {
        &self.snapshot
    }
    /// Finite excerpts, labelled in prompt order.
    pub fn evidence(&self) -> &[Evidence] {
        &self.evidence
    }
}
/// Whether a model claims to have sufficient supplied evidence.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    /// An answer requiring at least one valid source reference.
    Answered,
    /// Explicit abstention; this is not automatically correct.
    InsufficientEvidence,
}
/// One model-supplied citation; validity does not establish entailment.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Citation {
    /// Prompt-local source label.
    pub label: usize,
    /// Nonempty exact source quotation.
    pub quote: String,
}
/// Raw structured model response, with no automatic repair or semantic grading.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Answer {
    /// Model's answerability decision.
    pub status: Status,
    /// Model's answer or explanation of missing evidence.
    pub answer: String,
    /// Literal source references.
    pub citations: Vec<Citation>,
}
/// Model response using only supplied integer evidence labels.
#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct IdAnswer {
    /// Model's answerability decision.
    pub status: Status,
    /// Model's answer or explanation of missing evidence.
    pub answer: String,
    /// Prompt-local labels; source locations are resolved by the host.
    pub citations: Vec<usize>,
}
/// Canonical reference resolved from a native-held context, not model-generated text.
#[derive(Debug, Serialize)]
pub struct SourceReference<'a> {
    /// Prompt-local source label.
    pub label: usize,
    /// Source/index observation.
    pub snapshot: &'a str,
    /// Content/path/range identity of the excerpt.
    pub id: &'a str,
    /// Repository-relative source path.
    pub path: &'a str,
    /// Full captured file content identity.
    pub content: &'a str,
    /// One-based first excerpt line.
    pub start_line: usize,
    /// Inclusive last excerpt line.
    pub end_line: usize,
}
/// A content-addressed corpus and deterministic lexical search index.
pub struct Corpus {
    snapshot: String,
    files: BTreeMap<String, Arc<IndexedFile>>,
    windows: Vec<Window>,
    postings: HashMap<String, Vec<(usize, usize)>>,
    average: f64,
    languages: Languages,
}
fn require(ok: bool, message: &str) -> Result<(), Error> {
    if ok {
        Ok(())
    } else {
        Err(Error(message.into()))
    }
}
fn floor(text: &str, mut end: usize) -> usize {
    end = end.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    end
}
fn terms(text: &str) -> BTreeMap<String, usize> {
    let mut output = BTreeMap::new();
    for word in text.split(|c: char| !c.is_alphanumeric() && c != '_') {
        if word.is_empty() || word.len() > 128 {
            continue;
        }
        let lower = word.to_lowercase();
        if [
            "the", "a", "an", "is", "are", "to", "in", "of", "and", "or", "does", "do", "how",
            "what", "which", "where", "this", "that", "with", "for", "from", "be", "it", "can",
            "i", "we", "on",
        ]
        .contains(&lower.as_str())
        {
            continue;
        }
        *output.entry(lower.clone()).or_default() += 1;
        let mut start = 0;
        let mut parts = vec![];
        let mut previous_lower = false;
        for (index, c) in word.char_indices() {
            if c == '_' || (c.is_uppercase() && previous_lower) {
                if start < index {
                    parts.push(word[start..index].to_lowercase());
                }
                start = if c == '_' { index + 1 } else { index };
            }
            previous_lower = c.is_lowercase();
        }
        if start < word.len() {
            parts.push(word[start..].to_lowercase());
        }
        if !parts.is_empty() {
            for part in parts {
                if part != lower {
                    *output.entry(part).or_default() += 1;
                }
            }
        }
    }
    output
}
fn excerpt(file: &IndexedFile, range: &Range<usize>, label: usize) -> Evidence {
    let text = &file.text[range.clone()];
    let mut hash = blake3::Hasher::new();
    hash.update(file.path.as_bytes());
    hash.update(file.content.as_bytes());
    hash.update(&(range.start as u64).to_le_bytes());
    hash.update(&(range.end as u64).to_le_bytes());
    let start_line = 1 + file.text[..range.start]
        .bytes()
        .filter(|b| *b == b'\n')
        .count();
    let end_line = start_line
        + text
            .strip_suffix('\n')
            .unwrap_or(text)
            .bytes()
            .filter(|b| *b == b'\n')
            .count();
    Evidence {
        label,
        id: hash.finalize().to_hex().to_string(),
        path: file.path.clone(),
        content: file.content.to_hex().to_string(),
        start_line,
        end_line,
        text: text.into(),
        role: file.metadata.role_for(range),
        scope: file.metadata.scope(range.start).map(|s| s.label.clone()),
        role_basis: file.metadata.basis,
    }
}
impl Corpus {
    /// Creates an empty corpus using the registered raw-syntax adapters for chunk boundaries.
    pub fn new() -> Result<Self, Error> {
        Ok(Self {
            snapshot: String::new(),
            files: BTreeMap::new(),
            windows: vec![],
            postings: HashMap::new(),
            average: 1.0,
            languages: Languages::builtin().map_err(|e| Error(e.to_string()))?,
        })
    }
    /// Exact identity of the selected source and chunker configuration.
    pub fn snapshot(&self) -> &str {
        &self.snapshot
    }
    /// Atomically replaces source, reusing verified unchanged chunks and rebuilding postings.
    pub fn replace(&mut self, inputs: Vec<SourceFile>) -> Result<Update, Error> {
        require(inputs.len() <= 4096, "QA corpus exceeds 4,096 files")?;
        let mut files = BTreeMap::new();
        let (mut bytes, mut changed, mut reused) = (0, 0, 0);
        for input in inputs {
            ReviewPath::parse(input.path.as_bytes().to_vec()).map_err(|e| Error(e.to_string()))?;
            require(
                input.path.len() <= 4096 && input.text.len() <= FILE_BYTES,
                "QA path or file exceeds its allowance",
            )?;
            require(!files.contains_key(&input.path), "Duplicate source path")?;
            bytes += input.text.len();
            require(bytes <= CORPUS_BYTES, "QA corpus exceeds 32 MiB")?;
            let content = blake3::hash(input.text.as_bytes());
            if let Some(old) = self.files.get(&input.path).filter(|f| f.content == content) {
                files.insert(input.path, old.clone());
                reused += 1;
                continue;
            }
            let mut starts: BTreeSet<usize> = (0..input.text.len())
                .step_by(STRIDE)
                .map(|n| floor(&input.text, n))
                .collect();
            let language = if input.path.ends_with(".rs") {
                Some("rust")
            } else if input.path.ends_with(".py") {
                Some("python")
            } else {
                None
            };
            let mut captured_analysis = None;
            if let Some(adapter) = language.and_then(|name| self.languages.select(name)) {
                let mut analyzer = adapter.create().map_err(|e| Error(e.to_string()))?;
                let analysis = analyzer
                    .analyze(input.text.as_bytes())
                    .map_err(|e| Error(e.to_string()))?;
                for fact in analysis.facts.iter().take(4096) {
                    starts.insert(fact.span.start);
                }
                captured_analysis = Some(analysis);
            }
            let metadata =
                metadata::Metadata::new(&input.path, &input.text, captured_analysis.as_ref());
            let ranges = starts
                .into_iter()
                .filter_map(|start| {
                    let end = floor(&input.text, start.saturating_add(CHUNK));
                    (end > start).then_some(start..end)
                })
                .collect();
            files.insert(
                input.path.clone(),
                Arc::new(IndexedFile {
                    path: input.path,
                    content,
                    text: input.text,
                    ranges,
                    metadata,
                }),
            );
            changed += 1;
        }
        let removed = self
            .files
            .keys()
            .filter(|p| !files.contains_key(*p))
            .count();
        let mut hash = blake3::Hasher::new();
        hash.update(include_bytes!("lib.rs"));
        hash.update(include_bytes!("compact.rs"));
        hash.update(include_bytes!("metadata.rs"));
        hash.update(include_bytes!("curated.rs"));
        hash.update(include_bytes!("discovery.rs"));
        for name in self.languages.names() {
            if let Some(language) = self.languages.select(name) {
                hash.update(language.context().id.as_str().as_bytes());
            }
        }
        let mut windows = vec![];
        let mut postings: HashMap<String, Vec<(usize, usize)>> = HashMap::new();
        let mut total_length = 0;
        for file in files.values() {
            hash.update(&(file.path.len() as u64).to_le_bytes());
            hash.update(file.path.as_bytes());
            hash.update(file.content.as_bytes());
            for range in &file.ranges {
                require(windows.len() < 65536, "QA corpus exceeds 65,536 chunks")?;
                let mut counts = terms(&file.text[range.clone()]);
                for (term, count) in terms(&file.path) {
                    *counts.entry(term).or_default() += 2 * count;
                }
                let length = counts.values().sum::<usize>();
                total_length += length;
                for (term, count) in counts {
                    postings
                        .entry(term)
                        .or_default()
                        .push((windows.len(), count));
                }
                windows.push(Window {
                    file: file.clone(),
                    range: range.clone(),
                    length,
                });
            }
        }
        let stats = Update {
            changed,
            reused,
            removed,
            files: files.len(),
            bytes,
            chunks: windows.len(),
        };
        self.average = (total_length as f64 / windows.len().max(1) as f64).max(1.0);
        self.snapshot = hash.finalize().to_hex().to_string();
        self.files = files;
        self.windows = windows;
        self.postings = postings;
        Ok(stats)
    }
    fn lexical_scores(&self, question: &str) -> BTreeMap<usize, f64> {
        let mut scores: BTreeMap<usize, f64> = BTreeMap::new();
        for term in terms(question).keys().take(64) {
            if let Some(postings) = self.postings.get(term) {
                let idf = (1.0
                    + (self.windows.len() as f64 - postings.len() as f64 + 0.5)
                        / (postings.len() as f64 + 0.5))
                    .ln();
                for &(index, count) in postings {
                    let length = self.windows[index].length as f64 / self.average;
                    let tf = count as f64;
                    *scores.entry(index).or_default() +=
                        idf * tf * 2.2 / (tf + 1.2 * (0.25 + 0.75 * length));
                }
            }
        }
        scores
    }
    /// Ranks lexical matches under an explicit excerpt and byte budget; no match is an empty context.
    pub fn retrieve(&self, question: &str, limit: usize, budget: usize) -> Result<Context, Error> {
        require(
            !question.trim().is_empty() && question.len() <= 4096,
            "Question must contain 1–4,096 UTF-8 bytes",
        )?;
        require(
            (1..=8).contains(&limit) && (1..=16000).contains(&budget),
            "Invalid QA context allowance",
        )?;
        let scores = self.lexical_scores(question);
        let mut ranked: Vec<_> = scores.into_iter().collect();
        ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0)));
        let mut selected: Vec<usize> = vec![];
        let mut used = 0;
        for (index, _) in ranked {
            let window = &self.windows[index];
            let length = window.range.len();
            if used + length > budget {
                continue;
            }
            if selected.iter().any(|&old| {
                let other = &self.windows[old];
                other.file.path == window.file.path
                    && other
                        .range
                        .end
                        .min(window.range.end)
                        .saturating_sub(other.range.start.max(window.range.start))
                        * 2
                        > other.range.len().min(length)
            }) {
                continue;
            }
            selected.push(index);
            used += length;
            if selected.len() == limit {
                break;
            }
        }
        Ok(Context {
            snapshot: self.snapshot.clone(),
            evidence: selected
                .iter()
                .enumerate()
                .map(|(i, &n)| {
                    let w = &self.windows[n];
                    excerpt(&w.file, &w.range, i + 1)
                })
                .collect(),
        })
    }
    /// Selects literal gold-anchored source for a labelled diagnostic, never a normal retrieval query.
    pub fn oracle(&self, anchors: &[(String, String)], budget: usize) -> Result<Context, Error> {
        require(
            anchors.len() <= 8 && (1..=16000).contains(&budget),
            "Invalid oracle allowance",
        )?;
        let mut evidence: Vec<Evidence> = vec![];
        let mut used = 0;
        for (path, needle) in anchors {
            require(
                !needle.is_empty() && needle.len() <= CHUNK,
                "Invalid evidence needle",
            )?;
            let file = self
                .files
                .get(path)
                .ok_or_else(|| Error(format!("Oracle source absent: {path}")))?;
            let matches: Vec<_> = file.text.match_indices(needle).collect();
            require(
                matches.len() == 1,
                "Oracle needle must have exactly one match",
            )?;
            let position = matches[0].0;
            let start = floor(&file.text, position.saturating_sub(400));
            let start = if position + needle.len() > start + CHUNK {
                position
            } else {
                start
            };
            let end = floor(
                &file.text,
                (position + needle.len() + 1000).min(start + CHUNK),
            );
            let item = excerpt(file, &(start..end), evidence.len() + 1);
            if evidence.iter().any(|e| e.id == item.id) {
                continue;
            }
            used += item.text.len();
            require(used <= budget, "Oracle context exceeds allowance")?;
            evidence.push(item);
        }
        Ok(Context {
            snapshot: self.snapshot.clone(),
            evidence,
        })
    }
    /// Checks current snapshot, status and exact quotes; semantic correctness needs a separate rubric.
    pub fn validate(&self, context: &Context, answer: &Answer) -> Result<(), Error> {
        require(
            context.snapshot == self.snapshot,
            "Context belongs to a different source snapshot",
        )?;
        require(
            !answer.answer.trim().is_empty()
                && answer.answer.len() <= 4096
                && answer.citations.len() <= 8,
            "Invalid answer bounds",
        )?;
        if matches!(answer.status, Status::Answered) {
            require(
                !answer.citations.is_empty(),
                "An answer needs cited evidence",
            )?;
        }
        for citation in &answer.citations {
            let source = citation
                .label
                .checked_sub(1)
                .and_then(|n| context.evidence.get(n))
                .ok_or_else(|| Error("Citation label is unavailable".into()))?;
            require(
                !citation.quote.trim().is_empty()
                    && citation.quote.len() <= 512
                    && source.text.contains(&citation.quote),
                "Citation quote does not match supplied source",
            )?;
        }
        Ok(())
    }
    /// Resolves current, available evidence IDs; this does not check semantic support.
    pub fn validate_ids<'a>(
        &self,
        context: &'a Context,
        answer: &IdAnswer,
    ) -> Result<Vec<SourceReference<'a>>, Error> {
        require(
            context.snapshot == self.snapshot,
            "Context belongs to a different source snapshot",
        )?;
        require(
            !answer.answer.trim().is_empty()
                && answer.answer.len() <= 4096
                && answer.citations.len() <= 8,
            "Invalid answer bounds",
        )?;
        if matches!(answer.status, Status::Answered) {
            require(
                !answer.citations.is_empty(),
                "An answer needs cited evidence",
            )?;
        }
        let mut seen = BTreeSet::new();
        let mut references = Vec::new();
        for &label in &answer.citations {
            require(seen.insert(label), "Duplicate citation label")?;
            let source = label
                .checked_sub(1)
                .and_then(|n| context.evidence.get(n))
                .ok_or_else(|| Error("Citation label is unavailable".into()))?;
            references.push(SourceReference {
                label,
                snapshot: &context.snapshot,
                id: &source.id,
                path: &source.path,
                content: &source.content,
                start_line: source.start_line,
                end_line: source.end_line,
            });
        }
        Ok(references)
    }
}
