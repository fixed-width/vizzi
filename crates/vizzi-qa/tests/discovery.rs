//! Natural-language operation words do not replace the topic being searched for.
use vizzi_qa::{Context, Corpus, SourceFile};

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
fn has(context: &Context, needle: &str) -> bool {
    context.evidence().iter().any(|e| e.text.contains(needle))
}

#[test]
fn creation_finds_a_constructor_and_owning_entry_instead_of_just_new_keyword_matches() {
    let c = corpus(vec![
        file(
            "a/helper_tests.rs",
            "fn new_message() { let _ = Message::new(); }\n",
        ),
        file(
            "a/tests.rs",
            "fn message_new() { let _ = Message::new(); }\n",
        ),
        file(
            "src/message.rs",
            "struct Message; impl Message { fn new() -> Self { Self } fn amend(&mut self) {} }\n",
        ),
        file(
            "src/session.rs",
            "fn open_attempt() { let message = Message::new(); consume(message); }\n",
        ),
        file("src/unrelated.rs", "fn new() { let created = new(); }\n"),
    ]);
    let result = c
        .retrieve_search("where are new messages created?", 8, 4000)
        .unwrap();
    assert!(has(&result, "fn new() -> Self"));
    assert!(has(&result, "fn open_attempt"));
    assert!(
        !result
            .evidence()
            .iter()
            .any(|e| e.path == "src/unrelated.rs")
    );
    assert_ne!(result.evidence()[0].path, "a/helper_tests.rs");
    assert!(result.evidence()[0].path.starts_with("src/"));
}

#[test]
fn extension_keeps_the_registration_mechanism_alongside_handler_examples() {
    let c = corpus(vec![
        file(
            "src/add_codec.rs",
            "fn add_codec(context: CodecContext) { let new_codec = context; }\n",
        ),
        file(
            "src/registry.rs",
            "macro_rules! codecs { ($name:ident) => { fn $name() {} }; }\ncodecs! { decimal }\n",
        ),
    ]);
    let result = c
        .retrieve_search("how do I add a new codec?", 4, 2000)
        .unwrap();
    assert!(has(&result, "macro_rules! codecs"));
    assert!(
        result
            .evidence()
            .iter()
            .any(|e| e.scope.as_ref().is_some_and(|s| s.kind == "macro"))
    );
    assert!(has(&result, "fn add_codec"));
}

#[test]
fn flow_keeps_submission_and_validation_with_the_requested_topic() {
    let c = corpus(vec![
        file(
            "doc/build.md",
            "batch codepath checks: formatting checks compilation checks.\n",
        ),
        file(
            "src/handler.rs",
            "fn dispatch_batch(batch: Batch) { submit_batch(batch); }\n",
        ),
        file(
            "src/checks.rs",
            "fn validate_batch(batch: Batch) { if batch.conflict() { reject(); } }\n",
        ),
        file(
            "src/collector.rs",
            "fn check_gc() { if pins() { reject(); } }\n",
        ),
    ]);
    let result = c
        .retrieve_search(
            "show me the codepath batches take with all checks they go through",
            8,
            4000,
        )
        .unwrap();
    assert!(has(&result, "fn dispatch_batch"));
    assert!(has(&result, "fn validate_batch"));
    assert!(
        !result
            .evidence()
            .iter()
            .any(|e| e.path == "src/collector.rs")
    );
}

#[test]
fn python_creation_uses_the_same_topic_and_operation_policy() {
    let c = corpus(vec![
        file(
            "package/parcel.py",
            "class Parcel:\n    def __init__(self):\n        self.ready = True\n\ndef open_parcel():\n    parcel = Parcel()\n    return parcel\n",
        ),
        file("package/other.py", "def new():\n    return object()\n"),
    ]);
    let result = c
        .retrieve_search("where are parcels created?", 8, 4000)
        .unwrap();
    assert!(has(&result, "class Parcel") || has(&result, "def __init__"));
    assert!(has(&result, "def open_parcel"));
    assert!(
        !result
            .evidence()
            .iter()
            .any(|e| e.path == "package/other.py")
    );
}

#[test]
fn bounded_identifier_derivations_find_handlers_without_matching_arbitrarily_long_prefixes() {
    let c = corpus(vec![
        file("src/requester.rs", "fn dispatch() { validate(); }\n"),
        file(
            "src/requestconfigurator.rs",
            "fn dispatch() { validate(); }\n",
        ),
    ]);
    let result = c
        .retrieve_search("trace requests through their checks", 8, 4000)
        .unwrap();
    assert_eq!(result.evidence().len(), 1);
    assert_eq!(result.evidence()[0].path, "src/requester.rs");
}

#[test]
fn repeated_identifier_forms_preserve_topic_membership() {
    let c = corpus(vec![file(
        "src/factory.rs",
        "fn parcel_new() -> Parcel { Parcel::new() }\n",
    )]);
    let found = c
        .retrieve_search("where are parcels created?", 8, 4000)
        .unwrap();
    assert!(has(&found, "fn parcel_new"));
}

#[test]
fn complementary_pools_keep_an_entry_visible_when_constructor_matches_are_crowded() {
    let c = corpus(vec![file(
        "src/parcel.rs",
        "impl ParcelA { fn new() {} }\nimpl ParcelB { fn new() {} }\nimpl ParcelC { fn new() {} }\nfn open_parcel() { let parcel = ParcelA::new(); }\n",
    )]);
    let found = c
        .retrieve_search("where are new parcels created?", 2, 1000)
        .unwrap();
    assert_eq!(found.evidence().len(), 2);
    assert!(has(&found, "fn open_parcel"));
    assert!(has(&found, "fn new()"));
    assert_eq!(
        c.retrieve_search("where are new parcels created?", 1, 1000)
            .unwrap()
            .evidence()
            .len(),
        1
    );
    for (limit, budget) in [(0, 4000), (9, 4000), (1, 0), (1, 16001)] {
        assert!(
            c.retrieve_search("where are parcels created?", limit, budget)
                .is_err()
        );
    }
    let question = format!("where are parcels created? {}", "padding ".repeat(600));
    assert!(c.retrieve_search(&question, 8, 4000).is_err());
}

#[test]
fn parent_scope_cues_retain_methods_beyond_the_first_container_excerpt() {
    let text = format!(
        "struct Handler; impl Handler {{ {} fn codec_adapter(&self) {{ consume(); }} }}",
        "// codec padding\n".repeat(400)
    );
    let c = corpus(vec![file("src/codec.rs", &text)]);
    let found = c
        .retrieve_search("how do I add a new codec?", 4, 2000)
        .unwrap();
    assert!(found.evidence().iter().any(|e| {
        e.scope
            .as_ref()
            .is_some_and(|s| s.name == "codec_adapter" && s.kind == "function")
    }));
}

#[test]
fn preview_allocation_retains_short_guarded_calls_and_bounds_long_bodies() {
    let short = format!(
        "fn open_parcel() {{ if allowed {{ {} let parcel = Parcel::new(); consume(parcel); }} }}\n",
        "// padding\n".repeat(20)
    );
    let long = format!(
        "fn parcel_new() {{ {} }}\n",
        "// parcel construction\n".repeat(40)
    );
    let c = corpus(vec![
        file("src/entry.rs", &short),
        file("src/parcel.rs", &long),
    ]);
    let found = c
        .retrieve_search("where are parcels created?", 8, 4000)
        .unwrap();
    assert!(found.evidence().iter().all(|e| e.text.len() <= 500));
    assert!(
        found
            .evidence()
            .iter()
            .any(|e| e.text.contains("if allowed") && e.text.contains("Parcel::new()"))
    );
}

#[test]
fn rendered_scopes_cannot_borrow_topic_or_operation_matches_from_neighbours() {
    let c = corpus(vec![
        file(
            "src/parcel.rs",
            "fn idle() {}\nfn dispatch_parcel(parcel: Parcel) { send(parcel); }\n",
        ),
        file(
            "src/mixed.rs",
            "fn check() {}\nfn validate_parcel(parcel: Parcel) { if parcel.conflict() { reject(); } }\n",
        ),
    ]);
    let found = c
        .retrieve_search("trace parcels through checks", 8, 4000)
        .unwrap();
    assert!(has(&found, "dispatch_parcel"));
    assert!(has(&found, "validate_parcel"));
    assert!(found.evidence().iter().all(|e| {
        !e.scope
            .as_ref()
            .is_some_and(|s| s.name == "idle" || s.name == "check")
    }));
}

#[test]
fn the_total_budget_applies_even_when_minimum_card_size_exceeds_a_fair_share() {
    let text = (0..8)
        .map(|n| format!("impl Parcel{n} {{ fn new() -> Self {{ Self }} }}\n"))
        .collect::<String>();
    let c = corpus(vec![file("src/parcel.rs", &text)]);
    for budget in [40, 80, 128, 200] {
        let found = c
            .retrieve_search("where are new parcels created?", 8, budget)
            .unwrap();
        assert!(found.evidence().iter().map(|e| e.text.len()).sum::<usize>() <= budget);
    }
}

#[test]
fn exact_budget_fits_are_kept_and_multibyte_floors_never_publish_empty_cards() {
    let text = "fn open_parcel() { let parcel = Parcel::new(); }";
    let c = corpus(vec![file("src/parcel.rs", text)]);
    let found = c
        .retrieve_search("where are parcels created?", 1, text.len())
        .unwrap();
    assert_eq!(found.evidence().len(), 1);
    assert_eq!(found.evidence()[0].text, text);
    let unicode = format!(
        "fn open_parcel() {{ /*{} start begin allocate parcel {}*/ }}\n",
        "界".repeat(650),
        "界".repeat(1500)
    );
    let c = corpus(vec![file("src/parcel.rs", &unicode)]);
    let found = c
        .retrieve_search("where are parcels created?", 8, 1)
        .unwrap();
    assert!(found.evidence().iter().all(|e| !e.text.is_empty()));
    assert!(found.evidence().iter().map(|e| e.text.len()).sum::<usize>() <= 1);
}

#[test]
fn overlap_suppression_distinguishes_partial_duplicates_from_half_shared_context() {
    for (offset, length, expected) in [(100, 450, 1), (300, 400, 2)] {
        let before = "impl Parcel {\n";
        let start = "fn new() -> Self { /*";
        let end = "*/ Self }";
        let function = format!(
            "{start}{}{end}",
            "x".repeat(length - start.len() - end.len())
        );
        let text = format!(
            "{before}{}{function}\n}}\n",
            " ".repeat(offset - before.len())
        );
        let c = corpus(vec![file("src/parcel.rs", &text)]);
        let found = c
            .retrieve_search("where are new parcels created?", 4, 2000)
            .unwrap();
        assert_eq!(
            found.evidence().len(),
            expected,
            "offset {offset}, length {length}"
        );
        assert!(has(&found, "fn new()"));
    }
}

#[test]
fn unsupported_question_forms_preserve_compact_and_unknown_topics_remain_empty() {
    let c = corpus(vec![
        file("README.md", "A message transport project."),
        file("src/a.rs", "fn new() {}\n"),
    ]);
    let question = "What is this project intended for?";
    assert_eq!(
        serde_json::to_value(c.retrieve_search(question, 8, 4000).unwrap()).unwrap(),
        serde_json::to_value(c.retrieve_compact(question, 8, 4000).unwrap()).unwrap()
    );
    assert!(
        c.retrieve_search("where are missingwidgets created?", 8, 4000)
            .unwrap()
            .evidence()
            .is_empty()
    );
    for (q, n, b) in [
        ("", 8, 4000),
        ("x", 0, 4000),
        ("x", 9, 4000),
        ("x", 1, 0),
        ("x", 1, 16001),
    ] {
        assert!(c.retrieve_search(q, n, b).is_err());
    }
}

#[test]
fn file_paths_and_extensions_do_not_mistakenly_request_extension_or_call_flow_facets() {
    let c = corpus(vec![
        file(
            "src/loader.rs",
            "fn retry_path() { let extension = \"c\"; read_bytes(); }\n",
        ),
        file(
            "src/builder.rs",
            "fn create_loader() { register_extension(); }\n",
        ),
    ]);
    for question in [
        "When does the loader retry a source path with the c extension?",
        "Does checking a source path require reading bytes?",
    ] {
        assert_eq!(
            serde_json::to_value(c.retrieve_search(question, 8, 4000).unwrap()).unwrap(),
            serde_json::to_value(c.retrieve_compact(question, 8, 4000).unwrap()).unwrap()
        );
    }
}

#[test]
fn search_preserves_literal_bounds_and_held_context_invalidation() {
    let text = "fn open_界parcel() { let parcel = Parcel::new(); /* 界界界 */ }\n";
    let mut c = corpus(vec![file("src/parcel.rs", text)]);
    for budget in [1, 2, 3, 16, 80, 4000] {
        let found = c
            .retrieve_search("where are parcels created?", 1, budget)
            .unwrap();
        assert!(found.evidence().len() <= 1);
        assert!(found.evidence().iter().map(|e| e.text.len()).sum::<usize>() <= budget);
        for e in found.evidence() {
            assert!(text.contains(&e.text));
        }
    }
    let held = c
        .retrieve_search("where are parcels created?", 8, 4000)
        .unwrap();
    assert!(!held.evidence().is_empty());
    let labels = held.evidence().iter().map(|e| e.label).collect::<Vec<_>>();
    assert!(c.reorder(&held, &labels).is_ok());
    c.replace(vec![file("src/parcel.rs", "fn replaced() {}\n")])
        .unwrap();
    assert!(c.reorder(&held, &labels).is_err());
    assert!(
        c.retrieve_search("where are parcels created?", 8, 4000)
            .unwrap()
            .evidence()
            .is_empty()
    );
}
