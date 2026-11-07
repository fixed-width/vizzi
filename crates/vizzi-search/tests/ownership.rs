//! Captured owners carry information absent from constructor bodies and filenames.
use vizzi_search::{Context, Corpus, SourceFile};
fn file(path: &str, text: &str) -> SourceFile {
    SourceFile {
        path: path.into(),
        text: text.into(),
    }
}
fn corpus(files: Vec<SourceFile>) -> Corpus {
    let mut c = Corpus::new().unwrap();
    c.replace(files).unwrap();
    c
}
fn has(c: &Context, text: &str) -> bool {
    c.evidence().iter().any(|e| e.text.contains(text))
}

#[test]
fn a_captured_owner_beats_parameter_mentions_even_without_a_matching_filename() {
    let wrappers=(0..12).map(|n|format!("struct ParcelHandle{n}; impl ParcelHandle{n} {{ fn new(parcel: Parcel) -> Self {{ Self }} }}\n")).collect::<String>();
    let definition = format!(
        "struct Parcel {{ seed: Seed }} impl Parcel {{ fn new(seed: Seed) -> Self {{ Self {{ seed }} }} {} }}",
        "// filler\n".repeat(250)
    );
    let c = corpus(vec![
        file("src/a.rs", &wrappers),
        file("src/z.rs", &definition),
    ]);
    let found = c
        .retrieve_search("where are new parcels created?", 8, 4000)
        .unwrap();
    assert!(has(&found, "fn new(seed: Seed)"));
    assert_eq!(found.evidence()[0].path, "src/z.rs");
}

#[test]
fn python_implicit_receivers_use_captured_owners_in_generic_files() {
    let wrappers=(0..12).map(|n|format!("class ParcelHandle{n}:\n    def __init__(self, parcel: Parcel):\n        self.parcel = parcel\n\n")).collect::<String>();
    let definition = format!(
        "class Parcel:\n    def __init__(self, seed):\n        self.seed = seed\n{}",
        "        # filler\n".repeat(250)
    );
    let c = corpus(vec![
        file("package/a.py", &wrappers),
        file("package/z.py", &definition),
    ]);
    let found = c
        .retrieve_search("where are new parcels created?", 8, 4000)
        .unwrap();
    assert_eq!(found.evidence()[0].path, "package/z.py");
    assert!(has(&found, "def __init__(self, seed)"));
}

#[test]
fn distinct_entry_names_survive_a_pool_of_repeated_lifecycle_methods() {
    let repeated=(0..12).map(|n|format!("impl Driver{n} {{ fn begin_attempt() {{ start(); let parcel = Parcel::new(); }} }}\n")).collect::<String>();
    let c = corpus(vec![
        file("src/a.rs", &repeated),
        file(
            "src/m.rs",
            "struct Parcel; impl Parcel { fn new() -> Self { Self } }\n",
        ),
        file(
            "src/z.rs",
            "impl Runner { fn open_attempt() { start(); let parcel = Parcel::new(); } }\n",
        ),
    ]);
    let found = c
        .retrieve_search("where are new parcels created?", 4, 2000)
        .unwrap();
    assert!(has(&found, "fn open_attempt"));
    assert!(has(&found, "fn new() -> Self"));
}

#[test]
fn matching_baseline_anchors_keep_their_original_source_identity() {
    let text = "struct Parcel; impl Parcel { fn new() -> Self { Self } }\n";
    let mut c = corpus(vec![file("src/factory.rs", text)]);
    let baseline = c
        .retrieve_compact("where are new parcels created?", 8, 4000)
        .unwrap();
    let found = c
        .retrieve_search("where are new parcels created?", 8, 4000)
        .unwrap();
    let anchor = baseline
        .evidence()
        .iter()
        .find(|e| e.text.contains("fn new"))
        .unwrap();
    let retained = found.evidence().iter().find(|e| e.id == anchor.id).unwrap();
    let mut expected = serde_json::to_value(anchor).unwrap();
    let mut actual = serde_json::to_value(retained).unwrap();
    expected.as_object_mut().unwrap().remove("label");
    actual.as_object_mut().unwrap().remove("label");
    assert_eq!(actual, expected);
    assert!(actual.get("range").is_none());
    for budget in [1, 8, 80, 4000] {
        let small = c
            .retrieve_search("where are new parcels created?", 1, budget)
            .unwrap();
        assert!(small.evidence().len() <= 1);
        assert!(small.evidence().iter().map(|e| e.text.len()).sum::<usize>() <= budget);
    }
    let labels = found.evidence().iter().map(|e| e.label).collect::<Vec<_>>();
    assert_eq!(labels, (1..=found.evidence().len()).collect::<Vec<_>>());
    assert!(c.reorder(&found, &labels).is_ok());
    c.replace(vec![file("src/factory.rs", "fn replaced() {}\n")])
        .unwrap();
    assert!(c.reorder(&found, &labels).is_err());
}

#[test]
fn same_name_results_remain_available_after_distinct_names_are_considered() {
    let files = (0..6)
        .map(|n| {
            file(
                &format!("src/driver{n}.rs"),
                "fn begin_attempt() { let parcel = Parcel::new(); }\n",
            )
        })
        .collect();
    let c = corpus(files);
    let found = c
        .retrieve_search("where are new parcels created?", 8, 4000)
        .unwrap();
    assert!(
        found
            .evidence()
            .iter()
            .filter(|e| e.scope.as_ref().is_some_and(|s| s.name == "begin_attempt"))
            .count()
            >= 3
    );
}

#[test]
fn underscore_identifiers_remain_complete_owner_names() {
    let wrappers = (0..12)
        .map(|n| format!("impl Wrapper{n} {{ fn new(parcel: Parcel_Box) -> Self {{ Self }} }}\n"))
        .collect::<String>();
    let subject = format!(
        "struct Parcel_Box; impl Parcel_Box {{ fn new(seed: Seed) -> Self {{ Self }} {} }}",
        "// filler\n".repeat(250)
    );
    let c = corpus(vec![
        file("src/a.rs", &wrappers),
        file("src/z.rs", &subject),
    ]);
    let found = c
        .retrieve_search("where are new Parcel_Box created?", 8, 4000)
        .unwrap();
    assert_eq!(found.evidence()[0].path, "src/z.rs");
    assert!(has(&found, "fn new(seed: Seed)"));
}

#[test]
fn repeated_ancestor_names_do_not_cancel_a_baseline_anchor() {
    let text = "mod parcel { struct Parcel; impl Parcel { fn new() -> Self { Self } } #[cfg(test)] mod tests { fn sample() { let value = Parcel::new(); } } }\n";
    let c = corpus(vec![file("src/entity.rs", text)]);
    let baseline = c
        .retrieve_compact("where are new parcels created?", 8, 4000)
        .unwrap();
    let anchor = baseline
        .evidence()
        .iter()
        .find(|e| {
            e.scope
                .as_ref()
                .is_some_and(|s| s.kind == "implementation" && s.name == "Parcel")
        })
        .unwrap();
    let found = c
        .retrieve_search("where are new parcels created?", 8, 4000)
        .unwrap();
    assert!(found.evidence().iter().any(|e| e.id == anchor.id));
}

#[test]
fn a_test_only_owner_cannot_become_the_protected_baseline_anchor() {
    for path in ["a/tests.rs", "a/parcel_tests.rs"] {
        let text = "struct Parcel; impl Parcel { fn new() -> Self { Self } }\n";
        let c = corpus(vec![file(path, text), file("src/z.rs", text)]);
        let found = c
            .retrieve_search("where are new parcels created?", 8, 4000)
            .unwrap();
        assert_eq!(found.evidence()[0].path, "src/z.rs");
    }
}

#[test]
fn a_topic_only_declaration_does_not_use_an_operation_anchor_slot() {
    let c = corpus(vec![
        file("src/a.rs", "struct Parcel { marker_state: State }\n"),
        file(
            "src/z.rs",
            "fn open_attempt() { let parcel = Parcel::new(); }\n",
        ),
    ]);
    assert!(
        c.retrieve_compact("where are new parcels created?", 8, 4000)
            .unwrap()
            .evidence()
            .iter()
            .any(|e| e.path == "src/a.rs")
    );
    let found = c
        .retrieve_search("where are new parcels created?", 8, 4000)
        .unwrap();
    assert!(found.evidence().iter().all(|e| e.path != "src/a.rs"));
    assert!(has(&found, "fn open_attempt"));
}

#[test]
fn retained_anchor_bytes_count_toward_small_total_budgets() {
    let mut files = vec![file(
        "src/a.rs",
        "struct Parcel; impl Parcel { fn new() -> Self { Self } }\n",
    )];
    files.extend((0..6).map(|n| {
        file(
            &format!("src/caller{n}.rs"),
            "fn open_attempt() { let parcel = Parcel::new(); consume(parcel); }\n",
        )
    }));
    let c = corpus(files);
    for budget in [80, 128, 200] {
        let found = c
            .retrieve_search("where are new parcels created?", 8, budget)
            .unwrap();
        assert!(found.evidence().iter().map(|e| e.text.len()).sum::<usize>() <= budget);
    }
}

#[test]
fn one_topic_cannot_fill_the_entire_anchor_reservation() {
    let mut files = vec![file(
        "src/a0.rs",
        "impl Parcel { fn new() -> Self { Self } }\n",
    )];
    files.extend((1..8).map(|n| {
        file(
            &format!("src/a{n}.rs"),
            &format!("impl Parcel {{ fn construct_{n}() -> Self {{ Self }} }}\n"),
        )
    }));
    files.push(file(
        "src/z.rs",
        "fn open_attempt() { start(); let parcel = Parcel::new(); }\n",
    ));
    let c = corpus(files);
    let found = c
        .retrieve_search("where are new parcels created?", 4, 2000)
        .unwrap();
    assert!(has(&found, "fn open_attempt"));
}

#[test]
fn multiple_topics_retain_their_separate_baseline_identities() {
    let first = format!(
        "struct Parcel; impl Parcel {{ fn constructor_first_init() {{}} fn new() -> Self {{ Self }} {} }}\n",
        "// padding\n".repeat(80)
    );
    let second = format!(
        "struct Packet; impl Packet {{ fn constructor_second_init() {{}} fn new() -> Self {{ Self }} {} }}\n",
        "// padding\n".repeat(80)
    );
    let c = corpus(vec![file("src/a.rs", &first), file("src/z.rs", &second)]);
    let question = "where are new parcels and packets created?";
    let baseline = c.retrieve_compact(question, 8, 4000).unwrap();
    let found = c.retrieve_search(question, 8, 4000).unwrap();
    let anchors = baseline
        .evidence()
        .iter()
        .filter(|e| e.scope.as_ref().is_some_and(|s| s.kind == "implementation"))
        .collect::<Vec<_>>();
    assert_eq!(anchors.len(), 2);
    for anchor in anchors {
        assert!(found.evidence().iter().any(|e| e.id == anchor.id));
    }
}
