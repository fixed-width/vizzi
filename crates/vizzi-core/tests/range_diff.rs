//! Declaration diffs retain original source coordinates and bounded failure semantics.
use vizzi_core::{
    model::{Document, WorkspaceId},
    policy::{Classification, Provenance, SourceOnlyReason},
    text_diff::{DiffError, DiffInput, DiffLimit, DiffLimits, compare_ranges},
};
fn document(text: &str) -> Document {
    Document::capture(
        WorkspaceId::derive(&[b"range-diff"]),
        "a.rs".into(),
        text.as_bytes(),
        vec![],
        Classification::SourceOnly {
            language: None,
            provenance: Provenance::ExplicitScope {
                scope: "a.rs".into(),
            },
            reason: SourceOnlyReason::AnalysisNotRequested,
        },
    )
    .unwrap()
}
#[test]
fn compares_declarations_beyond_loaded_windows_without_diffing_other_source() {
    let context = "// context\n".repeat(120);
    let before = format!("{context}fn target() {{\n old();\n}}\nfn unrelated() {{ a(); }}\n");
    let after = "// shifted\nfn target() {\n new();\n}\nfn unrelated() { b(); }\n";
    let a = document(&before);
    let b = document(after);
    let edits = compare_ranges(
        DiffInput {
            document: &a,
            span: context.len()..before.find("\nfn unrelated").unwrap(),
        },
        DiffInput {
            document: &b,
            span: after.find("fn target").unwrap()..after.find("\nfn unrelated").unwrap(),
        },
        DiffLimits::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].base.lines, 121..122);
    assert_eq!(edits[0].working.lines, 2..3);
    assert_eq!(&before[edits[0].base.bytes.clone()], " old();\n");
    assert_eq!(&after[edits[0].working.bytes.clone()], " new();\n");
}
#[test]
fn insertion_deletion_and_empty_ranges_preserve_absolute_anchors() {
    let a = document("head\nold\n");
    let b = document("header\nnew\nextra\n");
    let inserted = compare_ranges(
        DiffInput {
            document: &a,
            span: 5..5,
        },
        DiffInput {
            document: &b,
            span: 7..17,
        },
        DiffLimits::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(inserted[0].base.lines, 1..1);
    assert_eq!(inserted[0].base.bytes, 5..5);
    assert_eq!(inserted[0].working.lines, 1..3);
    let deleted = compare_ranges(
        DiffInput {
            document: &b,
            span: 7..17,
        },
        DiffInput {
            document: &a,
            span: 5..5,
        },
        DiffLimits::default(),
        &|| false,
    )
    .unwrap();
    assert_eq!(deleted[0].working.bytes, 5..5);
}
#[test]
fn rejects_invalid_ranges_limits_and_cancellation_without_partial_results() {
    let a = document("🙂\r\nold\r\n");
    let b = document("🙂\r\nnew\r\n");
    let run = |span, limits, cancel| {
        compare_ranges(
            DiffInput { document: &a, span },
            DiffInput {
                document: &b,
                span: 6..11,
            },
            limits,
            &move || cancel,
        )
    };
    assert!(matches!(
        run(1..5, DiffLimits::default(), false),
        Err(DiffError::InvalidRange(_))
    ));
    assert!(matches!(
        run(0..99, DiffLimits::default(), false),
        Err(DiffError::InvalidRange(_))
    ));
    assert!(matches!(
        run(
            std::ops::Range { start: 8, end: 7 },
            DiffLimits::default(),
            false
        ),
        Err(DiffError::InvalidRange(_))
    ));
    assert_eq!(
        run(
            6..11,
            DiffLimits {
                max_bytes: 9,
                ..DiffLimits::default()
            },
            false
        ),
        Err(DiffError::Limit(DiffLimit::Bytes))
    );
    assert_eq!(
        run(
            6..11,
            DiffLimits {
                max_lines: 1,
                ..DiffLimits::default()
            },
            false
        ),
        Err(DiffError::Limit(DiffLimit::Lines))
    );
    assert_eq!(
        run(
            6..11,
            DiffLimits {
                max_work: 0,
                ..DiffLimits::default()
            },
            false
        ),
        Err(DiffError::Limit(DiffLimit::Work))
    );
    assert_eq!(
        run(
            6..11,
            DiffLimits {
                max_hunks: 0,
                ..DiffLimits::default()
            },
            false
        ),
        Err(DiffError::Limit(DiffLimit::Hunks))
    );
    assert_eq!(
        run(6..11, DiffLimits::default(), true),
        Err(DiffError::Cancelled)
    );
    assert_eq!(
        run(
            6..11,
            DiffLimits {
                max_hunks: 1,
                ..DiffLimits::default()
            },
            false
        )
        .unwrap()
        .len(),
        1
    );
    let edits = run(6..11, DiffLimits::default(), false).unwrap();
    assert_eq!(
        &a.resident_source().unwrap()[edits[0].base.bytes.clone()],
        "old\r\n"
    );
}

#[test]
fn exact_selected_byte_allowance_ignores_unselected_context_and_preserves_empty_identity() {
    let a = document("large outside context\nold\r\n");
    let b = document("different context\nnew\r\n");
    let run = |left: &Document, span, right: &Document, other, limits| {
        compare_ranges(
            DiffInput {
                document: left,
                span,
            },
            DiffInput {
                document: right,
                span: other,
            },
            limits,
            &|| false,
        )
    };
    assert_eq!(
        run(
            &a,
            22..27,
            &b,
            18..23,
            DiffLimits {
                max_bytes: 10,
                ..DiffLimits::default()
            }
        )
        .unwrap()
        .len(),
        1
    );
    assert!(
        run(&a, 22..27, &a, 22..27, DiffLimits::default())
            .unwrap()
            .is_empty()
    );
    let emoji = document("🙂");
    assert!(matches!(
        run(&emoji, 1..1, &emoji, 0..0, DiffLimits::default()),
        Err(DiffError::InvalidRange(_))
    ));
    let nul = document("\0");
    assert!(matches!(
        run(&nul, 0..1, &emoji, 0..0, DiffLimits::default()),
        Err(DiffError::Binary(_))
    ));
    let unsupported = Document::capture(
        WorkspaceId::derive(&[b"range-diff"]),
        "a.rs".into(),
        b"\xff",
        vec![],
        emoji.classification().clone(),
    )
    .unwrap();
    assert!(matches!(
        run(&unsupported, 0..1, &emoji, 0..0, DiffLimits::default()),
        Err(DiffError::UnsupportedEncoding(_))
    ));
}
