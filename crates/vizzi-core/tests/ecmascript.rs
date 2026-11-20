//! JavaScript and TypeScript evidence through the built-in registry and capture policy.

use vizzi_core::{
    analysis::{Analyzer, Languages},
    capture::CapturePlan,
    coloring::Style,
    model::{Analysis, Coverage, Document, WorkspaceId},
    policy::{Classification, PolicyConfig, Provenance, SourceOnlyReason, SourcePolicy},
};

fn parser(language: &str) -> Box<dyn Analyzer> {
    Languages::builtin()
        .unwrap()
        .select(language)
        .unwrap_or_else(|| panic!("missing {language} adapter"))
        .create()
        .unwrap()
}

fn outline(analysis: &Analysis) -> Vec<(&str, &str, Option<&str>)> {
    analysis
        .facts
        .iter()
        .map(|fact| {
            (
                fact.kind.as_str(),
                fact.name.as_str(),
                fact.parent
                    .map(|ordinal| analysis.facts[ordinal].name.as_str()),
            )
        })
        .collect()
}

#[test]
fn source_policy_selects_dialects_and_skips_nested_dependencies() {
    let languages = Languages::builtin().unwrap();
    let policy = SourcePolicy::new(
        PolicyConfig {
            exclusions: vec!["packages/generated".into()],
            ..Default::default()
        },
        languages.names().map(str::to_owned),
    )
    .unwrap();
    for (suffix, language) in [
        ("js", "javascript"),
        ("jsx", "javascript"),
        ("mjs", "javascript"),
        ("cjs", "javascript"),
        ("ts", "typescript"),
        ("mts", "typescript"),
        ("cts", "typescript"),
        ("tsx", "tsx"),
    ] {
        assert_eq!(
            policy
                .classify(&format!("packages/view.d.{suffix}"))
                .unwrap(),
            Some(Classification::Assigned {
                language: language.into(),
                provenance: Provenance::Extension {
                    extension: suffix.into()
                },
            })
        );
    }
    for path in [
        "node_modules",
        "packages/app/node_modules",
        "packages/app/node_modules/dep/index.js",
    ] {
        assert!(policy.excludes(path).unwrap(), "{path}");
        assert_eq!(policy.classify(path).unwrap(), None);
    }
    for path in ["packages/generated/result.ts", "src/upper.TS", "src/.tsx"] {
        assert_eq!(policy.classify(path).unwrap(), None, "{path}");
    }
    assert!(
        policy
            .classify("packages/node_modules_copy/source.js")
            .unwrap()
            .is_some()
    );
    for path in ["package.json", "app/tsconfig.json", "app/jsconfig.json"] {
        assert!(matches!(
            policy.classify(path).unwrap(),
            Some(Classification::SourceOnly {
                reason: SourceOnlyReason::ProjectMetadata,
                ..
            })
        ));
    }
    let missing = SourcePolicy::new(PolicyConfig::default(), []).unwrap();
    assert!(matches!(
        missing.classify("view.tsx").unwrap(),
        Some(Classification::SourceOnly {
            reason: SourceOnlyReason::AnalyzerUnavailable,
            language: Some(language),
            ..
        }) if language == "tsx"
    ));
}

#[test]
fn named_object_methods_and_outline_only_entries_preserve_call_ownership() {
    let text = r#"function run() {
  const actions = ({
    state: prepare(),
    "save"() { persist(); },
    [key()]() { dispatch(); },
  });
  each(items, (() => consume()));
  invoke({ spare() { ignore(); } });
  const { lost } = { lost() { ignoreBinding(); } };
  actions.save();
}
"#;
    let analysis = parser("javascript").analyze(text.as_bytes()).unwrap();
    assert_eq!(analysis.coverage, Coverage::Syntax);
    assert_eq!(
        outline(&analysis),
        [
            ("function", "run", None),
            ("object", "actions", Some("run")),
            ("function", "save", Some("actions")),
            ("computed_method", "[key()]", Some("actions")),
            ("callback", "callback", Some("run")),
        ]
    );
    let save = &analysis.facts[2];
    assert_eq!(&text[save.name_span.clone().unwrap()], "save");
    assert_eq!(&text[analysis.facts[4].span.clone()], "() => consume()");
    assert_eq!(analysis.facts[4].name_span, None);
    assert_eq!(
        analysis
            .calls
            .unwrap()
            .sites
            .iter()
            .map(|call| (call.name.as_deref(), call.owner))
            .collect::<Vec<_>>(),
        [
            (Some("prepare"), Some(0)),
            (Some("persist"), Some(2)),
            (Some("key"), Some(0)),
            (Some("dispatch"), Some(0)),
            (Some("each"), Some(0)),
            (Some("consume"), Some(0)),
            (Some("invoke"), Some(0)),
            (Some("ignore"), Some(0)),
            (Some("ignoreBinding"), Some(0)),
            (Some("save"), Some(0))
        ]
    );
}

#[test]
fn quoted_method_definition_accepts_the_opening_quote_and_keeps_the_exact_name() {
    let text = "class Service { \"save\"() {} 'load'() {} }\nfunction run(service: Service) { service.save(); }";
    let workspace = WorkspaceId::derive(&[b"quoted-method"]);
    let languages = Languages::builtin().unwrap();
    let plan = CapturePlan::new(PolicyConfig::default(), &languages).unwrap();
    let path = "service.ts";
    let doc = Document::capture(
        workspace.clone(),
        path.into(),
        text.as_bytes(),
        vec![parser("typescript").analyze(text.as_bytes()).unwrap()],
        plan.policy().classify(path).unwrap().unwrap(),
    )
    .unwrap();
    let snapshot = plan
        .snapshot(workspace, vec![std::sync::Arc::new(doc)])
        .unwrap();
    let index = vizzi_core::structure::Structure::new(&snapshot);
    for (name, token) in [("save", "\"save\""), ("load", "'load'")] {
        let quote = text.find(token).unwrap();
        let target = index.definition(path, quote).expect(name);
        assert_eq!(target.name, name);
        assert_eq!(index.identifier(&target.id), Some(quote + 1));
        assert_eq!(index.definition(path, quote + 1).unwrap().id, target.id);
        assert!(index.definition(path, quote + token.len() - 1).is_none());
    }
    let owner = &index.functions("", "run", 0).unwrap().items[0].symbol;
    let calls = index.outgoing(&owner.id, path, 0).unwrap();
    assert_eq!(calls.items[0].candidates[0].name, "save");
}

#[test]
fn captured_definition_anchors_require_an_exact_quoted_name() {
    let text = "class Service { \"save\"() {} load() {} }";
    let workspace = WorkspaceId::derive(&[b"untrusted-definition-anchor"]);
    let languages = Languages::builtin().unwrap();
    let plan = CapturePlan::new(PolicyConfig::default(), &languages).unwrap();
    let path = "service.ts";
    let classification = plan.policy().classify(path).unwrap().unwrap();
    let analysis = parser("typescript").analyze(text.as_bytes()).unwrap();
    let quoted = text.find("\"save\"").unwrap();
    let unquoted = text.find("load").unwrap();
    for (ordinal, anchor, keep_span) in [
        (1, quoted + 1, true),
        (1, quoted, false),
        (1, usize::MAX, true),
        (2, unquoted - 1, true),
    ] {
        let mut invalid = analysis.clone();
        invalid.facts[ordinal].definition_anchor = Some(anchor);
        if !keep_span {
            invalid.facts[ordinal].name_span = None;
        }
        let error = Document::capture(
            workspace.clone(),
            path.into(),
            text.as_bytes(),
            vec![invalid],
            classification.clone(),
        )
        .unwrap_err();
        assert!(error.to_string().contains("definition anchor"));
    }
}

#[test]
fn long_computed_keys_keep_bounded_labels_and_distinct_lexical_identities() {
    let prefix = "é".repeat(16_384);
    let boundary = format!("[\"{}\"]", "x".repeat(124));
    let nested = (0..512)
        .map(|n| format!("function child_{n}() {{}}\n"))
        .collect::<String>();
    let text = format!(
        "class Service {{ [\"z{prefix}A\"]() {{ {nested} }} [\"z{prefix}B\"]() {{ function child_0() {{}} }} {boundary}() {{}} }}"
    );
    let analysis = parser("javascript").analyze(text.as_bytes()).unwrap();
    assert_eq!(analysis.coverage, Coverage::Syntax);
    let methods = analysis
        .facts
        .iter()
        .filter(|fact| fact.kind == "computed_method")
        .collect::<Vec<_>>();
    assert_eq!(methods.len(), 3);
    let preview = format!("[\"z{}…]", "é".repeat(60));
    for method in &methods[..2] {
        assert_eq!(method.name, preview);
        assert!(method.unique);
    }
    assert_eq!(methods[2].name, boundary);
    assert_ne!(methods[0].key, methods[1].key);
    for fact in analysis.facts.iter().filter(|fact| fact.kind == "function") {
        assert!(fact.parent.is_some());
        assert!(fact.unique, "{}", fact.name);
        assert!(fact.key.len() < 256, "{}", fact.name);
    }
}

#[test]
fn package_identity_uses_the_closest_captured_manifest_for_all_dialects() {
    let languages = Languages::builtin().unwrap();
    let plan = CapturePlan::new(PolicyConfig::default(), &languages).unwrap();
    let workspace = WorkspaceId::derive(&[b"mixed-package"]);
    let capture = |root_name: &str| {
        let manifest = format!(r#"{{"name":"{root_name}"}}"#);
        let oversized =
            serde_json::json!({ "name": "layout", "description": "z".repeat(128 * 1024) })
                .to_string();
        let long_name = serde_json::json!({ "name": "z".repeat(513) }).to_string();
        let documents = [
            ("package.json", manifest.as_str()),
            ("lib.mjs", "function helper() {}"),
            ("src/entry.ts", "function entry() {}"),
            ("src/view.tsx", "const card = () => <Button />;"),
            ("sub/package.json", r#"{"name":"@org/tool"}"#),
            ("sub/one.jsx", "function nested() {}"),
            ("other/package.json", r#"{"name":"@org/tool"}"#),
            ("other/two.ts", "function sibling() {}"),
            ("unknown/package.json", r#"{"private":true}"#),
            ("unknown/a.js", "function unknown() {}"),
            ("bad/package.json", r#"{"name":"two words"}"#),
            ("bad/a.js", "function badName() {}"),
            ("scope/package.json", r#"{"name":"@org/"}"#),
            ("scope/a.js", "function badScope() {}"),
            ("array/package.json", r#"["fake"]"#),
            ("array/a.js", "function badArray() {}"),
            ("huge/package.json", oversized.as_str()),
            ("huge/a.js", "function tooLargeManifest() {}"),
            ("long/package.json", long_name.as_str()),
            ("long/a.js", "function tooLongName() {}"),
            ("dot/package.json", r#"{"name":".."}"#),
            ("dot/a.js", "function dotName() {}"),
        ]
        .into_iter()
        .map(|(path, text)| {
            let classification = plan.policy().classify(path).unwrap().expect(path);
            let analyses = classification
                .assigned_language()
                .map(|language| parser(language).analyze(text.as_bytes()).unwrap())
                .into_iter()
                .collect();
            std::sync::Arc::new(
                Document::capture(
                    workspace.clone(),
                    path.into(),
                    text.as_bytes(),
                    analyses,
                    classification,
                )
                .unwrap(),
            )
        })
        .collect();
        plan.snapshot(workspace.clone(), documents).unwrap()
    };
    let index = vizzi_core::structure::Structure::new(&capture("toolkit"));
    let module = |index: &vizzi_core::structure::Structure, name: &str| {
        index.functions("", name, 0).unwrap().items[0]
            .symbol
            .module
            .clone()
    };
    let root = module(&index, "entry").unwrap();
    assert_eq!(root.label, "toolkit");
    assert_eq!(root.root, "package.json");
    assert_eq!(module(&index, "helper"), Some(root.clone()));
    assert_eq!(module(&index, "card"), Some(root.clone()));
    let nested = module(&index, "nested").unwrap();
    assert_eq!(nested.label, "@org/tool");
    assert_eq!(nested.root, "sub/package.json");
    assert_ne!(nested.id, module(&index, "sibling").unwrap().id);
    for name in [
        "unknown",
        "badName",
        "badScope",
        "badArray",
        "tooLargeManifest",
        "tooLongName",
        "dotName",
    ] {
        assert!(module(&index, name).is_none(), "{name}");
    }
    let renamed = module(
        &vizzi_core::structure::Structure::new(&capture("renamed")),
        "entry",
    )
    .unwrap();
    assert_eq!(renamed.id, root.id);
    assert_eq!(renamed.label, "renamed");
}

#[test]
fn javascript_records_declaration_ranges_call_owners_and_dynamic_callees() {
    let text = r#"// Café
export async function begin(value) {
  const relay = function internal(arg) { return obj?.run(arg); };
  const café = function* () { yield next(); };
  return new workers.Worker(relay(value), handlers[value](value), café());
}
class Worker {
  #check() { return validate(); }
  run() { return this.#check(); }
  static run() {}
  ["dynamic"]() {}
}
const handlers = { run() { return fallback(); } };
"#;
    let analysis = parser("javascript").analyze(text.as_bytes()).unwrap();
    assert_eq!(analysis.coverage, Coverage::Syntax);
    assert_eq!(
        outline(&analysis),
        [
            ("function", "begin", None),
            ("function", "relay", Some("begin")),
            ("function", "café", Some("begin")),
            ("class", "Worker", None),
            ("function", "#check", Some("Worker")),
            ("function", "run", Some("Worker")),
            ("function", "run", Some("Worker")),
            ("computed_method", "[\"dynamic\"]", Some("Worker")),
            ("object", "handlers", None),
            ("function", "run", Some("handlers")),
        ]
    );
    for fact in &analysis.facts {
        if fact.kind == "computed_method" {
            assert!(fact.name_span.is_none());
        } else {
            let name_span = fact.name_span.clone().unwrap();
            assert_eq!(&text[name_span.clone()], fact.name);
            assert!(fact.span.start <= name_span.start && name_span.end <= fact.span.end);
        }
        assert_eq!(
            fact.unique,
            !(fact.name == "run"
                && fact
                    .parent
                    .is_some_and(|i| analysis.facts[i].name == "Worker"))
        );
    }
    assert_eq!(
        &text[analysis.facts[1].span.clone()],
        "relay = function internal(arg) { return obj?.run(arg); }"
    );
    let calls = analysis.calls.unwrap();
    let got = calls
        .sites
        .iter()
        .map(|call| {
            assert_eq!(&text[call.span.clone()], call.spelling);
            assert_eq!(
                call.name_span.as_ref().map(|range| &text[range.clone()]),
                call.name.as_deref()
            );
            assert!(call.target.is_none());
            (
                call.spelling.as_str(),
                call.name.as_deref(),
                call.owner.map(|i| analysis.facts[i].name.as_str()),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(
        got,
        [
            ("obj?.run", Some("run"), Some("relay")),
            ("next", Some("next"), Some("café")),
            ("workers.Worker", Some("Worker"), Some("begin")),
            ("relay", Some("relay"), Some("begin")),
            ("handlers[value]", None, Some("begin")),
            ("café", Some("café"), Some("begin")),
            ("validate", Some("validate"), Some("#check")),
            ("this.#check", Some("#check"), Some("run")),
            ("fallback", Some("fallback"), Some("run")),
        ]
    );
    assert_eq!(calls.omitted, 0);
}

#[test]
fn grouped_and_type_only_callees_preserve_the_written_name_and_spelling() {
    let javascript = [
        ("((actions.save))()", "((actions.save))", Some("save")),
        ("(/* note */ save)()", "(/* note */ save)", Some("save")),
        ("new (actions.Task)()", "(actions.Task)", Some("Task")),
        ("(actions[key])()", "(actions[key])", None),
        ("(actions['save'])()", "(actions['save'])", None),
        ("(other, save)()", "(other, save)", None),
    ];
    let typescript = [
        ("save!()", "save!", Some("save")),
        (
            "((actions.save as F)!)()",
            "((actions.save as F)!)",
            Some("save"),
        ),
        (
            "(actions.save satisfies F)()",
            "(actions.save satisfies F)",
            Some("save"),
        ),
        ("(<F>actions.save)()", "(<F>actions.save)", Some("save")),
        ("(save<number>)()", "(save<number>)", Some("save")),
        ("(actions[key] as F)()", "(actions[key] as F)", None),
    ];
    let tsx = [
        ("(actions.save!)()", "(actions.save!)", Some("save")),
        ("(actions[key]!)()", "(actions[key]!)", None),
    ];
    for (language, cases) in [
        ("javascript", javascript.as_slice()),
        ("typescript", typescript.as_slice()),
        ("tsx", tsx.as_slice()),
    ] {
        let mut analyzer = parser(language);
        for &(invocation, spelling, name) in cases {
            let text = format!("function run() {{ {invocation}; }}");
            let analysis = analyzer.analyze(text.as_bytes()).unwrap();
            assert_eq!(analysis.coverage, Coverage::Syntax, "{invocation}");
            let calls = analysis.calls.unwrap();
            assert_eq!(calls.sites.len(), 1, "{invocation}");
            let call = &calls.sites[0];
            assert_eq!(call.spelling, spelling);
            assert_eq!(&text[call.span.clone()], spelling);
            assert_eq!(call.name.as_deref(), name, "{invocation}");
            assert_eq!(
                call.name_span.clone().map(|range| &text[range]),
                name,
                "{invocation}"
            );
            assert_eq!(call.owner, Some(0));
        }
    }
}

#[test]
fn wrapped_function_initializers_own_calls_and_keep_the_full_declarator() {
    for (language, initializer) in [
        ("javascript", "(/* note */ (() => draw()))"),
        ("javascript", "((function* () { yield draw(); }))"),
        ("typescript", "((() => draw()) as () => void)"),
        ("tsx", "(() => <Card title={draw()} />)"),
    ] {
        let text = format!("function run() {{ const render = {initializer}, result = draw(); }}");
        let analysis = parser(language).analyze(text.as_bytes()).unwrap();
        assert_eq!(analysis.coverage, Coverage::Syntax, "{initializer}");
        assert_eq!(
            outline(&analysis),
            [
                ("function", "run", None),
                ("function", "render", Some("run"))
            ],
            "{initializer}"
        );
        let render = &analysis.facts[1];
        assert_eq!(&text[render.name_span.clone().unwrap()], "render");
        assert_eq!(
            &text[render.span.clone()],
            format!("render = {initializer}")
        );
        let calls = analysis.calls.unwrap();
        assert_eq!(
            calls
                .sites
                .iter()
                .map(|call| (call.name.as_deref(), call.owner))
                .collect::<Vec<_>>(),
            [(Some("draw"), Some(1)), (Some("draw"), Some(0))]
        );
    }
}

#[test]
fn typescript_uses_its_own_grammar_and_outlines_bodies_and_types() {
    let text = r#"interface Options { value: string }
type Alias = Options;
enum Choice { First }
abstract class Runner {
  abstract pending(): void;
  execute(arg: Alias) { return service.wrap<Alias>(<Alias>arg); }
}
function recur(x: number): number;
function recur(x: number) { return x; }
"#;
    let analysis = parser("typescript").analyze(text.as_bytes()).unwrap();
    assert_eq!(analysis.coverage, Coverage::Syntax);
    assert_eq!(
        outline(&analysis),
        [
            ("interface", "Options", None),
            ("type", "Alias", None),
            ("enum", "Choice", None),
            ("class", "Runner", None),
            ("signature", "pending", Some("Runner")),
            ("function", "execute", Some("Runner")),
            ("signature", "recur", None),
            ("function", "recur", None),
        ]
    );
    let calls = analysis.calls.unwrap();
    assert_eq!(calls.sites.len(), 1);
    let call = &calls.sites[0];
    assert_eq!(call.spelling, "service.wrap");
    assert_eq!(analysis.facts[call.owner.unwrap()].name, "execute");
    assert_eq!(
        parser("tsx").analyze(text.as_bytes()).unwrap().coverage,
        Coverage::Partial
    );
    let styles = analysis
        .coloring
        .unwrap()
        .spans(0..text.len(), 100)
        .unwrap();
    assert!(
        styles
            .iter()
            .any(|span| span.kind == Style::Keyword && &text[span.start..span.end] == "interface")
    );
    assert!(
        styles
            .iter()
            .any(|span| span.kind == Style::Type && &text[span.start..span.end] == "string")
    );
}

#[test]
fn jsx_has_coloring_and_emits_calls_only_in_expressions() {
    for (language, text) in [
        (
            "javascript",
            "const Card = value => <Widget title={format(value)} />;",
        ),
        (
            "tsx",
            "const Card = (value: number) => <Widget title={format(value)} />;",
        ),
    ] {
        let result = parser(language).analyze(text.as_bytes()).unwrap();
        assert_eq!(result.coverage, Coverage::Syntax);
        assert_eq!(outline(&result), [("function", "Card", None)]);
        let calls = result.calls.unwrap();
        assert_eq!(calls.sites.len(), 1);
        assert_eq!(calls.sites[0].name.as_deref(), Some("format"));
        assert_eq!(calls.sites[0].owner, Some(0));
        let styles = result.coloring.unwrap().spans(0..text.len(), 40).unwrap();
        assert!(styles.iter().any(|span| span.kind == Style::Attribute && &text[span.start..span.end] == "title"));
    }
}

#[test]
fn retained_trees_can_only_be_reused_within_their_grammar() {
    let mut typescript = parser("typescript");
    let mut tsx = parser("tsx");
    let text = b"const card = () => <Card />;";
    let retained = typescript
        .analyze_with_state(b"const card = () => 1;", None, usize::MAX, &|| false)
        .unwrap()
        .unwrap()
        .state
        .unwrap();
    let parsed = tsx
        .analyze_with_state(text, Some(retained), usize::MAX, &|| false)
        .unwrap()
        .unwrap();
    assert!(!parsed.tree_reused);
    assert_eq!(parsed.analysis.coverage, Coverage::Syntax);

    let edited = b"const card = () => <Card title={name()} />;";
    let updated = tsx
        .analyze_with_state(edited, parsed.state, usize::MAX, &|| false)
        .unwrap()
        .unwrap();
    assert!(updated.tree_reused);
    assert_eq!(updated.analysis, tsx.analyze(edited).unwrap());
}

#[test]
fn recovered_and_unavailable_analysis_use_shared_coverage() {
    let mut parser = parser("typescript");
    let result = parser
        .analyze(b"function ready() {}\nfunction broken(")
        .unwrap();
    assert_eq!(result.coverage, Coverage::Partial);
    assert_eq!(outline(&result), [("function", "ready", None)]);
    assert!(result.calls.is_some());
    assert!(!result.diagnostics.is_empty());

    let result = parser.analyze(&[255]).unwrap();
    assert_eq!(result.coverage, Coverage::Unavailable);
    assert!(result.facts.is_empty());
    assert!(result.calls.is_none());
}
