//! Structural queries use captured declarations/calls, with explicit ambiguity and scope.
use std::sync::Arc;
use vizzi_core::{
    analysis::LanguageAdapter,
    model::{CapturePolicyId, Document, Snapshot, WorkspaceId},
    policy::{Classification, Provenance},
    python::PythonAdapter,
    rust::RustAnalyzer,
    structure::Structure,
};
fn snapshot(files: &[(&str, &str)]) -> Snapshot {
    let workspace = WorkspaceId::derive(&[b"structure"]);
    let docs = files
        .iter()
        .map(|(path, text)| {
            let analysis = if path.ends_with(".py") {
                PythonAdapter
                    .create()
                    .unwrap()
                    .analyze(text.as_bytes())
                    .unwrap()
            } else {
                RustAnalyzer::new()
                    .unwrap()
                    .analyze(text.as_bytes())
                    .unwrap()
            };
            let class = Classification::Assigned {
                language: analysis.context.language.clone(),
                provenance: Provenance::Extension {
                    extension: "fixture".into(),
                },
            };
            Arc::new(
                Document::capture(
                    workspace.clone(),
                    (*path).into(),
                    text.as_bytes(),
                    vec![analysis],
                    class,
                )
                .unwrap(),
            )
        })
        .collect();
    Snapshot::new(workspace, docs, CapturePolicyId::derive(&[b"fixture"])).unwrap()
}
#[test]
fn inventory_groups_names_and_exposes_candidate_uses_without_claiming_resolution() {
    let snap = snapshot(&[
        ("kit/panel.rs", "pub fn knob() {} pub fn switch() {}"),
        ("app/run.rs", "fn draw() { kit::knob(); }"),
        ("kit-old/panel.rs", "fn unrelated() {}"),
    ]);
    let index = Structure::new(&snap);
    let areas = index.areas("kit", 0).unwrap();
    assert!(areas.items.iter().any(|a| a.path == "kit"));
    let inventory = index.inventory("kit", "", 0).unwrap();
    assert_eq!(inventory.matching, 2);
    let knob = inventory
        .items
        .iter()
        .find(|e| e.symbol.name == "knob")
        .unwrap();
    assert_eq!(knob.candidate_uses, 1);
    assert_eq!(knob.same_name_definitions, 1);
    assert!(!inventory.items.iter().any(|e| e.symbol.name == "unrelated"));
    let uses = index.uses(&knob.symbol.id, "kit", 0).unwrap();
    assert_eq!(uses.matching, 1);
    assert_eq!(uses.items[0].spelling, "kit::knob");
    assert!(uses.items[0].outside_area);
    let doc = snap.file(&knob.symbol.source.file).unwrap();
    assert_eq!(doc.version().as_str(), knob.symbol.source.version);
}
#[test]
fn outgoing_calls_preserve_ambiguity_dynamic_sites_and_area_priority() {
    let snap = snapshot(&[
        (
            "pipe/main.rs",
            "fn start() { decode(); self.decode(); (factory())(); } fn decode() {}",
        ),
        ("pipe/other.rs", "fn decode() {}"),
        ("editor/panel.rs", "fn decode() {}"),
    ]);
    let index = Structure::new(&snap);
    let inv = index.inventory("pipe", "start", 0).unwrap();
    let start = &inv.items[0].symbol;
    let calls = index.outgoing(&start.id, "pipe", 0).unwrap();
    assert_eq!(calls.matching, 4);
    let first = &calls.items[0];
    assert_eq!(first.candidate_count, 3);
    assert_eq!(first.candidates.len(), 3);
    assert!(first.candidates[0].source.path.starts_with("pipe/"));
    assert_eq!(first.candidates[0].source.path, "pipe/main.rs");
    assert!(first.candidates[2].source.path.starts_with("editor/"));
    assert!(
        calls
            .items
            .iter()
            .any(|c| c.name.is_none() && c.candidates.is_empty())
    );
    assert!(index.outgoing(&start.id, "", 0).is_err());
    assert!(index.outgoing("missing", "pipe", 0).is_err());
}
#[test]
fn python_and_recursive_calls_are_finite_and_version_bound() {
    let snap = snapshot(&[(
        "river/run.py",
        "def run():\n    turn()\ndef turn():\n    run()\n",
    )]);
    let index = Structure::new(&snap);
    let inv = index.inventory("river", "run", 0).unwrap();
    let start = &inv.items[0].symbol;
    let calls = index.outgoing(&start.id, "river", 0).unwrap();
    assert_eq!(calls.items[0].name.as_deref(), Some("turn"));
    let back = index
        .outgoing(&calls.items[0].candidates[0].id, "river", 0)
        .unwrap();
    assert_eq!(back.items[0].candidates[0].id, start.id);
    let changed = Structure::new(&snapshot(&[("river/run.py", "def other():\n    pass\n")]));
    assert!(changed.outgoing(&start.id, "river", 0).is_err());
}
#[test]
fn invalid_scopes_and_input_limits_are_rejected_and_pages_are_explicit() {
    let source = (0..30)
        .map(|n| format!("fn item_{n}() {{}}\n"))
        .collect::<String>();
    let index = Structure::new(&snapshot(&[("tree/item.rs", &source)]));
    let page = index.inventory("tree", "", 0).unwrap();
    assert_eq!(page.items.len(), 24);
    assert_eq!(page.matching, 30);
    assert_eq!(page.next, Some(24));
    assert_eq!(index.inventory("tree", "", 24).unwrap().items.len(), 6);
    for path in ["../tree", "/tree", "tree/", "tree\\item"] {
        assert!(index.inventory(path, "", 0).is_err());
    }
    assert!(index.inventory("tree", &"x".repeat(257), 0).is_err());
    assert!(index.inventory("missing", "", 0).unwrap().items.is_empty());
}

#[test]
fn source_area_finding_can_start_from_a_captured_name_without_knowing_its_path() {
    let index = Structure::new(&snapshot(&[
        ("river/dispatch.rs", "fn decode_packet() {}"),
        ("other/noise.rs", "fn ignored() {}"),
    ]));
    let areas = index.areas("decode_packet", 0).unwrap();
    assert_eq!(areas.matching, 1);
    assert_eq!(areas.items[0].path, "river/dispatch.rs");
}

#[test]
fn call_and_target_pages_keep_total_ambiguity_and_do_not_expand_every_match() {
    let body = format!("fn start() {{ {} }}", "hit();".repeat(9));
    let mut files = vec![("scope/start.rs".to_owned(), body)];
    for n in 0..10 {
        files.push((format!("scope/c{n}.rs"), "fn hit() {}".into()));
    }
    let refs = files
        .iter()
        .map(|(p, t)| (p.as_str(), t.as_str()))
        .collect::<Vec<_>>();
    let index = Structure::new(&snapshot(&refs));
    let start = &index.inventory("scope", "start", 0).unwrap().items[0].symbol;
    let calls = index.outgoing(&start.id, "scope", 0).unwrap();
    assert_eq!(calls.matching, 9);
    assert_eq!(calls.items.len(), 8);
    assert_eq!(calls.next, Some(8));
    assert_eq!(calls.items[0].candidate_count, 10);
    assert_eq!(calls.items[0].candidates.len(), 8);
    assert_eq!(
        index.outgoing(&start.id, "scope", 8).unwrap().items.len(),
        1
    );
    let target = &calls.items[0].candidates[0];
    let uses = index.uses(&target.id, "scope", 0).unwrap();
    assert_eq!(uses.matching, 9);
    assert_eq!(uses.items.len(), 8);
    assert!(
        index
            .inventory("scope", "", usize::MAX)
            .unwrap()
            .next
            .is_none()
    );
}
#[test]
fn unsupported_and_partial_call_coverage_do_not_look_like_proven_zero_uses() {
    use vizzi_core::policy::SourceOnlyReason;
    let base = snapshot(&[("scope/broken.rs", "fn broken() { called();")]);
    let workspace = base.workspace().clone();
    let mut docs = base.documents().to_vec();
    docs.push(Arc::new(
        Document::capture(
            workspace.clone(),
            "scope/plain.txt".into(),
            b"not analyzed",
            vec![],
            Classification::SourceOnly {
                language: None,
                provenance: Provenance::Extension {
                    extension: "txt".into(),
                },
                reason: SourceOnlyReason::AnalysisNotRequested,
            },
        )
        .unwrap(),
    ));
    let snap = Snapshot::new(workspace, docs, CapturePolicyId::derive(&[b"fixture"])).unwrap();
    let index = Structure::new(&snap);
    let page = index.inventory("scope", "", 0).unwrap();
    assert_eq!(page.coverage.unavailable_call_files, 1);
    assert_eq!(page.coverage.partial_files, 1);
    assert_eq!(page.coverage.available_files, 2);
}
#[test]
fn metadata_operations_do_not_load_private_backing() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use vizzi_core::model::{CapturedSource, SourceReadError, SourceReader};
    #[derive(Debug)]
    struct Backing(AtomicBool);
    const TEXT: &[u8] = b"fn start() { finish(); } fn finish() {}";
    impl SourceReader for Backing {
        fn read_at(&self, offset: usize, target: &mut [u8]) -> Result<(), SourceReadError> {
            if self.0.load(Ordering::SeqCst) {
                return Err(SourceReadError::Io("fixture unavailable".into()));
            }
            target.copy_from_slice(&TEXT[offset..offset + target.len()]);
            Ok(())
        }
    }
    let backing = Arc::new(Backing(AtomicBool::new(false)));
    let source = CapturedSource::capture_reader(TEXT.len(), backing.clone(), &|| false)
        .unwrap()
        .unwrap();
    let analysis = RustAnalyzer::new().unwrap().analyze(TEXT).unwrap();
    let input = source.read_all(TEXT.len()).unwrap();
    let workspace = WorkspaceId::derive(&[b"backed"]);
    let doc = Arc::new(
        Document::capture_loaded(
            workspace.clone(),
            "scope/backed.rs".into(),
            source.clone(),
            &input,
            vec![analysis],
            Classification::Assigned {
                language: "rust".into(),
                provenance: Provenance::Extension {
                    extension: "rs".into(),
                },
            },
        )
        .unwrap(),
    );
    drop(input);
    backing.0.store(true, Ordering::SeqCst);
    let snap = Snapshot::new(workspace, vec![doc], CapturePolicyId::derive(&[b"fixture"])).unwrap();
    let index = Structure::new(&snap);
    let inv = index.inventory("scope", "start", 0).unwrap();
    assert_eq!(
        index
            .outgoing(&inv.items[0].symbol.id, "scope", 0)
            .unwrap()
            .items[0]
            .spelling,
        "finish"
    );
}
#[test]
fn projection_counts_omitted_documents_and_accepts_its_exact_file_allowance() {
    use vizzi_core::policy::SourceOnlyReason;
    let workspace = WorkspaceId::derive(&[b"file-cap"]);
    let docs = (0..4097)
        .map(|n| {
            Arc::new(
                Document::capture(
                    workspace.clone(),
                    format!("f{n}.rs"),
                    b"",
                    vec![],
                    Classification::SourceOnly {
                        language: Some("rust".into()),
                        provenance: Provenance::Extension {
                            extension: "rs".into(),
                        },
                        reason: SourceOnlyReason::AnalysisNotRequested,
                    },
                )
                .unwrap(),
            )
        })
        .collect();
    let snap = Snapshot::new(workspace, docs, CapturePolicyId::derive(&[b"fixture"])).unwrap();
    let page = Structure::new(&snap).areas("", 0).unwrap();
    assert_eq!(page.coverage.indexed_files, 4096);
    assert_eq!(page.coverage.omitted_files, 1);
    assert_eq!(page.coverage.unavailable_call_files, 4096);
}

#[test]
fn declaration_and_call_allowances_are_finite_and_expose_omissions() {
    use vizzi_core::model::Fact;
    let workspace = WorkspaceId::derive(&[b"item-caps"]);
    let mut analysis = RustAnalyzer::new().unwrap().analyze(b"x").unwrap();
    analysis.coloring = None;
    analysis.calls = None;
    analysis.facts = (0..65537)
        .map(|n| Fact {
            name_span: None,
            parent: None,
            key: format!("k{n}"),
            name: format!("item_{n}"),
            kind: "variable".into(),
            span: 0..1,
            unique: true,
        })
        .collect();
    let doc = Arc::new(
        Document::capture(
            workspace.clone(),
            "items/data.rs".into(),
            b"x",
            vec![analysis],
            Classification::Assigned {
                language: "rust".into(),
                provenance: Provenance::Extension {
                    extension: "rs".into(),
                },
            },
        )
        .unwrap(),
    );
    let snap = Snapshot::new(workspace, vec![doc], CapturePolicyId::derive(&[b"fixture"])).unwrap();
    let page = Structure::new(&snap).inventory("items", "", 0).unwrap();
    assert_eq!(page.matching, 65536);
    assert_eq!(page.coverage.omitted_symbols, 1);
    let body = format!("fn run() {{ {} }}", "hit();".repeat(4096));
    let files = (0..17)
        .map(|n| (format!("units/{n}.rs"), body.clone()))
        .collect::<Vec<_>>();
    let refs = files
        .iter()
        .map(|(p, t)| (p.as_str(), t.as_str()))
        .collect::<Vec<_>>();
    let index = Structure::new(&snapshot(&refs));
    let page = index.inventory("units", "run", 0).unwrap();
    assert_eq!(page.coverage.omitted_calls, 4096);
    assert_eq!(page.items[0].candidate_uses, 0);
}

#[test]
fn repository_and_exact_file_scopes_include_their_declarations() {
    let index = Structure::new(&snapshot(&[
        ("unit/item.rs", "fn alpha() {} fn beta() {}"),
        ("unit2/item.rs", "fn gamma() {}"),
    ]));
    assert_eq!(index.inventory("", "", 0).unwrap().matching, 3);
    assert_eq!(index.inventory("unit/item.rs", "", 0).unwrap().matching, 2);
    assert!(index.inventory(&"x".repeat(4096), "", 0).is_ok());
    assert!(index.inventory(&"x".repeat(4097), "", 0).is_err());
    assert!(index.inventory("unit", &"x".repeat(256), 0).is_ok());
    assert!(index.areas(&"x".repeat(257), 0).is_err());
}
#[test]
fn qualified_parent_labels_preserve_order_and_valid_bounded_utf8() {
    let name = "界".repeat(60);
    let code = format!("mod outer {{ mod {name} {{ fn entry() {{}} }} }}");
    let index = Structure::new(&snapshot(&[("unit/deep.rs", &code)]));
    let page = index.inventory("unit", "entry", 0).unwrap();
    assert_eq!(
        page.items[0].symbol.parents,
        vec!["outer".to_owned(), "界".repeat(42)]
    );
    assert_eq!(page.coverage.omitted_symbols, 1);
}

#[test]
fn exhausted_pages_never_offer_an_empty_continuation() {
    let index = Structure::new(&snapshot(&[("one/file.rs", "fn item() {}")]));
    assert_eq!(index.inventory("one", "", 0).unwrap().next, None);
    assert_eq!(index.inventory("missing", "", 0).unwrap().next, None);
    let data = (0..24)
        .map(|n| format!("fn item_{n}() {{}}\n"))
        .collect::<String>();
    let index = Structure::new(&snapshot(&[("one/file.rs", &data)]));
    assert_eq!(index.inventory("one", "", 0).unwrap().next, None);
}

#[test]
fn native_call_identity_and_exact_definition_positions_do_not_depend_on_name_ranking() {
    let text = "fn start() { module::renamed(); } mod module { fn original() {} }";
    let index = Structure::new(&snapshot(&[("src/lib.rs", text)]));
    let start = &index.inventory("src", "start", 0).unwrap().items[0].symbol;
    let invocation = &index.outgoing(&start.id, "src", 0).unwrap().items[0];
    assert_eq!(invocation.id.len(), 64);
    let lookup = index.call(&invocation.id).unwrap();
    assert_eq!(&text[lookup.identifier.unwrap()], "renamed");
    assert_eq!(lookup.language, "rust");
    let definition = index
        .definition("src/lib.rs", text.find("original").unwrap())
        .unwrap();
    assert_eq!(definition.name, "original");
    assert!(index.definition("missing.rs", 0).is_none());
    assert!(index.call("not-a-call").is_none());
}

#[test]
fn a_local_variable_definition_is_not_misreported_as_its_enclosing_function() {
    let text = "fn owner() { let callback = || {}; callback(); }";
    let index = Structure::new(&snapshot(&[("src/lib.rs", text)]));
    assert!(
        index
            .definition("src/lib.rs", text.find("callback").unwrap())
            .is_none()
    );
    assert_eq!(
        index
            .definition("src/lib.rs", text.find("owner").unwrap())
            .unwrap()
            .name,
        "owner"
    );
}

#[test]
fn declaration_search_matches_qualified_names_and_partial_namespace_suffixes() {
    let index = Structure::new(&snapshot(&[(
        "src/audio.rs",
        "mod audio { struct LufsMeter; impl LufsMeter { fn process_sample(&mut self) {} } }",
    )]));
    for query in [
        "LufsMeter::process_sample",
        "audio::LufsMeter::process_sample",
        "lufsmeter.process_sample",
        " LUFSMETER::process_sam ",
    ] {
        let results = index.inventory("", query, 0).unwrap();
        assert_eq!(results.matching, 1, "{query}");
        assert_eq!(results.items[0].symbol.name, "process_sample");
    }
}

#[test]
fn exact_names_rank_before_prefixes_and_substrings_across_source_paths() {
    let index = Structure::new(&snapshot(&[
        (
            "a.rs",
            "fn process_sample() {} fn preprocess() {} fn process_block() {}",
        ),
        ("z.rs", "fn process() {}"),
    ]));
    let results = index.inventory("", "process", 0).unwrap();
    assert_eq!(
        results
            .items
            .iter()
            .map(|e| e.symbol.name.as_str())
            .collect::<Vec<_>>(),
        ["process", "process_block", "process_sample", "preprocess"]
    );
}

#[test]
fn ranking_happens_before_paging_so_later_exact_matches_reach_the_first_page() {
    let prefixes = (0..30)
        .map(|i| format!("fn process_{i:02}() {{}}\n"))
        .collect::<String>();
    let index = Structure::new(&snapshot(&[
        ("a.rs", &prefixes),
        ("z.rs", "fn process() {}"),
    ]));
    let first = index.inventory("", "process", 0).unwrap();
    assert_eq!(first.matching, 31);
    assert_eq!(first.items[0].symbol.name, "process");
    assert_eq!(first.next, Some(24));
    let second = index.inventory("", "process", 24).unwrap();
    assert_eq!(second.items.len(), 7);
    assert_eq!(second.next, None);
    assert_eq!(second.items[0].symbol.name, "process_23");
}

#[test]
fn function_search_filters_kinds_before_counting_and_paging() {
    let types = (0..30)
        .map(|i| format!("struct process_{i:02};\n"))
        .collect::<String>();
    let text = format!("{types} fn process() {{}} fn process_sample() {{}}");
    let index = Structure::new(&snapshot(&[("src.rs", &text)]));
    let result = index.functions("", "process", 0).unwrap();
    assert_eq!(result.matching, 2);
    assert_eq!(
        result
            .items
            .iter()
            .map(|e| e.symbol.name.as_str())
            .collect::<Vec<_>>(),
        ["process", "process_sample"]
    );
    assert_eq!(result.next, None);
    assert_eq!(index.inventory("", "process", 0).unwrap().matching, 32);
}

#[test]
fn equal_name_matches_have_consistent_qualified_path_and_source_order() {
    let index = Structure::new(&snapshot(&[
        ("z.rs", "mod a { fn process() {} }"),
        ("b.rs", "mod b { fn process() {} }"),
        ("a.rs", "mod a { fn process() {} }"),
    ]));
    let result = index.functions("", "process", 0).unwrap();
    assert_eq!(
        result
            .items
            .iter()
            .map(|e| e.symbol.source.path.as_str())
            .collect::<Vec<_>>(),
        ["a.rs", "z.rs", "b.rs"]
    );
    let scoped = index.functions("b.rs", "process", 0).unwrap();
    assert_eq!(scoped.matching, 1);
    assert_eq!(scoped.items[0].symbol.source.path, "b.rs");
}

#[test]
fn dotted_python_names_and_trait_impl_labels_are_searchable() {
    let index = Structure::new(&snapshot(&[
        (
            "meter.py",
            "class Meter:\n    def process_sample(self):\n        pass\n",
        ),
        (
            "plugin.rs",
            "struct GsMeter; impl Plugin for GsMeter { fn process(&mut self) {} }",
        ),
    ]));
    assert_eq!(
        index
            .functions("", "Meter.process_sample", 0)
            .unwrap()
            .items[0]
            .symbol
            .name,
        "process_sample"
    );
    assert_eq!(
        index.functions("", "GsMeter::process", 0).unwrap().items[0]
            .symbol
            .name,
        "process"
    );
    assert_eq!(
        index.areas("Meter.process_sample", 0).unwrap().items[0].path,
        "meter.py"
    );
}

#[test]
fn complete_qualified_names_outrank_suffixes_prefixes_and_scope_substrings() {
    let source = "struct Meter; impl Meter { fn run() {} fn runner() {} } mod outer { struct Meter; impl Meter { fn run() {} } } struct NotMeter; impl NotMeter { fn run() {} } mod prefix { mod Meter { mod run_module { fn aardvark() {} } } }";
    let index = Structure::new(&snapshot(&[("src.rs", source)]));
    let result = index.functions("", "Meter::run", 0).unwrap();
    assert_eq!(
        result
            .items
            .iter()
            .map(|e| e.symbol.parents.join("::") + "::" + &e.symbol.name)
            .collect::<Vec<_>>(),
        [
            "Meter::run",
            "outer::Meter::run",
            "prefix::Meter::run_module::aardvark",
            "Meter::runner",
            "NotMeter::run"
        ]
    );
}

#[test]
fn path_only_matches_follow_names_and_function_browsing_is_alphabetical() {
    let index = Structure::new(&snapshot(&[
        ("process.rs", "fn aardvark() {}"),
        ("z.rs", "fn process() {} fn process_sample() {}"),
    ]));
    assert_eq!(
        index
            .functions("", "process", 0)
            .unwrap()
            .items
            .iter()
            .map(|e| e.symbol.name.as_str())
            .collect::<Vec<_>>(),
        ["process", "process_sample", "aardvark"]
    );
    assert_eq!(
        index
            .functions("", "", 0)
            .unwrap()
            .items
            .iter()
            .map(|e| e.symbol.name.as_str())
            .collect::<Vec<_>>(),
        ["aardvark", "process", "process_sample"]
    );
    let result = index.functions("", "PROCESS.RS", 0).unwrap();
    assert_eq!(result.matching, 1);
    assert_eq!(result.items[0].symbol.name, "aardvark");
}

#[test]
fn same_qualified_names_in_one_file_use_source_lines_to_break_ties() {
    let index = Structure::new(&snapshot(&[(
        "src.rs",
        "fn process() {}\nfn process() {}\n",
    )]));
    assert_eq!(
        index
            .functions("", "process", 0)
            .unwrap()
            .items
            .iter()
            .map(|e| e.symbol.source.line)
            .collect::<Vec<_>>(),
        [1, 2]
    );
}

#[test]
fn unicode_namespaces_are_casefolded() {
    let index = Structure::new(&snapshot(&[("src.rs", "mod Δ { fn Über() {} }")]));
    let result = index.functions("", "δ::über", 0).unwrap();
    assert_eq!(result.matching, 1);
    assert_eq!(result.items[0].symbol.name, "Über");
}

#[test]
fn unfiltered_empty_inventories_keep_lexical_order() {
    let index = Structure::new(&snapshot(&[("src.rs", "fn zeta() {} fn alpha() {}")]));
    assert_eq!(
        index
            .inventory("", "", 0)
            .unwrap()
            .items
            .iter()
            .map(|e| e.symbol.name.as_str())
            .collect::<Vec<_>>(),
        ["zeta", "alpha"]
    );
    assert_eq!(
        index
            .functions("", "", 0)
            .unwrap()
            .items
            .iter()
            .map(|e| e.symbol.name.as_str())
            .collect::<Vec<_>>(),
        ["alpha", "zeta"]
    );
}

#[test]
fn identical_search_keys_preserve_source_order_even_on_one_line() {
    let source = "fn process() {} ".repeat(8);
    let index = Structure::new(&snapshot(&[("src.rs", &source)]));
    let source_order = index
        .inventory("src.rs", "", 0)
        .unwrap()
        .items
        .into_iter()
        .map(|e| e.symbol.id)
        .collect::<Vec<_>>();
    let search_order = index
        .functions("src.rs", "process", 0)
        .unwrap()
        .items
        .into_iter()
        .map(|e| e.symbol.id)
        .collect::<Vec<_>>();
    assert_eq!(search_order, source_order);
}

#[test]
fn function_name_matches_outrank_matches_only_in_the_enclosing_type() {
    let index = Structure::new(&snapshot(&[(
        "src.rs",
        "struct Processor; impl Processor { fn begin_block() {} fn process_sample() {} } fn preprocess() {} fn process() {}",
    )]));
    assert_eq!(
        index
            .functions("", "process", 0)
            .unwrap()
            .items
            .iter()
            .map(|e| e.symbol.name.as_str())
            .collect::<Vec<_>>(),
        ["process", "process_sample", "preprocess", "begin_block"]
    );
}
