//! Content fingerprints and deterministic changes for the indexing experiment.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Content identity and the number of bytes fingerprinted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fingerprint {
    /// BLAKE3 digest of the complete contents.
    pub hash: blake3::Hash,
    /// Content length in bytes.
    pub bytes: u64,
}

impl Fingerprint {
    /// Fingerprint an already available source buffer.
    pub fn of(contents: &[u8]) -> Self {
        Self {
            hash: blake3::hash(contents),
            bytes: contents.len() as u64,
        }
    }
}

/// The path and classification of a content change.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Change {
    /// Relative source path.
    pub path: PathBuf,
    /// How the path differs from the previous contents.
    pub kind: ChangeKind,
}

/// Content-level change classification; renames are not inferred.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    /// A previously absent path.
    Added,
    /// Different contents at an existing path.
    Modified,
    /// A previously present path is absent.
    Removed,
}

/// A deterministic manifest; it contains no language or source-control assumptions.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Manifest {
    files: BTreeMap<PathBuf, Fingerprint>,
}

impl Manifest {
    /// Construct a manifest from captured fingerprints.
    pub fn new(files: BTreeMap<PathBuf, Fingerprint>) -> Self {
        Self { files }
    }

    /// Read the captured files in path order.
    pub fn files(&self) -> &BTreeMap<PathBuf, Fingerprint> {
        &self.files
    }

    /// Compare this manifest to an earlier one.
    pub fn changes_from(&self, previous: &Self) -> Vec<Change> {
        let mut changes = Vec::new();
        for (path, fingerprint) in &self.files {
            if let Some(kind) = classify(previous.files.get(path), Some(fingerprint)) {
                changes.push(Change {
                    path: path.clone(),
                    kind,
                });
            }
        }
        for path in previous.files.keys() {
            if !self.files.contains_key(path) {
                changes.push(Change {
                    path: path.clone(),
                    kind: ChangeKind::Removed,
                });
            }
        }
        changes.sort_by(|a, b| a.path.cmp(&b.path));
        changes
    }

    /// Apply staged updates; `None` removes a path.
    pub fn apply(&mut self, updates: BTreeMap<PathBuf, Option<Fingerprint>>) -> Vec<Change> {
        let mut changes = Vec::new();
        for (path, next) in updates {
            let Some(kind) = classify(self.files.get(&path), next.as_ref()) else {
                continue;
            };
            match next {
                Some(fingerprint) => {
                    self.files.insert(path.clone(), fingerprint);
                }
                None => {
                    self.files.remove(&path);
                }
            }
            changes.push(Change { path, kind });
        }
        changes
    }
}

fn classify(previous: Option<&Fingerprint>, next: Option<&Fingerprint>) -> Option<ChangeKind> {
    match (previous, next) {
        (None, Some(_)) => Some(ChangeKind::Added),
        (Some(_), None) => Some(ChangeKind::Removed),
        (Some(before), Some(after)) if before != after => Some(ChangeKind::Modified),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(files: &[(&str, &[u8])]) -> Manifest {
        Manifest::new(
            files
                .iter()
                .map(|(p, contents)| (PathBuf::from(p), Fingerprint::of(contents)))
                .collect(),
        )
    }

    #[test]
    fn comparison_classifies_changes_in_path_order() {
        let before = manifest(&[("removed", b"old"), ("modified", b"old"), ("same", b"same")]);
        let after = manifest(&[("added", b"new"), ("modified", b"new"), ("same", b"same")]);
        assert_eq!(
            after.changes_from(&before),
            vec![
                Change {
                    path: "added".into(),
                    kind: ChangeKind::Added
                },
                Change {
                    path: "modified".into(),
                    kind: ChangeKind::Modified
                },
                Change {
                    path: "removed".into(),
                    kind: ChangeKind::Removed
                },
            ]
        );
    }

    #[test]
    fn supplied_updates_match_an_independently_constructed_manifest() {
        let mut before = manifest(&[("gone", b"old"), ("same", b"same"), ("edited", b"old")]);
        let updates = BTreeMap::from([
            ("gone".into(), None),
            ("edited".into(), Some(Fingerprint::of(b"new"))),
            ("added".into(), Some(Fingerprint::of(b"new"))),
            ("same".into(), Some(Fingerprint::of(b"same"))),
            ("never-present".into(), None),
        ]);
        let changes = before.apply(updates);
        let expected = manifest(&[("same", b"same"), ("edited", b"new"), ("added", b"new")]);
        assert_eq!(
            (before, changes),
            (
                expected,
                vec![
                    Change {
                        path: "added".into(),
                        kind: ChangeKind::Added
                    },
                    Change {
                        path: "edited".into(),
                        kind: ChangeKind::Modified
                    },
                    Change {
                        path: "gone".into(),
                        kind: ChangeKind::Removed
                    },
                ]
            )
        );
    }

    #[test]
    fn fingerprint_counts_bytes_including_non_utf8() {
        assert_eq!(Fingerprint::of(&[0, 255, 3]).bytes, 3);
    }

    #[test]
    fn identical_manifests_have_no_changes() {
        let before = manifest(&[("one", b"same")]);
        assert!(before.changes_from(&before).is_empty());
    }

    #[test]
    fn serialized_manifest_preserves_fingerprints() {
        let before = manifest(&[("one", b"same"), ("two", b"different")]);
        let encoded = serde_json::to_vec(&before).unwrap();
        assert_eq!(
            serde_json::from_slice::<Manifest>(&encoded).unwrap(),
            before
        );
    }

    #[test]
    fn accessor_exposes_captured_paths_and_content() {
        let captured = manifest(&[("one", b"same")]);
        assert_eq!(
            captured.files().get(&PathBuf::from("one")),
            Some(&Fingerprint::of(b"same"))
        );
    }

    #[test]
    fn removed_paths_are_sorted_with_added_paths() {
        let before = manifest(&[("a", b"old")]);
        let after = manifest(&[("z", b"new")]);
        assert_eq!(
            after.changes_from(&before),
            vec![
                Change {
                    path: "a".into(),
                    kind: ChangeKind::Removed
                },
                Change {
                    path: "z".into(),
                    kind: ChangeKind::Added
                },
            ]
        );
    }
}
