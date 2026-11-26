//! Behavioural checks for captured module/identity context.
use std::sync::Arc;
use vizzi_core::{
    analysis::{LanguageAdapter, Languages},
    model::{Analysis, CapturePolicyId, ContentId, Coverage, Document, Snapshot, WorkspaceId},
    policy::{Classification, PolicyConfig, Provenance, SourceOnlyReason, SourcePolicy},
    python::PythonAdapter,
    ruby::RubyAdapter,
    rust::RustAdapter,
    structure::Structure,
};
fn snapshot(files: &[(&str, &str)]) -> Snapshot {
    let workspace = WorkspaceId::derive(&[b"module-fixture"]);
    let mut rust = RustAdapter.create().unwrap();
    let mut python = PythonAdapter.create().unwrap();
    let mut ruby = RubyAdapter.create().unwrap();
    let mut java = Languages::builtin()
        .unwrap()
        .select("java")
        .unwrap()
        .create()
        .unwrap();
    let documents = files
        .iter()
        .map(|(path, text)| {
            let (analysis, classification) = if path.ends_with(".rs") {
                (
                    vec![rust.analyze(text.as_bytes()).unwrap()],
                    Classification::Assigned {
                        language: "rust".into(),
                        provenance: Provenance::Extension {
                            extension: "rs".into(),
                        },
                    },
                )
            } else if path.ends_with(".py") {
                (
                    vec![python.analyze(text.as_bytes()).unwrap()],
                    Classification::Assigned {
                        language: "python".into(),
                        provenance: Provenance::Extension {
                            extension: "py".into(),
                        },
                    },
                )
            } else if path.ends_with(".rb") {
                (
                    vec![ruby.analyze(text.as_bytes()).unwrap()],
                    Classification::Assigned {
                        language: "ruby".into(),
                        provenance: Provenance::Extension {
                            extension: "rb".into(),
                        },
                    },
                )
            } else if path.ends_with(".java") {
                (
                    vec![java.analyze(text.as_bytes()).unwrap()],
                    Classification::Assigned {
                        language: "java".into(),
                        provenance: Provenance::Extension {
                            extension: "java".into(),
                        },
                    },
                )
            } else {
                (
                    vec![],
                    Classification::SourceOnly {
                        language: None,
                        provenance: Provenance::Extension {
                            extension: "toml".into(),
                        },
                        reason: SourceOnlyReason::ProjectMetadata,
                    },
                )
            };
            Arc::new(
                Document::capture(
                    workspace.clone(),
                    (*path).into(),
                    text.as_bytes(),
                    analysis,
                    classification,
                )
                .unwrap(),
            )
        })
        .collect();
    Snapshot::new(workspace, documents, CapturePolicyId::derive(&[b"fixture"])).unwrap()
}
fn module(index: &Structure, name: &str) -> vizzi_core::modules::Module {
    index
        .functions("", name, 0)
        .unwrap()
        .items
        .into_iter()
        .find(|e| e.symbol.name == name)
        .unwrap()
        .symbol
        .module
        .expect(name)
}

#[test]
fn native_module_catalog_describes_emitted_language_and_provenance_pairs() {
    let source = snapshot(&[
        (
            "Cargo.toml",
            "[package]\nname = 'demo'\n[lib]\npath = 'lib.rs'\n",
        ),
        ("lib.rs", "fn from_rust() {}"),
        ("worker.py", "def from_python(): pass\n"),
        ("worker.rb", "module Audio; def from_ruby; end; end\n"),
        (
            "Worker.java",
            "package audio; class Worker { void fromJava() {} }",
        ),
    ]);
    let index = Structure::new(&source);
    let kinds = vizzi_core::modules::module_kinds()
        .map(|kind| serde_json::to_value(kind).unwrap())
        .collect::<Vec<_>>();
    for (name, tooltip) in [
        ("from_rust", "Rust module"),
        ("from_python", "Python module"),
        ("from_ruby", "Ruby module"),
        ("fromJava", "Java package"),
    ] {
        let member = module(&index, name);
        let kind = kinds
            .iter()
            .find(|kind| kind["language"] == member.language && kind["basis"] == member.basis)
            .unwrap();
        assert_eq!(kind["tooltip"], tooltip);
    }
}

#[test]
fn java_packages_use_declared_names_and_keep_separate_directories_distinct() {
    let snapshot = snapshot(&[
        (
            "First.java",
            "/* package wrong; */ @Deprecated package café /* comment */ . audio; class First { void first() {} }",
        ),
        (
            "Second.java",
            "package café.audio; class Second { void second() {} }",
        ),
        (
            "other/Third.java",
            "package café.audio; class Third { void third() {} }",
        ),
        (
            "other/Other.java",
            "package different.audio; class Other { void different() {} }",
        ),
        (
            "Neutral.java",
            "class Neutral { void neutral() { String text = \"package pretend;\"; } }",
        ),
        (
            "Duplicate.java",
            "package first; package second; class Duplicate { void duplicate() {} }",
        ),
        (
            "Broken.java",
            "package bad.; class Broken { void broken() {} }",
        ),
    ]);
    let index = Structure::new(&snapshot);
    let first = module(&index, "first");
    assert_eq!(first.label, "café.audio");
    assert_eq!(first.root, ".");
    assert_eq!(first, module(&index, "second"));
    let third = module(&index, "third");
    let different = module(&index, "different");
    assert_eq!(first.label, third.label);
    assert_ne!(first.id, third.id);
    assert_eq!(third.root, "other");
    assert_ne!(third.id, different.id);
    for name in ["neutral", "duplicate", "broken"] {
        let page = index.functions("", name, 0).unwrap();
        assert!(
            page.items
                .iter()
                .find(|entry| entry.symbol.name == name)
                .unwrap()
                .symbol
                .module
                .is_none(),
            "{name}"
        );
    }
}

#[test]
fn java_named_modules_use_the_matching_source_root_and_refresh_in_place() {
    let files = |descriptor| {
        snapshot(&[
            ("one/module-info.java", "module unrelated {}"),
            ("one/src/module-info.java", descriptor),
            (
                "one/src/demo/engine/App.java",
                "package demo.engine; class App { void start() {} }",
            ),
            (
                "one/src/demo/engine/Peer.java",
                "package demo.engine; class Peer { void peer() {} }",
            ),
            (
                "one/src/demo/util/Utility.java",
                "package demo.util; class Utility { void utility() {} }",
            ),
            (
                "one/src/Different.java",
                "package demo.engine; class Different { void misplaced() {} }",
            ),
            (
                "two/src/module-info.java",
                "open module com.example { exports demo.engine; }",
            ),
            (
                "two/src/demo/engine/Other.java",
                "package demo.engine; class Other { void other() {} }",
            ),
        ])
    };
    let mut cache = vizzi_core::modules::ModuleCache::default();
    let initial = Structure::with_module_cache(
        &files(
            "import java.lang.Deprecated; @Deprecated open module com.example { exports demo.engine; }",
        ),
        &mut cache,
    );
    let original = module(&initial, "start");
    assert_eq!(original.label, "com.example/demo.engine");
    assert_eq!(original.root, "one/src/module-info.java");
    assert_eq!(original, module(&initial, "peer"));
    assert_ne!(original.id, module(&initial, "other").id);
    assert_eq!(module(&initial, "utility").label, "com.example/demo.util");
    assert_eq!(module(&initial, "misplaced").label, "demo.engine");

    let renamed = files("module com.renamed {}");
    let updated = module(&Structure::with_module_cache(&renamed, &mut cache), "start");
    assert_eq!(updated, module(&Structure::new(&renamed), "start"));
    assert_eq!(updated.id, original.id);
    assert_eq!(updated.label, "com.renamed/demo.engine");

    for descriptor in [
        "module com.broken",
        "module com.first {} module com.second {}",
    ] {
        let invalid = Structure::with_module_cache(&files(descriptor), &mut cache);
        assert_eq!(module(&invalid, "start").label, "demo.engine");
    }
}

#[test]
fn java_cues_keep_descriptor_identity_when_deferred_files_fill_the_source_budget() {
    let baseline = snapshot(&[
        (
            "src/demo/Caller.java",
            "package demo; class Caller { void target() {} }",
        ),
        (
            "src/module-info.java",
            "module example.app { exports demo; }",
        ),
    ]);
    let mut cache = vizzi_core::modules::ModuleCache::default();
    let original = module(
        &Structure::with_module_cache(&baseline, &mut cache),
        "target",
    );
    assert_eq!(original.label, "example.app/demo");

    let caller = baseline.file_by_path("src/demo/Caller.java").unwrap();
    let mut documents = baseline.documents().to_vec();
    let content = " ".repeat(1024 * 1024);
    let deferred = Analysis {
        calls: None,
        coloring: None,
        has_entry_point: false,
        content: ContentId::derive(&[content.as_bytes()]),
        context: caller.analyses()[0].context.clone(),
        coverage: Coverage::Deferred,
        facts: vec![],
        diagnostics: vec![],
    };
    for n in 0..8 {
        let end = content.len() - if n == 7 { caller.bytes() } else { 0 };
        let bytes = &content.as_bytes()[..end];
        documents.push(Arc::new(
            Document::capture(
                baseline.workspace().clone(),
                format!("src/generated-{n}.java"),
                bytes,
                vec![Analysis {
                    content: ContentId::derive(&[bytes]),
                    ..deferred.clone()
                }],
                caller.classification().clone(),
            )
            .unwrap(),
        ));
    }
    let updated = Snapshot::new(
        baseline.workspace().clone(),
        documents,
        baseline.policy().clone(),
    )
    .unwrap();
    assert_eq!(
        module(
            &Structure::with_module_cache(&updated, &mut cache),
            "target"
        ),
        original
    );
    assert_eq!(module(&Structure::new(&updated), "target"), original);
}

#[test]
fn java_module_descriptors_reject_stray_top_level_tokens() {
    for descriptor in [
        "module example.app {} @",
        "module example.app {};",
        ";module example.app {}",
    ] {
        let snapshot = snapshot(&[
            ("src/module-info.java", descriptor),
            (
                "src/demo/Caller.java",
                "package demo; class Caller { void target() {} }",
            ),
        ]);
        assert_eq!(
            module(&Structure::new(&snapshot), "target").label,
            "demo",
            "{descriptor}"
        );
    }
}

#[test]
fn java_module_descriptors_apply_unicode_escapes_before_line_terminators() {
    for (descriptor, label) in [
        (r"module example.\u0061pp {}", "example.app/demo"),
        (
            "// leading comment\rmodule example.app {}",
            "example.app/demo",
        ),
        (
            r"// \u000d\u000amodule example.\uu0061pp {}",
            "example.app/demo",
        ),
        (r"module example.\uD801\uDC00 {}", "example.𐐀/demo"),
        (r"// \u005c\u000dmodule example.app {}", "example.app/demo"),
        (r"// \u005cu000dmodule unexpected {}", "demo"),
        (r"// \\u000dmodule unexpected {}", "demo"),
        ("// \\u00XX\nmodule example.app {}", "demo"),
    ] {
        let snapshot = snapshot(&[
            ("src/module-info.java", descriptor),
            (
                "src/demo/Caller.java",
                "package demo; class Caller { void target() {} }",
            ),
        ]);
        assert_eq!(
            module(&Structure::new(&snapshot), "target").label,
            label,
            "{descriptor}"
        );
    }
}

#[test]
fn manifest_contents_in_a_source_file_do_not_create_rust_module_membership() {
    let source = snapshot(&[
        (
            "metadata.py",
            "[package]\nname = 'unrelated'\n[lib]\npath = 'lib.rs'\n",
        ),
        ("lib.rs", "fn target() {}"),
    ]);
    let function = Structure::new(&source)
        .functions("lib.rs", "target", 0)
        .unwrap()
        .items
        .remove(0);
    assert!(function.symbol.module.is_none());
}
#[test]
fn ruby_reopened_modules_share_identity_without_guessing_from_filenames_or_classes() {
    let source = snapshot(&[
        (
            "one.rb",
            "module Audio; class Meter; def process; end; end; end",
        ),
        ("other.rb", "module Audio; def self.render; end; end"),
        (
            "nested.rb",
            "module Audio; module FX; def filter; end; end; end",
        ),
        ("plain.rb", "class Plain; def plain; end; end"),
        (
            "class_module.rb",
            "class Rack; module Nested; def inside_class; end; end; end",
        ),
    ]);
    let index = Structure::new(&source);
    let a = module(&index, "process");
    assert_eq!(
        index
            .functions("", "Audio::Meter#process", 0)
            .unwrap()
            .items
            .len(),
        1
    );
    let b = module(&index, "render");
    let nested = module(&index, "filter");
    assert_eq!(a.id, b.id);
    assert_eq!(a.label, "Audio");
    assert_eq!(a.root, "Audio");
    assert_eq!(a.language, "ruby");
    assert_eq!(nested.label, "Audio::FX");
    assert_eq!(module(&index, "inside_class").label, "Rack::Nested");
    assert_ne!(a.id, nested.id);
    assert!(
        index.functions("plain.rb", "plain", 0).unwrap().items[0]
            .symbol
            .module
            .is_none()
    );
}
#[test]
fn rust_types_share_their_actual_module_and_inline_scopes_remain_distinct() {
    let s = snapshot(&[
        ("Cargo.toml", "[package]\nname='audio-pack'\n"),
        (
            "src/lib.rs",
            "pub mod engine; struct Meter; impl Meter { fn process(){} } struct Other; impl Other { fn render(){} } mod nested { pub fn inside(){} }",
        ),
        (
            "src/engine.rs",
            "struct Processor; impl Processor { fn begin_block(){} fn sample(){} }",
        ),
    ]);
    let index = Structure::new(&s);
    assert_eq!(module(&index, "process"), module(&index, "render"));
    assert_eq!(module(&index, "process").label, "audio_pack");
    assert_eq!(module(&index, "begin_block"), module(&index, "sample"));
    assert_eq!(module(&index, "sample").label, "audio_pack::engine");
    assert_eq!(module(&index, "inside").label, "audio_pack::nested");
    assert_ne!(module(&index, "inside").id, module(&index, "process").id);
}
#[test]
fn declared_paths_and_literal_includes_can_share_a_module_across_files() {
    let s = snapshot(&[
        (
            "Cargo.toml",
            "[package]\nname='audio'\n[lib]\nname='custom_audio'\npath='root.rs'\n",
        ),
        (
            "root.rs",
            "#[path=\"parts/worker.rs\"] mod engine; include!(\"shared.rs\"); fn root_fn(){}",
        ),
        ("parts/worker.rs", "pub fn work(){}"),
        ("shared.rs", "fn included(){}"),
    ]);
    let index = Structure::new(&s);
    assert_eq!(module(&index, "work").label, "custom_audio::engine");
    assert_eq!(module(&index, "included"), module(&index, "root_fn"));
}
#[test]
fn ambiguity_invalid_manifests_or_undeclared_files_do_not_manufacture_membership() {
    let s = snapshot(&[
        ("Cargo.toml", "[package]\nname='audio'\n"),
        (
            "src/lib.rs",
            "#[path=\"shared.rs\"] mod a; #[path=\"shared.rs\"] mod b;",
        ),
        ("src/shared.rs", "pub fn ambiguous(){}"),
        ("src/orphan.rs", "fn orphan(){}"),
    ]);
    let index = Structure::new(&s);
    for name in ["ambiguous", "orphan"] {
        assert!(
            index.functions("", name, 0).unwrap().items[0]
                .symbol
                .module
                .is_none()
        );
    }
    let invalid = Structure::new(&snapshot(&[
        ("Cargo.toml", "not valid [["),
        ("src/lib.rs", "fn nope(){}"),
    ]));
    assert!(
        invalid.functions("", "nope", 0).unwrap().items[0]
            .symbol
            .module
            .is_none()
    );
}
#[test]
fn identities_survive_body_and_line_edits_but_separate_workspace_roots() {
    let files = [
        ("one/Cargo.toml", "[package]\nname='same'\n"),
        ("one/src/lib.rs", "fn first(){}"),
        ("two/Cargo.toml", "[package]\nname='same'\n"),
        ("two/src/lib.rs", "fn second(){}"),
    ];
    let old = Structure::new(&snapshot(&files));
    let changed = Structure::new(&snapshot(&[
        ("one/Cargo.toml", files[0].1),
        ("one/src/lib.rs", "// moved\nfn first(){ updated(); }"),
        ("two/Cargo.toml", files[2].1),
        ("two/src/lib.rs", files[3].1),
    ]));
    assert_eq!(module(&old, "first"), module(&changed, "first"));
    assert_ne!(module(&old, "first").id, module(&old, "second").id);
}
#[test]
fn python_modules_are_language_scopes_not_classes_or_generic_folders() {
    let s = snapshot(&[
        ("pkg/__init__.py", ""),
        (
            "pkg/worker.py",
            "class One:\n def run(self): pass\nclass Two:\n def stop(self): pass\n",
        ),
        ("pkg/api.py", "def entry(): pass\n"),
    ]);
    let index = Structure::new(&s);
    assert_eq!(module(&index, "run"), module(&index, "stop"));
    assert_eq!(module(&index, "run").label, "pkg.worker");
    assert_eq!(module(&index, "entry").label, "pkg.api");
    assert_ne!(module(&index, "run").id, module(&index, "entry").id);
}
#[test]
fn cargo_manifests_are_selected_as_immutable_project_inputs_without_an_analyser() {
    let languages = Languages::builtin().unwrap();
    let policy =
        SourcePolicy::new(PolicyConfig::default(), languages.names().map(String::from)).unwrap();
    let selected = policy.classify("crate/Cargo.toml").unwrap().unwrap();
    assert!(matches!(
        selected,
        Classification::SourceOnly {
            reason: SourceOnlyReason::ProjectMetadata,
            ..
        }
    ));
    assert!(matches!(
        policy.classify("crate/unrelated.toml").unwrap(),
        Some(Classification::SourceOnly {
            reason: SourceOnlyReason::AnalysisNotRequested,
            ..
        })
    ));
    assert!(policy.classify("target/Cargo.toml").unwrap().is_none());
    let index = Structure::new(&snapshot(&[
        ("Cargo.toml", "[package]\nname='audio'\n"),
        ("src/lib.rs", "fn start(){}"),
        ("src/other.rs", "fn another(){}"),
    ]));
    let inventory = index.functions("", "", 0).unwrap();
    assert_eq!(inventory.coverage.available_files, 2);
    assert_eq!(inventory.coverage.indexed_files, 2);
}
#[test]
fn cached_module_results_match_clean_rebuilds_after_declarations_and_manifests_change() {
    let mut cache = vizzi_core::modules::ModuleCache::default();
    let states = [
        ("[package]\nname='audio'\n", "mod engine;", "fn sample(){}"),
        (
            "[package]\nname='audio'\n",
            "// shifted\nmod engine;",
            "fn sample(){ changed(); }",
        ),
        (
            "[package]\nname='renamed'\n",
            "mod engine;",
            "fn sample(){}",
        ),
        (
            "[package]\nname='renamed'\n",
            "#[path=\"engine.rs\"] mod r#loop;",
            "fn sample(){}",
        ),
        ("[package]\nname='renamed'\n", "", "fn sample(){}"),
    ];
    for (manifest, root, child) in states {
        let captured = snapshot(&[
            ("Cargo.toml", manifest),
            ("src/lib.rs", root),
            ("src/engine.rs", child),
        ]);
        let cached = Structure::with_module_cache(&captured, &mut cache);
        let clean = Structure::new(&captured);
        assert_eq!(
            cached.functions("", "sample", 0).unwrap().items[0]
                .symbol
                .module,
            clean.functions("", "sample", 0).unwrap().items[0]
                .symbol
                .module
        );
    }
}
#[test]
fn literal_paths_normalise_inside_the_workspace_and_comments_preserve_attributes() {
    let index = Structure::new(&snapshot(&[
        ("Cargo.toml", "[package]\nname='_audio'\n"),
        (
            "src/lib.rs",
            "#[path=r#\"./parts/../worker.rs\"#]\n// retained attribute\nmod r#loop; include!(r#\"./included.rs\"#); fn root_fn(){}",
        ),
        ("src/worker.rs", "fn from_path(){}"),
        ("src/included.rs", "fn included(){}"),
    ]));
    assert_eq!(module(&index, "from_path").label, "_audio::loop");
    assert_eq!(module(&index, "included"), module(&index, "root_fn"));
}
#[test]
fn unrepresentable_names_and_absolute_declared_paths_do_not_gain_module_labels() {
    for package in ["", "9audio", "bad!name", "a_b!", "a b"] {
        let manifest = format!("[package]\nname='{package}'\n");
        let index = Structure::new(&snapshot(&[
            ("Cargo.toml", &manifest),
            ("src/lib.rs", "fn invalid(){}"),
        ]));
        assert!(
            index.functions("", "invalid", 0).unwrap().items[0]
                .symbol
                .module
                .is_none(),
            "{package}"
        );
    }
    let index = Structure::new(&snapshot(&[
        (
            "Cargo.toml",
            "[package]\nname='audio'\n[lib]\npath='root.rs'\n",
        ),
        (
            "root.rs",
            "#[path=\"/foreign.rs\"] mod absolute; #[path=\"../outside.rs\"] mod escaped;",
        ),
        ("foreign.rs", "fn foreign(){}"),
        ("outside.rs", "fn outside(){}"),
        ("bad-name/__init__.py", "def invalid_package(): pass\n"),
    ]));
    for name in ["foreign", "outside", "invalid_package"] {
        assert!(
            index.functions("", name, 0).unwrap().items[0]
                .symbol
                .module
                .is_none(),
            "{name}"
        );
    }
}
#[test]
fn bounded_extraction_still_reaches_modules_in_ordinary_large_source_files() {
    let root = format!("/*{}*/\nmod engine;", " ".repeat(600_000));
    let engine = format!("/*{}*/\nmod leaf;", " ".repeat(600_000));
    let index = Structure::new(&snapshot(&[
        ("Cargo.toml", "[package]\nname='audio'\n"),
        ("src/lib.rs", &root),
        ("src/engine.rs", &engine),
        ("src/engine/leaf.rs", "fn leaf(){}"),
    ]));
    assert_eq!(module(&index, "leaf").label, "audio::engine::leaf");
}
#[test]
fn deeply_nested_inline_declarations_have_a_finite_extraction_boundary() {
    for depth in [16, 17] {
        let root = format!("{}mod child;{}", "mod a {".repeat(depth), "}".repeat(depth));
        let path = format!("src/{}child.rs", "a/".repeat(depth));
        let index = Structure::new(&snapshot(&[
            ("Cargo.toml", "[package]\nname='audio'\n"),
            ("src/lib.rs", &root),
            (&path, "fn leaf(){}"),
        ]));
        let member = index.functions("", "leaf", 0).unwrap().items[0]
            .symbol
            .module
            .clone();
        assert_eq!(member.is_some(), depth == 16);
    }
}
#[test]
fn module_labels_are_bounded_and_function_scopes_are_not_added_to_them() {
    let index = Structure::new(&snapshot(&[
        ("Cargo.toml", "[package]\nname='audio'\n"),
        (
            "src/lib.rs",
            "fn outer(){ fn inner(){} mod local { fn nested(){} } }",
        ),
    ]));
    assert_eq!(module(&index, "outer"), module(&index, "inner"));
    assert!(
        index.functions("", "nested", 0).unwrap().items[0]
            .symbol
            .module
            .is_none()
    );
    for length in [512, 513] {
        let manifest = format!("[package]\nname='{}'\n", "a".repeat(128));
        let root = format!(
            "mod {} {{ mod {} {{ mod {} {{ fn leaf(){{}} }} }} }}",
            "b".repeat(126),
            "c".repeat(126),
            "d".repeat(length - 386)
        );
        let index = Structure::new(&snapshot(&[
            ("Cargo.toml", &manifest),
            ("src/lib.rs", &root),
        ]));
        let m = index.functions("", "leaf", 0).unwrap().items[0]
            .symbol
            .module
            .clone();
        assert_eq!(m.is_some(), length == 512);
        if let Some(m) = m {
            assert_eq!(m.label.len(), length);
        }
    }
}
#[test]
fn oversized_roots_and_total_parse_budget_leave_unreached_modules_neutral() {
    let oversized = format!("/*{}*/ mod child;", " ".repeat(1024 * 1024));
    let index = Structure::new(&snapshot(&[
        ("Cargo.toml", "[package]\nname='audio'\n"),
        ("src/lib.rs", &oversized),
        ("src/child.rs", "fn over_file_limit(){}"),
    ]));
    assert!(
        index.functions("", "over_file_limit", 0).unwrap().items[0]
            .symbol
            .module
            .is_none()
    );
    let mut files = vec![(
        "Cargo.toml".to_owned(),
        "[package]\nname='audio'\n".to_owned(),
    )];
    for depth in 0..9 {
        let path = if depth == 0 {
            "src/lib.rs".into()
        } else {
            format!("src/{}child.rs", "child/".repeat(depth - 1))
        };
        files.push((
            path,
            format!("/*{}*/ mod child;", " ".repeat(1024 * 1024 - 64)),
        ));
    }
    files.push((
        format!("src/{}child.rs", "child/".repeat(8)),
        "fn over_total_limit(){}".into(),
    ));
    let captured = snapshot(
        &files
            .iter()
            .map(|(p, t)| (p.as_str(), t.as_str()))
            .collect::<Vec<_>>(),
    );
    let index = Structure::new(&captured);
    assert!(
        index.functions("", "over_total_limit", 0).unwrap().items[0]
            .symbol
            .module
            .is_none()
    );
}
#[test]
fn an_unrepresentable_target_path_cannot_lend_membership_to_a_short_child_path() {
    let prefix = "d/".repeat(2048);
    let manifest = format!("[package]\nname='audio'\n[lib]\npath='{prefix}root.rs'\n");
    let root = format!("#[path=\"{}leaf.rs\"] mod child;", "../".repeat(2048));
    let index = Structure::new(&snapshot(&[
        ("Cargo.toml", &manifest),
        (&format!("{prefix}root.rs"), &root),
        ("leaf.rs", "fn short_child(){}"),
    ]));
    assert!(
        index.functions("", "short_child", 0).unwrap().items[0]
            .symbol
            .module
            .is_none()
    );
}
#[test]
fn module_and_include_edge_allowances_do_not_admit_an_extra_file() {
    for includes in [false, true] {
        let root = (0..513)
            .map(|n| {
                if includes {
                    format!("include!(\"m{n}.rs\");")
                } else {
                    format!("mod m{n};")
                }
            })
            .collect::<String>();
        let mut files = vec![
            (
                "Cargo.toml".to_owned(),
                "[package]\nname='audio'\n".to_owned(),
            ),
            ("src/lib.rs".to_owned(), root),
        ];
        files.extend((0..513).map(|n| (format!("src/m{n}.rs"), format!("fn sample{n}(){{}}"))));
        let index = Structure::new(&snapshot(
            &files
                .iter()
                .map(|(p, t)| (p.as_str(), t.as_str()))
                .collect::<Vec<_>>(),
        ));
        assert!(
            index.functions("", "sample511", 0).unwrap().items[0]
                .symbol
                .module
                .is_some()
        );
        assert!(
            index.functions("", "sample512", 0).unwrap().items[0]
                .symbol
                .module
                .is_none()
        );
    }
}
#[test]
fn cargo_named_binary_defaults_and_hyphenated_names_use_declared_target_rules() {
    let s = snapshot(&[
        (
            "Cargo.toml",
            "[package]\nname='audio-pack'\n[[bin]]\nname='audio-pack'\n",
        ),
        ("src/main.rs", "fn main(){}"),
    ]);
    assert_eq!(module(&Structure::new(&s), "main").label, "audio_pack");
    let other = snapshot(&[
        (
            "Cargo.toml",
            "[package]\nname='audio'\n[[bin]]\nname='meter-tool'\n",
        ),
        ("src/bin/meter-tool/main.rs", "fn meter(){}"),
    ]);
    assert_eq!(module(&Structure::new(&other), "meter").label, "meter_tool");
}
#[test]
fn unsupported_or_malformed_target_declarations_remain_neutral() {
    for manifest in [
        "[package]\nname='audio'\nautolib='false'\n",
        "lib='wrong'\n[package]\nname='audio'\n",
        "[package]\nname='audio'\n[[bin]]\npath='src/lib.rs'\n",
    ] {
        let s = snapshot(&[("Cargo.toml", manifest), ("src/lib.rs", "fn run(){}")]);
        assert!(
            Structure::new(&s).functions("", "run", 0).unwrap().items[0]
                .symbol
                .module
                .is_none(),
            "{manifest}"
        );
    }
}
#[test]
fn automatic_binaries_respect_legacy_edition_defaults_and_captured_workspace_edition() {
    let legacy = "[package]\nname='audio'\n[[bin]]\nname='tool'\npath='tool.rs'\n";
    let old = snapshot(&[
        ("Cargo.toml", legacy),
        ("src/lib.rs", "fn library(){}"),
        ("src/main.rs", "fn automatic(){}"),
        ("tool.rs", "fn tool(){}"),
    ]);
    let i = Structure::new(&old);
    assert!(
        i.functions("", "automatic", 0).unwrap().items[0]
            .symbol
            .module
            .is_none()
    );
    assert_eq!(module(&i, "library").label, "audio");
    let inherited =
        "[package]\nname='audio'\nedition.workspace=true\n[[bin]]\nname='tool'\npath='tool.rs'\n";
    let workspace = format!(
        "# {}\n[workspace]\n[workspace.package]\nedition='2024'\n",
        " ".repeat(2048)
    );
    let modern = snapshot(&[
        ("Cargo.toml", &workspace),
        ("child/Cargo.toml", inherited),
        ("child/src/main.rs", "fn automatic(){}"),
        ("child/tool.rs", "fn tool(){}"),
    ]);
    assert_eq!(module(&Structure::new(&modern), "automatic").label, "audio");
}
#[test]
fn automatic_binary_candidates_respect_explicit_targets_and_path_conflicts() {
    let index = Structure::new(&snapshot(&[
        (
            "Cargo.toml",
            "[package]\nname='audio'\nedition='2024'\n[[bin]]\nname='tool'\npath='custom.rs'\n",
        ),
        ("custom.rs", "fn explicit(){}"),
        ("src/bin/tool.rs", "fn shadowed(){}"),
        ("src/bin/extra.rs", "fn automatic(){}"),
        ("src/bin/dupe.rs", "fn ambiguous_file(){}"),
        ("src/bin/dupe/main.rs", "fn ambiguous_directory(){}"),
    ]));
    assert_eq!(module(&index, "explicit").label, "tool");
    assert_eq!(module(&index, "automatic").label, "extra");
    for name in ["shadowed", "ambiguous_file", "ambiguous_directory"] {
        assert!(
            index.functions("", name, 0).unwrap().items[0]
                .symbol
                .module
                .is_none(),
            "{name}"
        );
    }
}
