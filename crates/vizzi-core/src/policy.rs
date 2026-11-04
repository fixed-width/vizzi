//! Portable source selection, independent of filesystems and analyzer factories.

use crate::model::{CapturePolicyId, ModelError};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Requested raw C-family language; it does not select a compiler configuration.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FamilyLanguage {
    /// C syntax.
    C,
    /// C++ syntax.
    Cpp,
}
impl FamilyLanguage {
    fn name(self) -> &'static str {
        match self {
            Self::C => "c",
            Self::Cpp => "cpp",
        }
    }
}
/// A language assignment for a relative path and its descendants.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Assignment {
    /// Relative scope; `.` denotes the workspace root.
    pub scope: String,
    /// Requested raw language for supported C-family candidates.
    pub language: FamilyLanguage,
}
/// Frozen source-selection configuration; Rust/Python are included by default.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PolicyConfig {
    /// Explicitly enabled C-family candidate subtrees.
    pub c_family_roots: Vec<String>,
    /// Excluded relative paths/subtrees, taking precedence over assignments.
    pub exclusions: Vec<String>,
    /// Explicit raw-language assignments within enabled family scopes.
    pub assignments: Vec<Assignment>,
}
/// Evidence used to select a language or retain a source-only candidate.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Provenance {
    /// A case-sensitive suffix hint.
    Extension {
        /// Exact suffix without its leading dot.
        extension: String,
    },
    /// An explicit path rule.
    ExplicitScope {
        /// Normalized scope; `.` denotes the root.
        scope: String,
    },
}
/// Why captured source has no assigned analysis.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceOnlyReason {
    /// Review source is retained outside the engine's selected analysis scope.
    OutsideAnalysisScope,
    /// This source capture deliberately runs no language analysis.
    AnalysisNotRequested,
    /// An ambiguous header has no language assignment.
    LanguageUnassigned,
    /// A known dialect, initially CUDA, is not supported.
    DialectUnsupported,
    /// A textual include fragment has no language assignment.
    FragmentUnassigned,
    /// The requested language has no registered analyzer.
    AnalyzerUnavailable,
}
/// Immutable source-selection metadata, separate from grammar recovery and encoding.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Classification {
    /// Raw syntax analysis is assigned to a registered language.
    Assigned {
        /// Analyzer language identifier.
        language: String,
        /// Assignment evidence.
        provenance: Provenance,
    },
    /// Source remains available without an invented analysis record.
    SourceOnly {
        /// Known/requested language, absent for ambiguous input.
        language: Option<String>,
        /// Classification evidence.
        provenance: Provenance,
        /// Reason analysis was not assigned.
        reason: SourceOnlyReason,
    },
}
impl Classification {
    /// Returns the selected analyzer language, excluding source-only requests.
    pub fn assigned_language(&self) -> Option<&str> {
        match self {
            Self::Assigned { language, .. } => Some(language),
            Self::SourceOnly { .. } => None,
        }
    }
}

/// A normalized, immutable source policy and its available analyzer languages.
pub struct SourcePolicy {
    id: CapturePolicyId,
    roots: BTreeSet<String>,
    exclusions: BTreeSet<String>,
    assignments: BTreeMap<String, FamilyLanguage>,
    available: BTreeSet<String>,
}
impl SourcePolicy {
    /// Validates scopes, resolves redundant rules, and fingerprints selection semantics.
    pub fn new(
        config: PolicyConfig,
        available: impl IntoIterator<Item = String>,
    ) -> Result<Self, ModelError> {
        let roots = scopes(config.c_family_roots)?;
        let exclusions = scopes(config.exclusions)?;
        let available: BTreeSet<_> = available.into_iter().collect();
        let mut requested = BTreeMap::new();
        for assignment in config.assignments {
            let scope = normalize_scope(&assignment.scope)?;
            if !roots
                .iter()
                .any(|root| within(&scope, root) || within(root, &scope))
            {
                return Err(ModelError(
                    "assignment is outside enabled C-family scopes".into(),
                ));
            }
            if requested
                .insert(scope, assignment.language)
                .is_some_and(|old| old != assignment.language)
            {
                return Err(ModelError(
                    "conflicting language assignments at the same scope".into(),
                ));
            }
        }
        let mut assignments: BTreeMap<String, FamilyLanguage> = BTreeMap::new();
        for (scope, language) in requested {
            if assignments
                .iter()
                .filter(|(parent, _)| within(&scope, parent))
                .max_by_key(|(parent, _)| parent.len())
                .is_none_or(|(_, parent)| *parent != language)
            {
                assignments.insert(scope, language);
            }
        }
        let encoded = serde_json::to_vec(&(&roots, &exclusions, &assignments, &available))
            .map_err(|e| ModelError(e.to_string()))?;
        let id =
            CapturePolicyId::derive(&[b"source-policy-v1", include_bytes!("policy.rs"), &encoded]);
        Ok(Self {
            id,
            roots,
            exclusions,
            assignments,
            available,
        })
    }
    /// Returns the frozen capture-policy identity, including analyzer availability.
    pub fn id(&self) -> &CapturePolicyId {
        &self.id
    }

    /// Tests exclusions for a normalized relative file or directory path.
    pub fn excludes(&self, path: &str) -> Result<bool, ModelError> {
        validate_path(path)?;
        Ok([".git", "target", "node_modules"]
            .iter()
            .any(|scope| within(path, scope))
            || self.exclusions.iter().any(|scope| within(path, scope)))
    }
    /// Returns capture metadata or exclusion from this scope; performs no I/O.
    pub fn classify(&self, path: &str) -> Result<Option<Classification>, ModelError> {
        if self.excludes(path)? {
            return Ok(None);
        }
        let filename = path.rsplit('/').next().unwrap_or(path);
        let Some((stem, extension)) = filename.rsplit_once('.') else {
            return Ok(None);
        };
        if stem.is_empty() {
            return Ok(None);
        }
        let provenance = Provenance::Extension {
            extension: extension.into(),
        };
        let language = match extension {
            "rs" => Some("rust"),
            "py" => Some("python"),
            _ => {
                if !self.roots.iter().any(|scope| within(path, scope)) {
                    return Ok(None);
                }
                if matches!(extension, "cu" | "cuh") {
                    return Ok(Some(Classification::SourceOnly {
                        language: Some("cuda".into()),
                        provenance,
                        reason: SourceOnlyReason::DialectUnsupported,
                    }));
                }
                let hint = match extension {
                    "c" => Some("c"),
                    "cc" | "cpp" | "cxx" | "C" | "c++" | "hh" | "hpp" | "hxx" | "h++" => {
                        Some("cpp")
                    }
                    "h" | "H" | "inc" | "inl" | "ipp" | "tpp" => None,
                    _ => return Ok(None),
                };
                if let Some((scope, language)) = self
                    .assignments
                    .iter()
                    .filter(|(scope, _)| within(path, scope))
                    .max_by_key(|(scope, _)| scope.len())
                {
                    return Ok(Some(self.assignment(
                        language.name(),
                        Provenance::ExplicitScope {
                            scope: if scope.is_empty() {
                                ".".into()
                            } else {
                                scope.clone()
                            },
                        },
                    )));
                }
                if hint.is_none() {
                    let reason = if matches!(extension, "h" | "H") {
                        SourceOnlyReason::LanguageUnassigned
                    } else {
                        SourceOnlyReason::FragmentUnassigned
                    };
                    return Ok(Some(Classification::SourceOnly {
                        language: None,
                        provenance,
                        reason,
                    }));
                }
                hint
            }
        };
        Ok(language.map(|language| self.assignment(language, provenance)))
    }
    fn assignment(&self, language: &str, provenance: Provenance) -> Classification {
        if self.available.contains(language) {
            Classification::Assigned {
                language: language.into(),
                provenance,
            }
        } else {
            Classification::SourceOnly {
                language: Some(language.into()),
                provenance,
                reason: SourceOnlyReason::AnalyzerUnavailable,
            }
        }
    }
}

fn within(path: &str, scope: &str) -> bool {
    scope.is_empty()
        || path == scope
        || path
            .strip_prefix(scope)
            .is_some_and(|tail| tail.starts_with('/'))
}
fn scopes(input: Vec<String>) -> Result<BTreeSet<String>, ModelError> {
    let normalized = input
        .iter()
        .map(|scope| normalize_scope(scope))
        .collect::<Result<BTreeSet<_>, _>>()?;
    let mut result = BTreeSet::<String>::new();
    for scope in normalized {
        if !result.iter().any(|parent| within(&scope, parent)) {
            result.insert(scope);
        }
    }
    Ok(result)
}
fn normalize_scope(scope: &str) -> Result<String, ModelError> {
    if scope.is_empty() || scope.starts_with('/') {
        return Err(ModelError("expected a relative source scope".into()));
    }
    let scope = scope
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect::<Vec<_>>()
        .join("/");
    if !scope.is_empty() {
        validate_path(&scope)?;
    }
    Ok(scope)
}
fn validate_path(path: &str) -> Result<(), ModelError> {
    if path.is_empty()
        || path.contains('\\')
        || path.as_bytes().get(1) == Some(&b':')
        || path.split('/').any(|part| matches!(part, "" | "." | ".."))
    {
        return Err(ModelError(
            "expected a normalized relative source path".into(),
        ));
    }
    Ok(())
}
