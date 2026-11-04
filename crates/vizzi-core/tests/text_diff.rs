//! Exact byte/line edits, complete contextual hunks and explicit bounded failures.

use std::{
    ops::Range,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};
use vizzi_core::{
    comparison::{
        Comparison, EntryContent, EntryKind, EntryState, InventoryEntry, Observation, ReviewPath,
        Side, Unavailability,
    },
    model::{
        CapturePolicyId, CapturedSource, Document, Snapshot, SourceReadError, SourceReader,
        WorkspaceId,
    },
    policy::{Classification, Provenance, SourceOnlyReason},
    text_diff::{DiffError, DiffLimit, DiffLimits, TextDiff},
};

fn path() -> ReviewPath {
    ReviewPath::parse(b"doc.md".to_vec()).unwrap()
}

fn observation(source: Option<Arc<CapturedSource>>) -> Arc<Observation> {
    let workspace = WorkspaceId::derive(&[b"diff-fixture"]);
    let mut documents = vec![];
    let mut entries = vec![];
    if let Some(source) = source {
        entries.push(InventoryEntry {
            path: path(),
            state: EntryState::Present {
                kind: EntryKind::Regular { executable: false },
                content: EntryContent::Verified {
                    id: source.content().clone(),
                    bytes: source.len(),
                },
            },
        });
        documents.push(Arc::new(
            Document::capture_source(
                workspace.clone(),
                "doc.md".into(),
                source,
                vec![],
                Classification::SourceOnly {
                    language: None,
                    provenance: Provenance::Extension {
                        extension: "md".into(),
                    },
                    reason: SourceOnlyReason::LanguageUnassigned,
                },
            )
            .unwrap(),
        ));
    }
    Arc::new(
        Observation::from_inventory(
            Arc::new(
                Snapshot::new(
                    workspace,
                    documents,
                    CapturePolicyId::derive(&[b"analysis"]),
                )
                .unwrap(),
            ),
            CapturePolicyId::derive(&[b"review"]),
            entries,
        )
        .unwrap(),
    )
}

fn pair(base: Option<&[u8]>, working: Option<&[u8]>) -> Comparison {
    let capture =
        |bytes: Option<&[u8]>| bytes.map(|bytes| CapturedSource::capture(bytes.to_vec(), None));
    Comparison::between(observation(capture(base)), observation(capture(working))).unwrap()
}

fn diff(base: &str, working: &str) -> TextDiff {
    pair(Some(base.as_bytes()), Some(working.as_bytes()))
        .text_diff(&path(), DiffLimits::default(), &|| false)
        .unwrap()
}

fn reconstruct(base: &[u8], working: &[u8], result: &TextDiff) {
    let mut output = vec![];
    let mut at = 0;
    let mut working_at = 0;
    for edit in &result.edits {
        assert!(edit.base.bytes.start >= at);
        assert!(edit.working.bytes.start >= working_at);
        assert_eq!(
            &base[at..edit.base.bytes.start],
            &working[working_at..edit.working.bytes.start]
        );
        output.extend_from_slice(&base[at..edit.base.bytes.start]);
        output.extend_from_slice(&working[edit.working.bytes.clone()]);
        at = edit.base.bytes.end;
        working_at = edit.working.bytes.end;
    }
    assert_eq!(&base[at..], &working[working_at..]);
    output.extend_from_slice(&base[at..]);
    assert_eq!(output, working);
}

fn ranges(result: &TextDiff) -> Vec<(Range<usize>, Range<usize>)> {
    result
        .edits
        .iter()
        .map(|edit| (edit.base.lines.clone(), edit.working.lines.clone()))
        .collect()
}

#[test]
fn replacement_insertion_deletion_and_empty_files_have_exact_expected_ranges() {
    let cases = [
        ("", "", vec![]),
        ("", "new\n", vec![(0..0, 0..1)]),
        ("old\n", "", vec![(0..1, 0..0)]),
        ("a\nb\nc\n", "a\nnew\nc\n", vec![(1..2, 1..2)]),
        ("a\nc\n", "a\nb\nc\n", vec![(1..1, 1..2)]),
        ("a\nb\nc\n", "a\nc\n", vec![(1..2, 1..1)]),
        ("a\n", "a\nlast", vec![(1..1, 1..2)]),
    ];
    for (base, working, expected) in cases {
        let result = diff(base, working);
        assert_eq!(ranges(&result), expected, "{base:?} -> {working:?}");
        reconstruct(base.as_bytes(), working.as_bytes(), &result);
        assert_eq!(result.base.bytes, base.len());
        assert_eq!(result.working.bytes, working.len());
    }
}

#[test]
fn line_terminators_unicode_and_final_newline_are_not_normalised() {
    for (base, working, expected) in [
        ("α\r\n🙂\n", "α\n🙂\n", vec![(0..1, 0..1)]),
        ("x", "x\n", vec![(0..1, 0..1)]),
        ("x\n", "x", vec![(0..1, 0..1)]),
        ("\n", "", vec![(0..1, 0..0)]),
        ("a\rb", "a\rc", vec![(0..1, 0..1)]),
        ("🙂\n", "🙂\nα", vec![(1..1, 1..2)]),
    ] {
        let result = diff(base, working);
        assert_eq!(ranges(&result), expected);
        assert_eq!(result.base.final_newline, base.ends_with('\n'));
        assert_eq!(result.working.final_newline, working.ends_with('\n'));
        assert_eq!(result.base.lines, base.split_inclusive('\n').count());
        assert_eq!(result.working.lines, working.split_inclusive('\n').count());
        reconstruct(base.as_bytes(), working.as_bytes(), &result);
    }
}

#[test]
fn presence_is_distinct_from_empty_text_and_missing_source_is_not_equality() {
    for (base, working, base_present) in [
        (None, Some(b"".as_slice()), false),
        (Some(b"".as_slice()), None, true),
    ] {
        let comparison = pair(base, working);
        let result = comparison
            .text_diff(&path(), DiffLimits::default(), &|| false)
            .unwrap();
        assert_eq!(result.base.version.is_some(), base_present);
        assert_eq!(result.working.version.is_some(), !base_present);
        assert!(result.edits.is_empty());
        assert!(result.hunks.is_empty());
    }
    assert_eq!(
        pair(None, None).text_diff(&path(), DiffLimits::default(), &|| false),
        Err(DiffError::UnknownPath)
    );
    for (base, working, expected) in [
        (
            b"\xff".as_slice(),
            b"ok".as_slice(),
            DiffError::UnsupportedEncoding(Side::Base),
        ),
        (
            b"ok".as_slice(),
            b"\xff".as_slice(),
            DiffError::UnsupportedEncoding(Side::Working),
        ),
        (
            b"\0".as_slice(),
            b"ok".as_slice(),
            DiffError::Binary(Side::Base),
        ),
        (
            b"ok".as_slice(),
            b"\0".as_slice(),
            DiffError::Binary(Side::Working),
        ),
    ] {
        assert_eq!(
            pair(Some(base), Some(working)).text_diff(&path(), DiffLimits::default(), &|| false),
            Err(expected)
        );
    }
}

#[test]
fn contextual_hunks_merge_at_six_unchanged_lines_and_split_at_seven() {
    let base = (0..20).map(|i| format!("line {i}\n")).collect::<String>();
    for (second, count) in [(6, 1), (9, 1), (10, 2)] {
        let working = base
            .replace("line 2\n", "first edit\n")
            .replace(&format!("line {second}\n"), "second edit\n");
        let result = diff(&base, &working);
        assert_eq!(
            ranges(&result),
            [(2..3, 2..3), (second..second + 1, second..second + 1)]
        );
        assert_eq!(result.hunks.len(), count);
        assert_eq!(result.hunks[0].base.lines.start, 0);
        assert_eq!(result.hunks[0].working.lines.start, 0);
        assert_eq!(result.hunks[0].edits.start, 0);
        assert_eq!(result.hunks.last().unwrap().edits.end, 2);
        assert_eq!(result.hunks.last().unwrap().base.lines.end, second + 4);
        assert_eq!(result.hunks.last().unwrap().working.lines.end, second + 4);
        reconstruct(base.as_bytes(), working.as_bytes(), &result);
    }
}

#[test]
fn defaults_and_inclusive_resource_limits_are_explicit() {
    let defaults = DiffLimits::default();
    assert_eq!(
        (
            defaults.max_bytes,
            defaults.max_lines,
            defaults.max_hunks,
            defaults.max_work
        ),
        (8 * 1024 * 1024, 200_000, 10_000, 10_000_000)
    );
    let comparison = pair(Some(b"a\n"), Some(b"b\n"));
    let success = comparison.text_diff(&path(), defaults, &|| false).unwrap();
    for (limits, expected) in [
        (
            DiffLimits {
                max_bytes: 3,
                ..defaults
            },
            DiffLimit::Bytes,
        ),
        (
            DiffLimits {
                max_lines: 1,
                ..defaults
            },
            DiffLimit::Lines,
        ),
        (
            DiffLimits {
                max_hunks: 0,
                ..defaults
            },
            DiffLimit::Hunks,
        ),
        (
            DiffLimits {
                max_work: success.work - 1,
                ..defaults
            },
            DiffLimit::Work,
        ),
    ] {
        assert_eq!(
            comparison.text_diff(&path(), limits, &|| false),
            Err(DiffError::Limit(expected))
        );
    }
    let exact = DiffLimits {
        max_bytes: 4,
        max_lines: 2,
        max_hunks: 1,
        max_work: success.work,
    };
    let result = comparison.text_diff(&path(), exact, &|| false).unwrap();
    assert_eq!(result.edits, success.edits);
    assert_eq!(result.work, success.work);
    assert_eq!(diff("same\n", "same\n").hunks.len(), 0);
}

#[test]
fn hunk_identity_binds_order_path_pair_and_diff_policy() {
    let comparison = pair(Some(b"before\n"), Some(b"after\n"));
    let result = comparison
        .text_diff(&path(), DiffLimits::default(), &|| false)
        .unwrap();
    let repeated = comparison
        .text_diff(&path(), DiffLimits::default(), &|| false)
        .unwrap();
    assert_eq!(result, repeated);
    assert_eq!(&result.comparison, comparison.id());
    assert_eq!(result.path, path());
    let reverse = pair(Some(b"after\n"), Some(b"before\n"))
        .text_diff(&path(), DiffLimits::default(), &|| false)
        .unwrap();
    let changed = diff("before\n", "different\n");
    let policy = comparison
        .text_diff(
            &path(),
            DiffLimits {
                max_hunks: 2,
                ..DiffLimits::default()
            },
            &|| false,
        )
        .unwrap();
    assert_ne!(result.hunks[0].id, reverse.hunks[0].id);
    assert_ne!(result.hunks[0].id, changed.hunks[0].id);
    assert_ne!(result.hunks[0].id, policy.hunks[0].id);
}

#[test]
fn cancellation_never_returns_partial_hunks() {
    let comparison = pair(Some(b"a\nb\nc\nd\n"), Some(b"w\nx\ny\nz\n"));
    let calls = std::cell::Cell::new(0);
    comparison
        .text_diff(&path(), DiffLimits::default(), &|| {
            calls.set(calls.get() + 1);
            false
        })
        .unwrap();
    let full = calls.get();
    for stop in [0, full / 2, full - 1] {
        let count = std::cell::Cell::new(0);
        assert_eq!(
            comparison.text_diff(&path(), DiffLimits::default(), &|| {
                let at = count.get();
                count.set(at + 1);
                at >= stop
            }),
            Err(DiffError::Cancelled)
        );
    }
}

fn edit_distance(base: &str, working: &str) -> usize {
    let a: Vec<_> = base.split_inclusive('\n').collect();
    let b: Vec<_> = working.split_inclusive('\n').collect();
    let mut distance = vec![vec![0; b.len() + 1]; a.len() + 1];
    for (i, row) in distance.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, value) in distance[0].iter_mut().enumerate() {
        *value = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            distance[i][j] = if a[i - 1] == b[j - 1] {
                distance[i - 1][j - 1]
            } else {
                1 + distance[i - 1][j].min(distance[i][j - 1])
            };
        }
    }
    distance[a.len()][b.len()]
}

#[test]
fn exhaustive_small_inputs_reconstruct_and_match_an_independent_minimum_distance() {
    let mut words = vec![String::new()];
    for length in 1..=4 {
        for mask in 0..1 << length {
            words.push(
                (0..length)
                    .map(|bit| if mask & (1 << bit) == 0 { "a\n" } else { "b\n" })
                    .collect(),
            );
        }
    }
    for base in &words {
        for working in &words {
            let result = diff(base, working);
            reconstruct(base.as_bytes(), working.as_bytes(), &result);
            let cost: usize = result
                .edits
                .iter()
                .map(|edit| edit.base.lines.len() + edit.working.lines.len())
                .sum();
            assert_eq!(
                cost,
                edit_distance(base, working),
                "{base:?} -> {working:?}"
            );
            assert_eq!(
                result
                    .hunks
                    .iter()
                    .flat_map(|hunk| hunk.edits.clone())
                    .collect::<Vec<_>>(),
                (0..result.edits.len()).collect::<Vec<_>>()
            );
            for edit in &result.edits {
                let offsets = |text: &str| {
                    std::iter::once(0)
                        .chain(text.split_inclusive('\n').scan(0, |at, line| {
                            *at += line.len();
                            Some(*at)
                        }))
                        .collect::<Vec<_>>()
                };
                let before = offsets(base);
                let after = offsets(working);
                assert_eq!(
                    edit.base.bytes,
                    before[edit.base.lines.start]..before[edit.base.lines.end]
                );
                assert_eq!(
                    edit.working.bytes,
                    after[edit.working.lines.start]..after[edit.working.lines.end]
                );
            }
        }
    }
    for (base, working) in [
        ("ABCABBA", "CBABAC"),
        ("ABC", "AXBCY"),
        ("BAAAAAB", "AAAAA"),
        ("BABABA", "ABABAB"),
    ] {
        let lines = |text: &str| text.chars().map(|ch| format!("{ch}\n")).collect::<String>();
        let (base, working) = (lines(base), lines(working));
        let result = diff(&base, &working);
        reconstruct(base.as_bytes(), working.as_bytes(), &result);
        assert_eq!(
            result
                .edits
                .iter()
                .map(|edit| edit.base.lines.len() + edit.working.lines.len())
                .sum::<usize>(),
            edit_distance(&base, &working)
        );
    }
}

fn metadata_observation(state: EntryState) -> Arc<Observation> {
    let empty = observation(None);
    Arc::new(
        Observation::from_inventory(
            empty.snapshot().clone(),
            empty.policy().clone(),
            vec![InventoryEntry {
                path: path(),
                state,
            }],
        )
        .unwrap(),
    )
}

#[test]
fn unavailable_nonregular_and_unretained_entries_have_distinct_side_specific_errors() {
    let text = observation(Some(CapturedSource::capture(b"text\n".to_vec(), None)));
    for side in [Side::Base, Side::Working] {
        let cases = [
            (
                EntryState::Unobserved(Unavailability::Sparse),
                DiffError::Unavailable {
                    side,
                    reason: Unavailability::Sparse,
                },
            ),
            (
                EntryState::Present {
                    kind: EntryKind::Regular { executable: false },
                    content: EntryContent::Unavailable(Unavailability::Excluded),
                },
                DiffError::Unavailable {
                    side,
                    reason: Unavailability::Excluded,
                },
            ),
            (
                EntryState::Present {
                    kind: EntryKind::Regular { executable: false },
                    content: EntryContent::verified(b"text\n"),
                },
                DiffError::SourceNotRetained(side),
            ),
            (
                EntryState::Present {
                    kind: EntryKind::Symlink,
                    content: EntryContent::verified(b"target"),
                },
                DiffError::UnsupportedKind(side),
            ),
            (
                EntryState::Present {
                    kind: EntryKind::Reference,
                    content: EntryContent::verified(b"pointer"),
                },
                DiffError::UnsupportedKind(side),
            ),
            (
                EntryState::Present {
                    kind: EntryKind::Special,
                    content: EntryContent::Unavailable(Unavailability::UnsupportedKind),
                },
                DiffError::UnsupportedKind(side),
            ),
        ];
        for (state, error) in cases {
            let metadata = metadata_observation(state);
            let comparison = match side {
                Side::Base => Comparison::between(metadata, text.clone()),
                Side::Working => Comparison::between(text.clone(), metadata),
            }
            .unwrap();
            assert_eq!(
                comparison.text_diff(&path(), DiffLimits::default(), &|| false),
                Err(error)
            );
        }
    }
}

#[derive(Debug)]
struct Backing {
    bytes: Vec<u8>,
    fail: AtomicBool,
    reads: AtomicUsize,
    cancel_on_read: Option<Arc<AtomicBool>>,
}

impl SourceReader for Backing {
    fn read_at(&self, offset: usize, target: &mut [u8]) -> Result<(), SourceReadError> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        if self.fail.load(Ordering::SeqCst) {
            return Err(SourceReadError::Io("fixture unavailable".into()));
        }
        if let Some(cancelled) = &self.cancel_on_read {
            cancelled.store(true, Ordering::SeqCst);
        }
        let bytes = self
            .bytes
            .get(offset..offset + target.len())
            .ok_or(SourceReadError::ShortRead)?;
        target.copy_from_slice(bytes);
        Ok(())
    }
}

fn backing(bytes: &[u8]) -> Arc<Backing> {
    Arc::new(Backing {
        bytes: bytes.to_vec(),
        fail: AtomicBool::new(false),
        reads: AtomicUsize::new(0),
        cancel_on_read: None,
    })
}

#[test]
fn backed_sources_verify_reads_and_byte_admission_precedes_io() {
    let bytes = b"original\r\n";
    let source = CapturedSource::capture(bytes.to_vec(), None);
    let reader = backing(bytes);
    let comparison = Comparison::between(
        observation(Some(source.clone().with_backing(reader.clone()))),
        observation(Some(CapturedSource::capture(b"changed\n".to_vec(), None))),
    )
    .unwrap();
    assert_eq!(
        comparison.text_diff(
            &path(),
            DiffLimits {
                max_bytes: 1,
                ..DiffLimits::default()
            },
            &|| false
        ),
        Err(DiffError::Limit(DiffLimit::Bytes))
    );
    assert_eq!(reader.reads.load(Ordering::SeqCst), 0);
    let result = comparison
        .text_diff(&path(), DiffLimits::default(), &|| false)
        .unwrap();
    reconstruct(bytes, b"changed\n", &result);
    reader.fail.store(true, Ordering::SeqCst);
    assert_eq!(
        comparison.text_diff(&path(), DiffLimits::default(), &|| false),
        Err(DiffError::Read {
            side: Side::Base,
            source: SourceReadError::Io("fixture unavailable".into())
        })
    );
    for (corrupt, expected) in [
        (b"incorrect!".as_slice(), SourceReadError::Integrity),
        (b"short".as_slice(), SourceReadError::ShortRead),
    ] {
        let comparison = Comparison::between(
            observation(Some(source.clone().with_backing(backing(corrupt)))),
            observation(None),
        )
        .unwrap();
        assert_eq!(
            comparison.text_diff(&path(), DiffLimits::default(), &|| false),
            Err(DiffError::Read {
                side: Side::Base,
                source: expected
            })
        );
    }
}

#[test]
fn cancellation_during_a_backing_load_does_not_start_the_next_load() {
    let cancelled = Arc::new(AtomicBool::new(false));
    let bytes = b"a\n";
    let base = Arc::new(Backing {
        bytes: bytes.to_vec(),
        fail: AtomicBool::new(false),
        reads: AtomicUsize::new(0),
        cancel_on_read: Some(cancelled.clone()),
    });
    let working = backing(bytes);
    let source = CapturedSource::capture(bytes.to_vec(), None);
    let comparison = Comparison::between(
        observation(Some(source.clone().with_backing(base))),
        observation(Some(source.with_backing(working.clone()))),
    )
    .unwrap();
    assert_eq!(
        comparison.text_diff(&path(), DiffLimits::default(), &|| cancelled
            .load(Ordering::SeqCst)),
        Err(DiffError::Cancelled)
    );
    assert_eq!(working.reads.load(Ordering::SeqCst), 0);
}

#[test]
fn later_hunk_and_search_exhaustion_cannot_publish_a_partial_result() {
    let base = (0..30).map(|i| format!("line {i}\n")).collect::<String>();
    let working = base
        .replace("line 3\n", "first edit\n")
        .replace("line 25\n", "second edit\n");
    let comparison = pair(Some(base.as_bytes()), Some(working.as_bytes()));
    assert_eq!(
        comparison.text_diff(
            &path(),
            DiffLimits {
                max_hunks: 1,
                ..DiffLimits::default()
            },
            &|| false
        ),
        Err(DiffError::Limit(DiffLimit::Hunks))
    );
    assert_eq!(
        comparison
            .text_diff(
                &path(),
                DiffLimits {
                    max_hunks: 2,
                    ..DiffLimits::default()
                },
                &|| false
            )
            .unwrap()
            .hunks
            .len(),
        2
    );
    let base = "a\n".repeat(100);
    let working = "b\n".repeat(100);
    assert_eq!(
        pair(Some(base.as_bytes()), Some(working.as_bytes())).text_diff(
            &path(),
            DiffLimits {
                max_work: 100,
                ..DiffLimits::default()
            },
            &|| false
        ),
        Err(DiffError::Limit(DiffLimit::Work))
    );
    let long = "x".repeat(1_000);
    assert_eq!(
        pair(Some(long.as_bytes()), Some(long.as_bytes())).text_diff(
            &path(),
            DiffLimits {
                max_work: 100,
                ..DiffLimits::default()
            },
            &|| false
        ),
        Err(DiffError::Limit(DiffLimit::Work))
    );
    assert!(
        pair(Some(b"same\n"), Some(b"same\n"))
            .text_diff(
                &path(),
                DiffLimits {
                    max_hunks: 0,
                    ..DiffLimits::default()
                },
                &|| false
            )
            .unwrap()
            .hunks
            .is_empty()
    );
}

#[test]
fn one_sided_large_additions_and_deletions_avoid_a_quadratic_search() {
    let text = "line\n".repeat(1_000);
    for comparison in [
        pair(None, Some(text.as_bytes())),
        pair(Some(text.as_bytes()), None),
    ] {
        let result = comparison
            .text_diff(
                &path(),
                DiffLimits {
                    max_work: 2,
                    ..DiffLimits::default()
                },
                &|| false,
            )
            .unwrap();
        assert_eq!(result.edits.len(), 1);
        assert_eq!(result.hunks.len(), 1);
        assert_eq!(result.work, 2);
    }
}

#[test]
fn identical_hunks_in_different_files_of_one_pair_have_distinct_ids() {
    let files = |bytes: &[u8]| {
        let source = observation(Some(CapturedSource::capture(bytes.to_vec(), None)));
        let first = source.snapshot().documents()[0].clone();
        let second = Arc::new(
            Document::capture_source(
                source.snapshot().workspace().clone(),
                "other.md".into(),
                first.captured().clone(),
                vec![],
                first.classification().clone(),
            )
            .unwrap(),
        );
        let mut inventory = source.inventory().to_vec();
        inventory.push(InventoryEntry {
            path: ReviewPath::parse(b"other.md".to_vec()).unwrap(),
            state: inventory[0].state.clone(),
        });
        Arc::new(
            Observation::from_inventory(
                Arc::new(
                    Snapshot::new(
                        source.snapshot().workspace().clone(),
                        vec![first, second],
                        source.snapshot().policy().clone(),
                    )
                    .unwrap(),
                ),
                source.policy().clone(),
                inventory,
            )
            .unwrap(),
        )
    };
    let comparison = Comparison::between(files(b"before\n"), files(b"after\n")).unwrap();
    let first = comparison
        .text_diff(&path(), DiffLimits::default(), &|| false)
        .unwrap();
    let other_path = ReviewPath::parse(b"other.md".to_vec()).unwrap();
    let other = comparison
        .text_diff(&other_path, DiffLimits::default(), &|| false)
        .unwrap();
    assert_eq!(first.comparison, other.comparison);
    assert_eq!(first.edits, other.edits);
    assert_eq!(other.path, other_path);
    assert_ne!(first.hunks[0].id, other.hunks[0].id);
    for (side, metadata) in [(Side::Base, first.base), (Side::Working, first.working)] {
        assert_eq!(
            metadata.version.as_ref(),
            Some(comparison.document(side, &path()).unwrap().version())
        );
    }
}

#[test]
fn work_admission_charges_reserved_trace_storage_as_well_as_byte_comparison() {
    let comparison = pair(Some(b"x"), Some(b"x"));
    // One reserved frontier cell, one line comparison and one compared byte.
    assert_eq!(
        comparison
            .text_diff(
                &path(),
                DiffLimits {
                    max_work: 3,
                    ..DiffLimits::default()
                },
                &|| false
            )
            .unwrap()
            .work,
        3
    );
    assert_eq!(
        comparison.text_diff(
            &path(),
            DiffLimits {
                max_work: 2,
                ..DiffLimits::default()
            },
            &|| false
        ),
        Err(DiffError::Limit(DiffLimit::Work))
    );
}

#[test]
fn contextual_ranges_stay_aligned_after_an_edit_changes_line_counts() {
    let base = (0..30).map(|i| format!("line {i}\n")).collect::<String>();
    let inserted = (0..20)
        .map(|i| format!("inserted {i}\n"))
        .collect::<String>();
    for (second, expected) in [
        (8, vec![(0..12, 0..31)]),
        (20, vec![(0..6, 0..25), (17..24, 36..43)]),
    ] {
        let working = base
            .replace("line 2\n", &inserted)
            .replace(&format!("line {second}\n"), "second edit\n");
        let result = diff(&base, &working);
        assert_eq!(
            result
                .hunks
                .iter()
                .map(|hunk| (hunk.base.lines.clone(), hunk.working.lines.clone()))
                .collect::<Vec<_>>(),
            expected
        );
        let offset = |text: &str, line| {
            text.split_inclusive('\n')
                .take(line)
                .map(str::len)
                .sum::<usize>()
        };
        for hunk in &result.hunks {
            assert_eq!(
                hunk.base.bytes,
                offset(&base, hunk.base.lines.start)..offset(&base, hunk.base.lines.end)
            );
            assert_eq!(
                hunk.working.bytes,
                offset(&working, hunk.working.lines.start)
                    ..offset(&working, hunk.working.lines.end)
            );
        }
        reconstruct(base.as_bytes(), working.as_bytes(), &result);
    }
}
