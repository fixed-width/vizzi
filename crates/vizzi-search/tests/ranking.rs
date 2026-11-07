//! Reordering preserves held evidence and cannot turn model labels into source facts.
use vizzi_search::{Corpus, SourceFile};

fn corpus() -> Corpus {
    let mut corpus = Corpus::new().unwrap();
    corpus
        .replace(vec![
            SourceFile {
                path: "src/transaction.rs".into(),
                text: "pub fn create_transaction() {}\n".into(),
            },
            SourceFile {
                path: "tests/transaction.rs".into(),
                text: "fn transaction_example() {}\n".into(),
            },
        ])
        .unwrap();
    corpus
}

#[test]
fn complete_orders_resolve_original_excerpts_without_relabelling() {
    let corpus = corpus();
    let context = corpus.retrieve_compact("transaction", 8, 4000).unwrap();
    assert_eq!(context.evidence().len(), 2);
    let ranked = corpus.reorder(&context, &[2, 1]).unwrap();
    for (actual, expected) in ranked.iter().zip(context.evidence().iter().rev()) {
        assert!(std::ptr::eq(*actual, expected));
        assert_eq!(actual.label, expected.label);
    }
    assert_eq!(corpus.reorder(&context, &[1, 2]).unwrap()[0].label, 1);
    for order in [
        vec![],
        vec![1],
        vec![1, 2, 3],
        vec![1, 1],
        vec![0, 2],
        vec![1, 3],
        vec![usize::MAX, 1],
    ] {
        assert!(corpus.reorder(&context, &order).is_err(), "{order:?}");
    }
    assert_eq!(context.evidence()[0].label, 1);
}

#[test]
fn no_matches_and_source_replacement_have_explicit_order_semantics() {
    let mut corpus = corpus();
    let empty = corpus.retrieve_compact("missing_54ce37", 8, 4000).unwrap();
    assert!(corpus.reorder(&empty, &[]).unwrap().is_empty());
    assert!(corpus.reorder(&empty, &[1]).is_err());
    let held = corpus.retrieve_compact("transaction", 8, 4000).unwrap();
    corpus
        .replace(vec![SourceFile {
            path: "src/transaction.rs".into(),
            text: "pub fn transaction_replaced() {}\n".into(),
        }])
        .unwrap();
    assert!(
        corpus
            .reorder(&held, &[1, 2])
            .unwrap_err()
            .to_string()
            .contains("different source snapshot")
    );
    assert!(corpus.reorder(&empty, &[]).is_err());
}
