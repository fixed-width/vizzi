//! Operation roles separate external entry evidence from ordinary owned members.
use vizzi_qa::{Corpus, SourceFile};
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

#[test]
fn ordinary_owned_members_do_not_crowd_out_an_external_creation_entry() {
    let noise = (0..16)
        .map(|n| {
            format!(
                "fn member_{n}(&self) {{ let cache = Store::new(); make_mut(cache); {} }}\n",
                "// padding\n".repeat(20)
            )
        })
        .collect::<String>();
    let source = format!("struct Parcel; impl Parcel {{ fn new() -> Self {{ Self }} {noise} }}\n");
    let c = corpus(vec![
        file("src/a.rs", &source),
        file(
            "src/z.rs",
            "impl Runner { fn open_attempt() { start(); let parcel = Parcel::new(); } }\n",
        ),
    ]);
    let result = c
        .retrieve_search("where are new parcels created?", 4, 2000)
        .unwrap();
    assert!(
        result
            .evidence()
            .iter()
            .any(|e| e.text.contains("fn open_attempt"))
    );
    assert!(
        result
            .evidence()
            .iter()
            .any(|e| e.text.contains("fn new() -> Self"))
    );
}

#[test]
fn external_dispatch_and_validation_survive_owned_publication_helpers() {
    let noise = (0..20)
        .map(|n| {
            format!(
                "fn publish_{n}(&self) {{ publish(); check_cache(); {} }}\n",
                "// padding\n".repeat(20)
            )
        })
        .collect::<String>();
    let c = corpus(vec![
        file(
            "src/a.rs",
            &format!("struct Batch; impl Batch {{ {noise} }}\n"),
        ),
        file(
            "src/m.rs",
            "fn dispatch_batch(batch: Batch) { submit(batch); send(batch); publish(batch); }\n",
        ),
        file(
            "src/z.rs",
            "fn validate_batch(batch: Batch) { check(batch); if conflict(batch) { reject(); } }\n",
        ),
    ]);
    let result = c
        .retrieve_search("trace batch commits through checks", 4, 2000)
        .unwrap();
    assert!(
        result
            .evidence()
            .iter()
            .any(|e| e.text.contains("fn dispatch_batch"))
    );
    assert!(
        result
            .evidence()
            .iter()
            .any(|e| e.text.contains("fn validate_batch"))
    );
}

#[test]
fn an_ancestor_operation_word_is_context_rather_than_a_direct_operation_name() {
    let c = corpus(vec![
        file(
            "src/a.rs",
            "struct Validation; impl Validation { fn batch_data(&self) { check(); conflict(); reject(); } }\n",
        ),
        file(
            "src/z.rs",
            "fn validate_batch(batch: Batch) { check(batch); }\n",
        ),
    ]);
    let result = c
        .retrieve_search("trace batches through checks", 2, 1000)
        .unwrap();
    assert_eq!(result.evidence()[0].path, "src/z.rs");
}

#[test]
fn macro_registration_remains_a_structural_definition_signal() {
    let handlers = (0..6)
        .map(|n| {
            format!("fn add_codec_{n}(context: CodecContext) {{ let new_codec = context; }}\n")
        })
        .collect::<String>();
    let c = corpus(vec![
        file("src/a.rs", &handlers),
        file(
            "src/m.rs",
            "fn register_codec(context: CodecContext) { register(context); }\n",
        ),
        file(
            "src/z.rs",
            "macro_rules! codecs { ($name:ident) => { fn $name() {} }; }\n",
        ),
    ]);
    assert!(
        c.retrieve_compact("how do I add a new codec?", 2, 1000)
            .unwrap()
            .evidence()
            .iter()
            .all(|e| !e.text.contains("macro_rules! codecs"))
    );
    let result = c
        .retrieve_search("how do I add a new codec?", 2, 1000)
        .unwrap();
    assert!(result.evidence()[0].text.contains("macro_rules! codecs"));
}
