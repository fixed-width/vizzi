//! Behavioral checks for the experimental retrieval and citation boundary.
use vizzi_search::{Answer, Citation, Corpus, SourceFile, Status};
fn file(path: &str, text: &str) -> SourceFile {
    SourceFile {
        path: path.into(),
        text: text.into(),
    }
}
fn corpus() -> Corpus {
    let mut corpus = Corpus::new().unwrap();
    corpus
        .replace(vec![
            file(
                "src/read.rs",
                "pub fn readFile() { /* read bytes from disk */ }\n",
            ),
            file(
                "README.md",
                "Use the compiler to turn source into bytecode.\n",
            ),
        ])
        .unwrap();
    corpus
}
#[test]
fn natural_words_match_code_identifiers_and_results_have_exact_source_bindings() {
    let corpus = corpus();
    let context = corpus
        .retrieve("How does read file obtain bytes?", 6, 8000)
        .unwrap();
    assert!(!context.evidence().is_empty());
    let first = &context.evidence()[0];
    assert_eq!(first.path, "src/read.rs");
    assert_eq!(first.start_line, 1);
    assert!(first.text.contains("readFile"));
    assert_eq!(context.snapshot(), corpus.snapshot());
    assert_eq!(
        serde_json::to_string(&context).unwrap(),
        serde_json::to_string(
            &corpus
                .retrieve("How does read file obtain bytes?", 6, 8000)
                .unwrap()
        )
        .unwrap()
    );
}
#[test]
fn updates_reuse_unchanged_documents_remove_deleted_material_and_reject_held_context() {
    let mut corpus = corpus();
    let old = corpus.retrieve("readFile", 6, 8000).unwrap();
    let before = corpus.snapshot().to_owned();
    let stats = corpus
        .replace(vec![
            file(
                "README.md",
                "Use the compiler to turn source into bytecode.\n",
            ),
            file("src/new.rs", "pub fn loadBytes() {}\n"),
        ])
        .unwrap();
    assert_eq!(stats.reused, 1);
    assert_eq!(stats.changed, 1);
    assert_eq!(stats.removed, 1);
    assert_ne!(before, corpus.snapshot());
    assert!(
        corpus
            .retrieve("readFile", 6, 8000)
            .unwrap()
            .evidence()
            .is_empty()
    );
    assert_eq!(
        corpus.retrieve("loadBytes", 6, 8000).unwrap().evidence()[0].path,
        "src/new.rs"
    );
    let answer = Answer {
        status: Status::Answered,
        answer: "Reads bytes".into(),
        citations: vec![Citation {
            label: 1,
            quote: "read bytes from disk".into(),
        }],
    };
    assert!(corpus.validate(&old, &answer).is_err());
}
#[test]
fn rejected_replacements_preserve_a_complete_previous_index() {
    let mut corpus = corpus();
    let before = corpus.snapshot().to_owned();
    for files in [
        vec![file("../escape.rs", "x")],
        vec![file("a.rs", "a"), file("a.rs", "b")],
        vec![file("/absolute.rs", "x")],
    ] {
        assert!(corpus.replace(files).is_err());
        assert_eq!(corpus.snapshot(), before);
    }
}
#[test]
fn source_order_does_not_change_the_snapshot_and_empty_files_are_part_of_identity() {
    let mut corpus = corpus();
    let before = corpus.snapshot().to_owned();
    let stats = corpus
        .replace(vec![
            file(
                "README.md",
                "Use the compiler to turn source into bytecode.\n",
            ),
            file(
                "src/read.rs",
                "pub fn readFile() { /* read bytes from disk */ }\n",
            ),
        ])
        .unwrap();
    assert_eq!(stats.reused, 2);
    assert_eq!(stats.removed, 0);
    assert_eq!(before, corpus.snapshot());
    corpus.replace(vec![file("empty.rs", "")]).unwrap();
    let empty = corpus.snapshot().to_owned();
    corpus.replace(vec![]).unwrap();
    assert_ne!(empty, corpus.snapshot());
}

#[test]
fn retrieval_avoids_redundant_overlapping_function_windows() {
    let mut corpus = Corpus::new().unwrap();
    let text = format!(
        "fn target_one() {{}}\nfn target_two() {{}}\n{}fn target_three() {{}}\n",
        "// filler\n".repeat(300)
    );
    corpus.replace(vec![file("a.rs", &text)]).unwrap();
    let context = corpus.retrieve("target", 8, 16000).unwrap();
    assert_eq!(context.evidence().len(), 2);
    let ranges = context
        .evidence()
        .iter()
        .map(|e| {
            let start = text.find(&e.text).unwrap();
            start..start + e.text.len()
        })
        .collect::<Vec<_>>();
    let overlap = ranges[0]
        .end
        .min(ranges[1].end)
        .saturating_sub(ranges[0].start.max(ranges[1].start));
    assert!(overlap * 2 <= ranges[0].len().min(ranges[1].len()));
}
#[test]
fn context_and_utf8_windows_obey_budgets_without_inventing_evidence() {
    let mut corpus = Corpus::new().unwrap();
    corpus
        .replace(vec![file(
            "notes.md",
            &format!("needle {}", "界".repeat(6000)),
        )])
        .unwrap();
    let context = corpus.retrieve("needle", 2, 2400).unwrap();
    assert!(!context.evidence().is_empty());
    assert!(context.evidence().len() <= 2);
    assert!(context.evidence().iter().all(|e| e.text.len() <= 1800));
    assert!(
        context
            .evidence()
            .iter()
            .map(|e| e.text.len())
            .sum::<usize>()
            <= 2400
    );
    assert!(corpus.retrieve("needle", 0, 8000).is_err());
    assert!(corpus.retrieve("needle", 9, 8000).is_err());
    assert!(corpus.retrieve(&"x".repeat(4097), 2, 8000).is_err());
    assert!(corpus.retrieve("needle", 2, 0).is_err());
}

#[test]
fn empty_and_overlong_tokens_do_not_create_matches_and_frequency_affects_ranking() {
    let mut corpus = Corpus::new().unwrap();
    corpus
        .replace(vec![
            file("a.md", "needle filler filler filler filler"),
            file("z.md", "needle needle needle filler filler"),
            file(
                "tokens.md",
                &format!("{} {}", "x".repeat(128), "y".repeat(129)),
            ),
        ])
        .unwrap();
    assert_eq!(
        corpus.retrieve("needle", 2, 8000).unwrap().evidence()[0].path,
        "z.md"
    );
    assert!(
        corpus
            .retrieve("Where is absent_symbol?", 2, 8000)
            .unwrap()
            .evidence()
            .is_empty()
    );
    assert!(
        !corpus
            .retrieve(&"x".repeat(128), 2, 8000)
            .unwrap()
            .evidence()
            .is_empty()
    );
    assert!(
        corpus
            .retrieve(&"y".repeat(129), 2, 8000)
            .unwrap()
            .evidence()
            .is_empty()
    );
}
#[test]
fn private_and_dunder_identifiers_are_searchable_as_words_without_empty_token_matches() {
    let mut corpus = Corpus::new().unwrap();
    corpus
        .replace(vec![file(
            "a.py",
            "def __init__(self): pass\ndef _validate(self): pass\n",
        )])
        .unwrap();
    for word in ["init", "validate"] {
        assert!(
            !corpus
                .retrieve(word, 1, 8000)
                .unwrap()
                .evidence()
                .is_empty()
        );
    }
    assert!(
        corpus
            .retrieve("Where is __nonexistent__?", 1, 8000)
            .unwrap()
            .evidence()
            .is_empty()
    );
}

#[test]
fn an_over_total_source_budget_is_rejected_without_publishing_partial_replacement() {
    let mut corpus = corpus();
    let before = corpus.snapshot().to_owned();
    let files = (0..17)
        .map(|i| file(&format!("file{i}.txt"), &"x".repeat(2 * 1024 * 1024)))
        .collect();
    assert!(corpus.replace(files).is_err());
    assert_eq!(corpus.snapshot(), before);
}
#[test]
fn a_long_oracle_needle_is_fully_present_in_its_bounded_excerpt() {
    let mut corpus = Corpus::new().unwrap();
    let needle = "x".repeat(1700);
    corpus
        .replace(vec![file(
            "document.md",
            &format!("{}{}end", "p".repeat(500), needle),
        )])
        .unwrap();
    let context = corpus
        .oracle(&[("document.md".into(), needle.clone())], 8000)
        .unwrap();
    assert!(context.evidence()[0].text.contains(&needle));
    assert!(context.evidence()[0].text.len() <= 1800);
}
#[test]
fn oracle_keeps_distinct_sources_deduplicates_identical_anchors_and_rejects_existing_oversized_needles()
 {
    let mut corpus = corpus();
    let long = "z".repeat(1801);
    corpus
        .replace(vec![
            file("a.md", "alpha evidence"),
            file("b.md", "beta evidence"),
            file("long.md", &long),
        ])
        .unwrap();
    let anchors = vec![
        ("a.md".into(), "alpha evidence".into()),
        ("b.md".into(), "beta evidence".into()),
        ("a.md".into(), "alpha evidence".into()),
    ];
    let context = corpus.oracle(&anchors, 8000).unwrap();
    assert_eq!(context.evidence().len(), 2);
    assert_eq!(
        context
            .evidence()
            .iter()
            .map(|e| e.path.as_str())
            .collect::<Vec<_>>(),
        ["a.md", "b.md"]
    );
    assert_eq!(context.evidence()[1].label, 2);
    assert!(corpus.oracle(&anchors, 10).is_err());
    assert!(corpus.oracle(&[("long.md".into(), long)], 8000).is_err());
}
#[test]
fn citations_require_available_labels_and_literal_nonempty_quotes_but_do_not_grade_meaning() {
    let corpus = corpus();
    let context = corpus.retrieve("readFile", 6, 8000).unwrap();
    let mut answer = Answer {
        status: Status::Answered,
        answer: "An explanation".into(),
        citations: vec![Citation {
            label: 1,
            quote: "read bytes from disk".into(),
        }],
    };
    corpus.validate(&context, &answer).unwrap();
    for (label, quote) in [
        (0, "read bytes from disk"),
        (99, "read bytes from disk"),
        (1, "invented text"),
        (1, ""),
    ] {
        answer.citations = vec![Citation {
            label,
            quote: quote.into(),
        }];
        assert!(corpus.validate(&context, &answer).is_err());
    }
    answer.citations.clear();
    assert!(corpus.validate(&context, &answer).is_err());
    answer.status = Status::InsufficientEvidence;
    corpus.validate(&context, &answer).unwrap();
}
#[test]
fn oracle_excerpts_are_explicit_source_only_diagnostics_and_do_not_rewrite_the_index() {
    let corpus = corpus();
    let before = corpus.snapshot().to_owned();
    let context = corpus
        .oracle(
            &[("README.md".into(), "turn source into bytecode".into())],
            8000,
        )
        .unwrap();
    assert_eq!(context.evidence()[0].path, "README.md");
    assert!(
        context.evidence()[0]
            .text
            .contains("turn source into bytecode")
    );
    assert_eq!(before, corpus.snapshot());
    assert!(
        corpus
            .oracle(&[("README.md".into(), "missing evidence".into())], 8000)
            .is_err()
    );
}
#[test]
fn source_ranges_include_trailing_blank_lines_and_ids_are_not_used_as_semantic_grades() {
    let mut corpus = Corpus::new().unwrap();
    corpus
        .replace(vec![file("README.md", "needle\n\n")])
        .unwrap();
    let context = corpus.retrieve("needle", 1, 8000).unwrap();
    assert_eq!(context.evidence()[0].start_line, 1);
    assert_eq!(context.evidence()[0].end_line, 2);
    let answer = Answer {
        status: Status::Answered,
        answer: "An unrelated claim requiring manual rejection".into(),
        citations: vec![Citation {
            label: 1,
            quote: "needle".into(),
        }],
    };
    corpus.validate(&context, &answer).unwrap();
}

#[test]
fn split_identifiers_work_without_help_from_paths_or_comments() {
    let mut corpus = Corpus::new().unwrap();
    corpus
        .replace(vec![file(
            "a.rs",
            "fn parseWidgetPayload() {}\nfn calculate_packet_limit() {}\n",
        )])
        .unwrap();
    for question in ["widget", "payload", "packet", "limit"] {
        assert!(
            !corpus
                .retrieve(question, 1, 8000)
                .unwrap()
                .evidence()
                .is_empty(),
            "{question}"
        );
    }
}

#[test]
fn byte_counts_and_input_boundaries_are_enforced_before_publication() {
    let mut corpus = corpus();
    let before = corpus.snapshot().to_owned();
    let stats = corpus
        .replace(vec![file("one.txt", "123"), file("two.txt", "45678")])
        .unwrap();
    assert_eq!(stats.bytes, 8);
    let after = corpus.snapshot().to_owned();
    assert!(
        corpus
            .replace(vec![file("large.txt", &"x".repeat(2 * 1024 * 1024 + 1))])
            .is_err()
    );
    assert_eq!(corpus.snapshot(), after);
    assert!(corpus.replace(vec![file(&"p".repeat(4097), "")]).is_err());
    assert_eq!(corpus.snapshot(), after);
    corpus.replace(vec![file(&"p".repeat(4096), "")]).unwrap();
    assert_ne!(corpus.snapshot(), before);
    corpus
        .replace(vec![file("large.txt", &"x".repeat(2 * 1024 * 1024))])
        .unwrap();
    assert!(
        corpus
            .retrieve(&format!("needle{}", " ".repeat(4090)), 8, 16000)
            .is_ok()
    );
    assert!(corpus.retrieve("needle", 8, 16001).is_err());
}

#[test]
fn a_mid_file_excerpt_points_to_the_line_that_actually_contains_its_marker() {
    let mut corpus = Corpus::new().unwrap();
    let text = format!("{}\nneedle_marker\n\n", "padding\n".repeat(500));
    corpus.replace(vec![file("document.md", &text)]).unwrap();
    let context = corpus.retrieve("needle_marker", 1, 8000).unwrap();
    let source = &context.evidence()[0];
    let before = source
        .text
        .split("needle_marker")
        .next()
        .unwrap()
        .lines()
        .count();
    assert_eq!(source.start_line + before, 502);
}

#[test]
fn multiple_results_respect_the_total_budget_and_have_contiguous_labels() {
    let mut corpus = Corpus::new().unwrap();
    let text = format!("needle {}", "word ".repeat(280));
    corpus
        .replace(vec![file("alpha.md", &text), file("beta.md", &text)])
        .unwrap();
    let both = corpus.retrieve("needle", 6, 8000).unwrap();
    assert_eq!(both.evidence().len(), 2);
    assert_eq!(
        both.evidence().iter().map(|e| e.label).collect::<Vec<_>>(),
        [1, 2]
    );
    let limited = corpus.retrieve("needle", 6, 1800).unwrap();
    assert_eq!(limited.evidence().len(), 1);
    assert!(
        limited
            .evidence()
            .iter()
            .map(|e| e.text.len())
            .sum::<usize>()
            <= 1800
    );
    assert_eq!(
        corpus.retrieve("needle", 1, 8000).unwrap().evidence().len(),
        1
    );
}

#[test]
fn invalid_oracle_and_answer_bounds_fail_explicitly() {
    let corpus = corpus();
    let context = corpus.retrieve("readFile", 6, 8000).unwrap();
    assert!(
        corpus
            .oracle(&[("README.md".into(), String::new())], 8000)
            .is_err()
    );
    assert!(
        corpus
            .oracle(&[("README.md".into(), "x".repeat(1801))], 8000)
            .is_err()
    );
    assert!(corpus.oracle(&[], 0).is_err());
    assert!(corpus.oracle(&[], 16001).is_err());
    let excessive = (0..9)
        .map(|_| ("README.md".into(), "compiler".into()))
        .collect::<Vec<_>>();
    assert!(corpus.oracle(&excessive, 8000).is_err());
    let mut answer = Answer {
        status: Status::InsufficientEvidence,
        answer: " ".into(),
        citations: vec![],
    };
    assert!(corpus.validate(&context, &answer).is_err());
    answer.answer = "x".repeat(4097);
    assert!(corpus.validate(&context, &answer).is_err());
    answer.answer = "Insufficient supplied evidence".into();
    answer.citations = (0..9)
        .map(|_| Citation {
            label: 1,
            quote: "read bytes".into(),
        })
        .collect();
    assert!(corpus.validate(&context, &answer).is_err());
}

#[test]
fn namespace_matches_and_substantial_body_mentions_both_influence_lexical_order() {
    let mut c = Corpus::new().unwrap();
    c.replace(vec![
        file("needle/needle.txt", "noise"),
        file("other.txt", "needle needle needle"),
    ])
    .unwrap();
    assert_eq!(
        c.retrieve("needle", 1, 4000).unwrap().evidence()[0].path,
        "needle/needle.txt"
    );
    c.replace(vec![
        file("needle.txt", "noise noise"),
        file("other.txt", "needle needle needle"),
    ])
    .unwrap();
    assert_eq!(
        c.retrieve("needle", 1, 4000).unwrap().evidence()[0].path,
        "other.txt"
    );
}

#[test]
fn a_moderate_paragraph_with_several_mentions_can_outrank_one_isolated_mention() {
    let mut c = Corpus::new().unwrap();
    c.replace(vec![
        file(
            "long.txt",
            &format!("{}{}", "needle ".repeat(4), "plain ".repeat(24)),
        ),
        file("brief.txt", "needle plain plain"),
    ])
    .unwrap();
    assert_eq!(
        c.retrieve("needle", 1, 4000).unwrap().evidence()[0].path,
        "long.txt"
    );
    c.replace(vec![
        file(
            "long.txt",
            &format!("{}{}", "needle ".repeat(2), "plain ".repeat(20)),
        ),
        file("brief.txt", "needle plain plain plain"),
    ])
    .unwrap();
    assert_eq!(
        c.retrieve("needle", 1, 4000).unwrap().evidence()[0].path,
        "brief.txt"
    );
}

#[test]
fn standalone_same_path_same_size_edits_invalidate_results_and_held_context() {
    let mut c = Corpus::new().unwrap();
    c.replace(vec![file("src/item.rs", "fn first() {}\n")])
        .unwrap();
    let held = c.retrieve_compact("first", 1, 1000).unwrap();
    let before = c.snapshot().to_owned();
    let changed = c
        .replace(vec![file("src/item.rs", "fn other() {}\n")])
        .unwrap();
    assert_eq!(changed.changed, 1);
    assert_eq!(changed.reused, 0);
    assert_ne!(c.snapshot(), before);
    assert!(
        c.retrieve_compact("first", 1, 1000)
            .unwrap()
            .evidence()
            .is_empty()
    );
    assert!(
        c.retrieve_compact("other", 1, 1000).unwrap().evidence()[0]
            .text
            .contains("other")
    );
    assert!(held.evidence()[0].text.contains("first"));
    assert!(c.reorder(&held, &[1]).is_err());
}
