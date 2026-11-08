//! Call evidence is captured from syntax, not inferred from text matches.
use vizzi_core::{
    analysis::LanguageAdapter,
    model::{Document, WorkspaceId},
    policy::{Classification, Provenance},
    python::PythonAdapter,
    rust::RustAnalyzer,
};

#[test]
fn rust_calls_preserve_callee_spelling_owner_and_dynamic_uncertainty() {
    let text = "fn run() { stage::open(); self.writer.flush(); build::<u8>(); (factory())(); /* fake() */ let s = \"also_fake()\"; fn nested() { inner(); } }";
    let analysis = RustAnalyzer::new()
        .unwrap()
        .analyze(text.as_bytes())
        .unwrap();
    let calls = analysis.calls.as_ref().unwrap();
    assert_eq!(calls.omitted, 0);
    let spelled = calls
        .sites
        .iter()
        .map(|c| c.spelling.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        spelled,
        vec![
            "stage::open",
            "self.writer.flush",
            "build::<u8>",
            "(factory())",
            "factory",
            "inner"
        ]
    );
    assert_eq!(calls.sites[0].name.as_deref(), Some("open"));
    assert_eq!(calls.sites[1].name.as_deref(), Some("flush"));
    assert_eq!(calls.sites[2].name.as_deref(), Some("build"));
    assert_eq!(calls.sites[3].name, None);
    for site in &calls.sites {
        assert_eq!(&text[site.span.clone()], site.spelling);
    }
    assert_eq!(analysis.facts[calls.sites[0].owner.unwrap()].name, "run");
    assert_eq!(
        analysis.facts[calls.sites.last().unwrap().owner.unwrap()].name,
        "nested"
    );
}
#[test]
fn python_calls_capture_actual_attributes_and_ignore_strings_and_comments() {
    let text =
        "def run():\n    pipe.emit()\n    ship()\n    # ignored()\n    label = 'also_ignored()'\n";
    let analysis = PythonAdapter
        .create()
        .unwrap()
        .analyze(text.as_bytes())
        .unwrap();
    let calls = analysis.calls.as_ref().unwrap();
    assert_eq!(
        calls
            .sites
            .iter()
            .map(|c| c.spelling.as_str())
            .collect::<Vec<_>>(),
        vec!["pipe.emit", "ship"]
    );
    assert_eq!(calls.sites[0].name.as_deref(), Some("emit"));
    assert_eq!(analysis.facts[calls.sites[0].owner.unwrap()].name, "run");
}
#[test]
fn document_capture_rejects_fabricated_call_text_ranges_and_ownership() {
    let text = b"fn run() { send(); } fn other() {}";
    let analysis = RustAnalyzer::new().unwrap().analyze(text).unwrap();
    let class = Classification::Assigned {
        language: "rust".into(),
        provenance: Provenance::Extension {
            extension: "rs".into(),
        },
    };
    assert!(
        Document::capture(
            WorkspaceId::derive(&[b"calls"]),
            "item.rs".into(),
            text,
            vec![analysis.clone()],
            class.clone()
        )
        .is_ok()
    );
    for mode in 0..3 {
        let mut bad = analysis.clone();
        let call = &mut bad.calls.as_mut().unwrap().sites[0];
        match mode {
            0 => call.spelling = "invented".into(),
            1 => call.span = 0..999,
            _ => call.owner = Some(1),
        }
        assert!(
            Document::capture(
                WorkspaceId::derive(&[b"calls"]),
                "item.rs".into(),
                text,
                vec![bad],
                class.clone()
            )
            .is_err()
        );
    }
}
#[test]
fn omitted_calls_are_explicit_and_legacy_analysis_does_not_claim_coverage() {
    let text = format!("fn run() {{ {} }}", "send();".repeat(4097));
    let analysis = RustAnalyzer::new()
        .unwrap()
        .analyze(text.as_bytes())
        .unwrap();
    assert_eq!(analysis.calls.as_ref().unwrap().sites.len(), 4096);
    assert_eq!(analysis.calls.as_ref().unwrap().omitted, 1);
    let mut old = serde_json::to_value(&analysis).unwrap();
    old.as_object_mut().unwrap().remove("calls");
    let old: vizzi_core::model::Analysis = serde_json::from_value(old).unwrap();
    assert!(old.calls.is_none());
}

#[test]
fn top_level_calls_and_oversized_callees_remain_explicit_and_utf8_failure_has_no_capability() {
    let text = "boot()\ndef run():\n    pass\n";
    let analysis = PythonAdapter
        .create()
        .unwrap()
        .analyze(text.as_bytes())
        .unwrap();
    assert_eq!(analysis.calls.unwrap().sites[0].owner, None);
    let long = format!("fn run() {{ {}(); }}", "x".repeat(257));
    let analysis = RustAnalyzer::new()
        .unwrap()
        .analyze(long.as_bytes())
        .unwrap();
    assert!(analysis.calls.as_ref().unwrap().sites.is_empty());
    assert_eq!(analysis.calls.unwrap().omitted, 1);
    let invalid = RustAnalyzer::new().unwrap().analyze(&[255]).unwrap();
    assert!(invalid.calls.is_none());
}

#[test]
fn forged_names_unsupported_coverage_and_invalid_owner_ordinals_are_rejected() {
    use vizzi_core::model::Coverage;
    let text = b"fn go() { send(); }";
    let analysis = RustAnalyzer::new().unwrap().analyze(text).unwrap();
    let class = Classification::Assigned {
        language: "rust".into(),
        provenance: Provenance::Extension {
            extension: "rs".into(),
        },
    };
    for mode in 0..5 {
        let mut bad = analysis.clone();
        let call = &mut bad.calls.as_mut().unwrap().sites[0];
        match mode {
            0 => call.name = Some("".into()),
            1 => call.name = Some("x".repeat(129)),
            2 => call.name = Some("foreign".into()),
            3 => call.owner = Some(usize::MAX),
            _ => bad.coverage = Coverage::Unavailable,
        };
        assert!(
            Document::capture(
                WorkspaceId::derive(&[b"bad"]),
                "item.rs".into(),
                text,
                vec![bad],
                class.clone()
            )
            .is_err()
        );
    }
    let exact = format!("fn go() {{ {}(); }}", "x".repeat(256));
    let found = RustAnalyzer::new()
        .unwrap()
        .analyze(exact.as_bytes())
        .unwrap();
    assert_eq!(found.calls.as_ref().unwrap().sites.len(), 1);
    assert!(found.calls.as_ref().unwrap().sites[0].name.is_none());
    assert_eq!(found.calls.unwrap().omitted, 0);
}

#[test]
fn other_language_adapters_do_not_claim_empty_call_coverage() {
    use vizzi_core::c_family::{CAdapter, CppAdapter};
    assert!(
        CAdapter
            .create()
            .unwrap()
            .analyze(b"void run(void) { call(); }")
            .unwrap()
            .calls
            .is_none()
    );
    assert!(
        CppAdapter
            .create()
            .unwrap()
            .analyze(b"void run() { call(); }")
            .unwrap()
            .calls
            .is_none()
    );
}

#[test]
fn call_capture_checks_empty_oversized_and_exact_literal_bounds_independently() {
    use vizzi_core::calls::CallSite;
    let class = Classification::Assigned {
        language: "rust".into(),
        provenance: Provenance::Extension {
            extension: "rs".into(),
        },
    };
    for size in [256, 257] {
        let text = "x".repeat(size);
        let mut analysis = RustAnalyzer::new()
            .unwrap()
            .analyze(text.as_bytes())
            .unwrap();
        analysis.calls.as_mut().unwrap().sites = vec![CallSite {
            name_span: None,
            target: None,
            owner: None,
            name: None,
            spelling: text.clone(),
            span: 0..size,
        }];
        let captured = Document::capture(
            WorkspaceId::derive(&[b"literal-bound"]),
            "item.rs".into(),
            text.as_bytes(),
            vec![analysis],
            class.clone(),
        );
        assert_eq!(captured.is_ok(), size == 256);
    }
    let mut empty = RustAnalyzer::new().unwrap().analyze(b"x").unwrap();
    empty.calls.as_mut().unwrap().sites = vec![CallSite {
        name_span: None,
        target: None,
        owner: None,
        name: None,
        spelling: "".into(),
        span: 0..0,
    }];
    assert!(
        Document::capture(
            WorkspaceId::derive(&[b"literal-bound"]),
            "item.rs".into(),
            b"x",
            vec![empty],
            class
        )
        .is_err()
    );
}
#[test]
fn call_capture_accepts_4096_sites_and_refuses_one_more() {
    use vizzi_core::calls::CallSite;
    let text = format!("fn run() {{ {} }}", "send();".repeat(4097));
    let mut analysis = RustAnalyzer::new()
        .unwrap()
        .analyze(text.as_bytes())
        .unwrap();
    let class = Classification::Assigned {
        language: "rust".into(),
        provenance: Provenance::Extension {
            extension: "rs".into(),
        },
    };
    assert!(
        Document::capture(
            WorkspaceId::derive(&[b"call-count"]),
            "item.rs".into(),
            text.as_bytes(),
            vec![analysis.clone()],
            class.clone()
        )
        .is_ok()
    );
    let last = text.rfind("send").unwrap();
    analysis.calls.as_mut().unwrap().sites.push(CallSite {
        name_span: None,
        target: None,
        owner: Some(0),
        name: Some("send".into()),
        spelling: "send".into(),
        span: last..last + 4,
    });
    assert!(
        Document::capture(
            WorkspaceId::derive(&[b"call-count"]),
            "item.rs".into(),
            text.as_bytes(),
            vec![analysis],
            class
        )
        .is_err()
    );
}

#[test]
fn name_limits_and_owner_boundaries_are_checked_without_correlated_invalid_fields() {
    let class = Classification::Assigned {
        language: "rust".into(),
        provenance: Provenance::Extension {
            extension: "rs".into(),
        },
    };
    for size in [128, 129] {
        let name = "x".repeat(size);
        let text = format!("fn run() {{ {name}(); }}");
        let mut analysis = RustAnalyzer::new()
            .unwrap()
            .analyze(text.as_bytes())
            .unwrap();
        analysis.calls.as_mut().unwrap().sites[0].name = Some(name);
        assert_eq!(
            Document::capture(
                WorkspaceId::derive(&[b"name-bound"]),
                "item.rs".into(),
                text.as_bytes(),
                vec![analysis],
                class.clone()
            )
            .is_ok(),
            size == 128
        );
    }
    let text = b"fn run() { send(); }";
    let base = RustAnalyzer::new().unwrap().analyze(text).unwrap();
    let span = base.calls.as_ref().unwrap().sites[0].span.clone();
    for mode in 0..3 {
        let mut analysis = base.clone();
        analysis.facts[0].name_span = None;
        analysis.facts[0].span = match mode {
            0 => span.clone(),
            1 => span.start + 1..span.end,
            _ => span.start..span.end - 1,
        };
        assert_eq!(
            Document::capture(
                WorkspaceId::derive(&[b"owner-bound"]),
                "item.rs".into(),
                text,
                vec![analysis],
                class.clone()
            )
            .is_ok(),
            mode == 0
        );
    }
}
#[test]
fn bundled_call_grammars_require_a_function_field_even_for_recovery_nodes() {
    for (types, kind) in [
        (tree_sitter_rust::NODE_TYPES, "call_expression"),
        (tree_sitter_python::NODE_TYPES, "call"),
    ] {
        let nodes: serde_json::Value = serde_json::from_str(types).unwrap();
        let node = nodes
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["type"] == kind && n["named"] == true)
            .unwrap();
        assert_eq!(node["fields"]["function"]["required"], true);
    }
}

#[test]
fn captured_identifier_spans_point_to_the_method_not_its_receiver_or_type_arguments() {
    let text =
        "struct A; impl A { fn run(&self) { self.writer.process_sample::<process_sample>(); } }";
    let analysis = RustAnalyzer::new()
        .unwrap()
        .analyze(text.as_bytes())
        .unwrap();
    let site = &analysis.calls.unwrap().sites[0];
    let value = serde_json::to_value(site).unwrap();
    let start = value["name_span"]["start"]
        .as_u64()
        .expect("captured identifier start") as usize;
    let end = value["name_span"]["end"].as_u64().unwrap() as usize;
    assert_eq!(&text[start..end], "process_sample");
    assert_eq!(start, text.find("process_sample").unwrap());
}
