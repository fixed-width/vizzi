//! Explicit source packs select captured bytes rather than expanded anchor windows.
use vizzi_qa::{Corpus, IdAnswer, SourceFile, SourceRange, Status};

fn source(path: &str, text: &str) -> SourceFile {
    SourceFile {
        path: path.into(),
        text: text.into(),
    }
}
fn range(path: &str, text: &str, start: usize, end: usize) -> SourceRange {
    SourceRange {
        path: path.into(),
        content: blake3::hash(text.as_bytes()).to_hex().to_string(),
        start,
        end,
    }
}

#[test]
fn selects_complete_definitions_in_request_order_with_canonical_references() {
    let body = format!(
        "fn commit() {{\n{}    if rejected {{ return; }}\n    publish();\n}}\n",
        "    // preserved branch context\n".repeat(80)
    );
    let text = format!("// prefix 界\n{body}// suffix\n");
    let start = text.find("fn commit").unwrap();
    let other = "// caller\nfn submit() { commit(); }\n";
    let mut corpus = Corpus::new().unwrap();
    corpus
        .replace(vec![
            source("src/commit.rs", &text),
            source("src/caller.rs", other),
        ])
        .unwrap();
    let selections = [
        range("src/caller.rs", other, 10, other.len()),
        range("src/commit.rs", &text, start, start + body.len()),
    ];
    let context = corpus
        .select_ranges(corpus.snapshot(), &selections, 16000)
        .unwrap();
    assert_eq!(context.evidence().len(), 2);
    assert_eq!(context.evidence()[0].text, "fn submit() { commit(); }\n");
    assert_eq!(context.evidence()[1].text, body);
    assert!(context.evidence()[1].text.len() > 1800);
    assert_eq!(context.evidence()[1].start_line, 2);
    assert_eq!(context.evidence()[1].end_line, 85);
    let refs = corpus
        .validate_ids(
            &context,
            &IdAnswer {
                status: Status::Answered,
                answer: "submit calls commit; rejection prevents publication".into(),
                citations: vec![1, 2],
            },
        )
        .unwrap();
    assert_eq!(refs[1].path, "src/commit.rs");
    assert_eq!(refs[1].content, selections[1].content);
    assert_eq!(refs[1].snapshot, corpus.snapshot());
    let reordered = corpus
        .select_ranges(
            corpus.snapshot(),
            &[selections[1].clone(), selections[0].clone()],
            16000,
        )
        .unwrap();
    assert_eq!(context.evidence()[1].id, reordered.evidence()[0].id);
    assert_eq!(reordered.evidence()[0].label, 1);
}

#[test]
fn validates_snapshot_content_byte_bounds_utf8_and_exact_total_allowance() {
    let text = "a界\nb\n";
    let mut corpus = Corpus::new().unwrap();
    corpus.replace(vec![source("a.rs", text)]).unwrap();
    let valid = range("a.rs", text, 1, text.len());
    let snapshot = corpus.snapshot().to_owned();
    assert!(
        corpus
            .select_ranges(&snapshot, std::slice::from_ref(&valid), text.len() - 1)
            .is_ok()
    );
    assert!(
        corpus
            .select_ranges(&snapshot, std::slice::from_ref(&valid), text.len() - 2)
            .is_err()
    );
    assert!(
        corpus
            .select_ranges("old", std::slice::from_ref(&valid), 16000)
            .is_err()
    );
    let mut changed = valid.clone();
    changed.content = "0".repeat(64);
    assert!(corpus.select_ranges(&snapshot, &[changed], 16000).is_err());
    for (start, end) in [
        (0, 0),
        (4, 1),
        (0, text.len() + 1),
        (usize::MAX, usize::MAX),
        (2, text.len()),
        (0, 2),
    ] {
        assert!(
            corpus
                .select_ranges(&snapshot, &[range("a.rs", text, start, end)], 16000)
                .is_err(),
            "{start}..{end}"
        );
    }
    assert!(
        corpus
            .select_ranges(&snapshot, &[range("missing.rs", text, 0, 1)], 16000)
            .is_err()
    );
    for budget in [0, 16001] {
        assert!(
            corpus
                .select_ranges(&snapshot, std::slice::from_ref(&valid), budget)
                .is_err()
        );
    }
    corpus
        .replace(vec![
            source("a.rs", text),
            source("other.rs", "fn changed() {}"),
        ])
        .unwrap();
    assert!(corpus.select_ranges(&snapshot, &[valid], 16000).is_err());
}

#[test]
fn refuses_empty_excessive_or_overlapping_selections_but_allows_adjacency() {
    let text = "abcd\n";
    let mut corpus = Corpus::new().unwrap();
    corpus
        .replace(vec![source("a.rs", text), source("b.rs", text)])
        .unwrap();
    let snapshot = corpus.snapshot();
    assert!(corpus.select_ranges(snapshot, &[], 16000).is_err());
    for pair in [[(0, 3), (0, 3)], [(0, 4), (1, 2)], [(0, 3), (2, 4)]] {
        let ranges = pair.map(|(s, e)| range("a.rs", text, s, e));
        assert!(corpus.select_ranges(snapshot, &ranges, 16000).is_err());
    }
    let context = corpus
        .select_ranges(
            snapshot,
            &[
                range("a.rs", text, 0, 2),
                range("a.rs", text, 2, 4),
                range("b.rs", text, 0, 2),
            ],
            6,
        )
        .unwrap();
    assert_eq!(
        context
            .evidence()
            .iter()
            .map(|e| e.text.as_str())
            .collect::<Vec<_>>(),
        ["ab", "cd", "ab"]
    );
    let reverse = corpus
        .select_ranges(
            snapshot,
            &[range("a.rs", text, 2, 4), range("a.rs", text, 0, 2)],
            4,
        )
        .unwrap();
    assert_eq!(
        reverse
            .evidence()
            .iter()
            .map(|e| e.text.as_str())
            .collect::<Vec<_>>(),
        ["cd", "ab"]
    );
    let many = (0..9)
        .map(|i| range("a.rs", text, i % 4, i % 4 + 1))
        .collect::<Vec<_>>();
    assert!(corpus.select_ranges(snapshot, &many, 16000).is_err());
    let mut files = vec![];
    let mut ranges = vec![];
    for i in 0..8 {
        let path = format!("{i}.rs");
        files.push(source(&path, "x"));
        ranges.push(range(&path, "x", 0, 1));
    }
    corpus.replace(files).unwrap();
    assert_eq!(
        corpus
            .select_ranges(corpus.snapshot(), &ranges, 8)
            .unwrap()
            .evidence()
            .len(),
        8
    );
}
