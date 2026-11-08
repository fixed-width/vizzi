//! Source-local Rust receiver bindings through public analysis/projection interfaces.
use std::sync::Arc;
use vizzi_core::{
    model::{CapturePolicyId, Document, Snapshot, WorkspaceId},
    policy::{Classification, Provenance},
    rust::RustAnalyzer,
    structure::{Invocation, Structure},
};

fn index(text: &str) -> Structure {
    let workspace = WorkspaceId::derive(&[b"self-targets"]);
    let mut analyzer = RustAnalyzer::new().unwrap();
    let docs = [("main.rs", text), ("unrelated.rs", "fn render() {}")]
        .into_iter()
        .map(|(path, text)| {
            Arc::new(
                Document::capture(
                    workspace.clone(),
                    path.into(),
                    text.as_bytes(),
                    vec![analyzer.analyze(text.as_bytes()).unwrap()],
                    Classification::Assigned {
                        language: "rust".into(),
                        provenance: Provenance::Extension {
                            extension: "rs".into(),
                        },
                    },
                )
                .unwrap(),
            )
        })
        .collect();
    Structure::new(&Snapshot::new(workspace, docs, CapturePolicyId::derive(&[b"fixture"])).unwrap())
}
fn outgoing(index: &Structure) -> Vec<Invocation> {
    let inventory = index.inventory("main.rs", "process", 0).unwrap();
    let process = inventory
        .items
        .iter()
        .find(|e| e.symbol.kind == "function")
        .unwrap();
    index
        .outgoing(&process.symbol.id, "main.rs", 0)
        .unwrap()
        .items
}

#[test]
fn self_call_in_trait_impl_opens_the_concrete_types_inherent_method() {
    let index = index(
        "struct Minitosis; struct Other; impl Other { fn render(&mut self) {} } impl Minitosis { fn render(&mut self) {} } impl Plugin for Minitosis { fn process(&mut self) { self.render(); } }",
    );
    let calls = outgoing(&index);
    assert_eq!(calls[0].candidate_count, 1);
    assert_eq!(
        calls[0].binding,
        Some(vizzi_core::calls::TargetBasis::RustSelfReceiver)
    );
    assert_eq!(calls[0].candidates[0].parents, ["Minitosis"]);
    // Name inventories retain the actual number of declarations sharing the name.
    assert_eq!(
        index.inventory("main.rs", "render", 0).unwrap().items[0].same_name_definitions,
        3
    );
}

#[test]
fn same_named_types_in_different_lexical_scopes_are_not_conflated() {
    let index = index(
        "mod other { struct Widget; impl Widget { fn render(&self) {} } } struct Widget; impl Widget { fn render(&self) {} fn process(&self) { self.render(); } }",
    );
    let calls = outgoing(&index);
    assert_eq!(calls[0].candidate_count, 1);
    assert_eq!(calls[0].candidates[0].parents, ["Widget"]);
}

#[test]
fn a_field_or_an_unknown_receiver_keeps_its_choices() {
    let index = index(
        "struct Widget; impl Widget { fn render(&mut self) {} fn process(&mut self, other: &mut Other) { self.child.render(); other.render(); } } struct Other; impl Other { fn render(&mut self) {} }",
    );
    let calls = outgoing(&index);
    assert!(calls.iter().all(|c| c.candidate_count == 3));
}

#[test]
fn shorthand_receiver_forms_bind_only_when_the_exact_receiver_matches() {
    for receiver in ["self", "&self", "&mut self", "&'a self", "&'a mut self"] {
        let text = format!(
            "struct Widget; impl Widget {{ fn process({receiver}) {{ self.render(); }} fn render({receiver}) {{}} }}"
        );
        assert_eq!(outgoing(&index(&text))[0].candidate_count, 1, "{receiver}");
    }
    for (caller, target) in [
        ("&self", "&mut self"),
        ("self", "&self"),
        ("&mut self", "&self"),
        ("self: Box<Self>", "&self"),
    ] {
        let text = format!(
            "struct Widget; impl Widget {{ fn process({caller}) {{ self.render(); }} fn render({target}) {{}} }}"
        );
        assert_eq!(
            outgoing(&index(&text))[0].candidate_count,
            2,
            "{caller} -> {target}"
        );
    }
}

#[test]
fn conditional_or_duplicate_methods_do_not_become_definite_targets() {
    for methods in [
        "#[cfg(feature=\"a\")] fn render(&self) {}",
        "fn render(&self) {} fn render(&self) {}",
        "fn render(&self) {} #[cfg(feature=\"a\")] fn render(&self) {}",
    ] {
        let text = format!(
            "struct Widget; impl Widget {{ fn process(&self) {{ self.render(); }} {methods} }}"
        );
        assert!(outgoing(&index(&text))[0].candidate_count > 1);
    }
}

#[test]
fn comments_turbofish_and_recursion_keep_the_source_receiver() {
    for expression in ["self /* receiver */ . render()", "self.render::<u8>()"] {
        let text = format!(
            "struct Widget; impl Widget {{ fn process(&mut self) {{ {expression}; }} fn render<T>(&mut self) {{}} }}"
        );
        assert_eq!(outgoing(&index(&text))[0].candidate_count, 1);
    }
    let index = index("struct Widget; impl Widget { fn process(&self) { self.process(); } }");
    let call = outgoing(&index).remove(0);
    assert_eq!(call.candidates[0].id, call.owner.unwrap());
}

#[test]
fn uncertain_syntax_or_self_type_does_not_bind() {
    for text in [
        "struct Widget<T>(T); impl<T> Widget<T> { fn process(&self) { self.render(); } fn render(&self) {} }",
        "struct Widget<T>(T); impl Widget<u8> { fn process(&self) { self.render(); } fn render(&self) {} }",
        "struct Widget; #[cfg(feature=\"x\")] impl Widget { fn process(&self) { self.render(); } fn render(&self) {} }",
        "struct Widget; impl Widget { #[instrument] fn process(&self) { self.render(); } fn render(&self) {} }",
        "struct Widget; impl Trait for Widget { fn process(&self) { self.render(); } fn render(&self) {} }",
        "struct Widget; impl Widget { fn process(&self) { fn inner() { self.render(); } } fn render(&self) {} }",
    ] {
        let analysis = RustAnalyzer::new()
            .unwrap()
            .analyze(text.as_bytes())
            .unwrap();
        assert!(
            analysis
                .calls
                .unwrap()
                .sites
                .iter()
                .all(|c| c.target.is_none()),
            "{text}"
        );
    }
}

#[test]
fn source_local_destination_is_validated_and_round_trips_without_inventing_legacy_bindings() {
    use vizzi_core::model::{Analysis, Coverage};
    let text = b"struct Widget; struct render; impl Widget { fn process(&self) { self.render(); } fn render(&self) {} }";
    let analysis = RustAnalyzer::new().unwrap().analyze(text).unwrap();
    let encoded = serde_json::to_value(&analysis).unwrap();
    let restored: Analysis = serde_json::from_value(encoded.clone()).unwrap();
    assert_eq!(restored.calls, analysis.calls);
    let mut legacy = encoded;
    legacy["calls"]["sites"][0]
        .as_object_mut()
        .unwrap()
        .remove("target");
    assert!(
        serde_json::from_value::<Analysis>(legacy)
            .unwrap()
            .calls
            .unwrap()
            .sites[0]
            .target
            .is_none()
    );
    let class = Classification::Assigned {
        language: "rust".into(),
        provenance: Provenance::Extension {
            extension: "rs".into(),
        },
    };
    for mode in 0..5 {
        let mut forged = analysis.clone();
        let target = forged.calls.as_mut().unwrap().sites[0]
            .target
            .as_mut()
            .unwrap();
        match mode {
            0 => target.declaration = usize::MAX,
            1 => target.declaration = 0,
            2 => target.declaration = 1,
            3 => forged.calls.as_mut().unwrap().sites[0].owner = None,
            _ => forged.coverage = Coverage::Partial,
        }
        let error = Document::capture(
            WorkspaceId::derive(&[b"forged"]),
            "a.rs".into(),
            text,
            vec![forged],
            class.clone(),
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("invalid source-local call destination"),
            "{mode}: {error}"
        );
    }
}

#[test]
fn conditional_module_and_inner_attributes_do_not_become_definite_bindings() {
    for text in [
        "#[cfg(feature=\"x\")] mod conditional { struct Widget; impl Widget { fn process(&self) { self.render(); } fn render(&self) {} } }",
        "#![cfg(feature=\"x\")] struct Widget; impl Widget { fn process(&self) { self.render(); } fn render(&self) {} }",
        "struct Widget; impl Widget { #![cfg(feature=\"x\")] fn other() {} fn process(&self) { self.render(); } fn render(&self) {} }",
    ] {
        let analysis = RustAnalyzer::new()
            .unwrap()
            .analyze(text.as_bytes())
            .unwrap();
        assert!(
            analysis
                .calls
                .unwrap()
                .sites
                .iter()
                .all(|c| c.target.is_none()),
            "{text}"
        );
    }
}

#[test]
fn python_cannot_claim_a_rust_receiver_binding() {
    use vizzi_core::{
        analysis::LanguageAdapter,
        calls::{CallTarget, TargetBasis},
        python::PythonAdapter,
    };
    let text = b"def render():\n    pass\ndef process():\n    render()\n";
    let mut analysis = PythonAdapter.create().unwrap().analyze(text).unwrap();
    assert!(analysis.calls.as_ref().unwrap().sites[0].target.is_none());
    analysis.calls.as_mut().unwrap().sites[0].target = Some(CallTarget {
        declaration: 0,
        basis: TargetBasis::RustSelfReceiver,
    });
    let class = Classification::Assigned {
        language: "python".into(),
        provenance: Provenance::Extension {
            extension: "py".into(),
        },
    };
    let error = Document::capture(
        WorkspaceId::derive(&[b"python"]),
        "a.py".into(),
        text,
        vec![analysis],
        class,
    )
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("invalid source-local call destination")
    );
}

#[test]
fn comments_around_attributes_do_not_hide_conditional_compilation() {
    for source in [
        "#![cfg(feature=\"x\")] // root\nstruct Widget; impl Widget { fn process(&self) { self.render(); } fn render(&self) {} }",
        "// root\n#![cfg(feature=\"x\")] struct Widget; impl Widget { fn process(&self) { self.render(); } fn render(&self) {} }",
        "struct Widget; impl Widget { fn process(&self) { self.render(); } #[cfg(feature=\"x\")] /* between */ fn render(&self) {} }",
    ] {
        let analysis = RustAnalyzer::new()
            .unwrap()
            .analyze(source.as_bytes())
            .unwrap();
        assert!(
            analysis
                .calls
                .unwrap()
                .sites
                .iter()
                .all(|c| c.target.is_none())
        );
    }
}

#[test]
fn partial_source_does_not_claim_source_local_bindings() {
    let analysis = RustAnalyzer::new().unwrap().analyze(b"struct Widget; impl Widget { fn process(&self) { self.render(); } fn render(&self) {}\nfn broken(").unwrap();
    assert_eq!(analysis.coverage, vizzi_core::model::Coverage::Partial);
    assert!(
        analysis
            .calls
            .unwrap()
            .sites
            .iter()
            .all(|c| c.target.is_none())
    );
}

#[test]
fn a_binding_is_independent_of_display_area_and_keeps_call_site_scope() {
    let index = index(
        "struct Widget; impl Widget { fn process(&self) { self.render(); } fn render(&self) {} }",
    );
    let local = outgoing(&index).remove(0);
    assert!(!local.outside_area);
    let elsewhere = index
        .outgoing(local.owner.as_deref().unwrap(), "unrelated.rs", 0)
        .unwrap()
        .items
        .remove(0);
    assert!(elsewhere.outside_area);
    assert_eq!(elsewhere.candidates[0].id, local.candidates[0].id);
    assert_eq!(elsewhere.candidate_count, 1);
}
