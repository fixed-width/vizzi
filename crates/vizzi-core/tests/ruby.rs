//! Ruby syntax and written-call evidence through the portable analysis interfaces.
use vizzi_core::{
    analysis::Languages,
    model::Coverage,
    policy::{PolicyConfig, SourcePolicy},
};

#[test]
fn ruby_call_sites_distinguish_locals_from_bare_methods_and_keep_exact_offsets() {
    let text = "class Meter\n  def process(value, gain: 1)\n    local = value\n    @meter.process_sample(local)\n    prepare local\n    ready?\n    local\n    gain\n    values.each do |item; scratch|\n      consume item\n      item\n      scratch\n    end\n  end\nend\n";
    let mut analyzer = Languages::builtin()
        .unwrap()
        .select("ruby")
        .unwrap()
        .create()
        .unwrap();
    let analysis = analyzer.analyze(text.as_bytes()).unwrap();
    let calls = analysis.calls.as_ref().expect("Ruby call evidence");
    assert_eq!(
        calls
            .sites
            .iter()
            .map(|c| c.name.as_deref())
            .collect::<Vec<_>>(),
        [
            Some("process_sample"),
            Some("prepare"),
            Some("ready?"),
            Some("each"),
            Some("values"),
            Some("consume")
        ]
    );
    for call in &calls.sites {
        assert_eq!(&text[call.span.clone()], call.spelling);
        assert_eq!(
            text.get(call.name_span.clone().unwrap()),
            call.name.as_deref()
        );
        assert_eq!(analysis.facts[call.owner.unwrap()].name, "process");
        assert!(call.target.is_none());
    }
    assert_eq!(calls.omitted, 0);
}

#[test]
fn ruby_singleton_method_identity_is_distinct_and_name_identifiers_are_exact() {
    let text = "module Audio\nclass Meter\n def run = finish\n def self.run = finish!\n def ready? = true\n def value=(value)\n  @value = value\n end\n class << self\n  def run = finish\n end\nend\nend\n";
    let mut analyzer = Languages::builtin()
        .unwrap()
        .select("ruby")
        .unwrap()
        .create()
        .unwrap();
    let analysis = analyzer.analyze(text.as_bytes()).unwrap();
    let methods = analysis
        .facts
        .iter()
        .filter(|f| f.kind == "function")
        .collect::<Vec<_>>();
    assert_eq!(
        methods.iter().map(|f| f.name.as_str()).collect::<Vec<_>>(),
        ["run", "run", "ready?", "value=", "run"]
    );
    assert!(methods.iter().all(|f| f.unique));
    for method in methods {
        assert_eq!(
            text.get(method.name_span.clone().unwrap()),
            Some(method.name.as_str())
        );
    }
}

#[test]
fn ruby_patterns_rescue_loop_variables_and_assignment_order_keep_calls_honest() {
    let cases = [
        (
            "def run; value = prepare; consume(value); end",
            vec!["prepare", "consume"],
        ),
        (
            "def run(value); case value; in ^(factory.result); ready?; end; end",
            vec!["result", "factory", "ready?"],
        ),
        (
            "def run(value); case value; in {key: other}; key; other; end; end",
            vec!["key"],
        ),
        (
            "receiver=Object.new; def receiver.run = finish",
            vec!["new", "finish"],
        ),
        (
            "def run; values.each { finish }; values.each { |value| it }; end",
            vec!["each", "values", "finish", "each", "values", "it"],
        ),
        (
            "def run; for item in items; use item; end; begin; f; rescue Error => error; error; end; end",
            vec!["items", "use", "f"],
        ),
        (
            "def run(value); case value; in {key:, nested: [first, *rest]}; use key; first; rest; end; end",
            vec!["use"],
        ),
        (
            "def run; foo; foo=1; foo; xs.each { _1 }; xs.each { it }; end",
            vec!["foo", "each", "xs", "each", "xs"],
        ),
        (
            "alias new_name old_name\ndef run(value = prepare, **options, &block); value; options; block; end",
            vec!["prepare"],
        ),
    ];
    let mut analyzer = Languages::builtin()
        .unwrap()
        .select("ruby")
        .unwrap()
        .create()
        .unwrap();
    for (text, expected) in cases {
        let result = analyzer.analyze(text.as_bytes()).unwrap();
        assert_eq!(
            result
                .calls
                .unwrap()
                .sites
                .iter()
                .filter_map(|c| c.name.as_deref())
                .collect::<Vec<_>>(),
            expected,
            "{text}"
        );
    }
}

#[test]
fn ruby_name_allowances_preserve_exact_boundaries_and_report_long_bare_calls() {
    let mut parser = Languages::builtin()
        .unwrap()
        .select("ruby")
        .unwrap()
        .create()
        .unwrap();
    for length in [128, 129] {
        let name = "m".repeat(length);
        let text = format!("def run\n{name}\nself.{name}()\nend\n");
        let result = parser.analyze(text.as_bytes()).unwrap();
        let calls = result.calls.unwrap();
        if length == 128 {
            assert_eq!(calls.sites.len(), 2);
            assert_eq!(calls.omitted, 0);
            assert!(
                calls
                    .sites
                    .iter()
                    .all(|site| site.name.as_deref() == Some(name.as_str()))
            );
        } else {
            assert_eq!(calls.sites.len(), 1);
            assert_eq!(calls.omitted, 1);
            assert!(calls.sites[0].name.is_none());
        }
        let local = format!("def run\n{name}=0\n{name}\nend");
        let local = parser.analyze(local.as_bytes()).unwrap().calls.unwrap();
        assert!(local.sites.is_empty());
    }
}

#[test]
fn ruby_local_allowance_retains_explicit_calls_without_guessing_bare_names() {
    let mut analyzer = Languages::builtin()
        .unwrap()
        .select("ruby")
        .unwrap()
        .create()
        .unwrap();
    for (locals, bare, omitted) in [(4096, true, 0), (4097, false, 1)] {
        let text = format!(
            "def run\n{}finish\nself.explicit()\nend\n",
            (0..locals)
                .map(|n| format!("local_{n}=0\n"))
                .collect::<String>()
        );
        let analysis = analyzer.analyze(text.as_bytes()).unwrap();
        let calls = analysis.calls.unwrap();
        assert_eq!(
            calls
                .sites
                .iter()
                .any(|c| c.name.as_deref() == Some("finish")),
            bare
        );
        assert!(
            calls
                .sites
                .iter()
                .any(|c| c.name.as_deref() == Some("explicit"))
        );
        assert_eq!(calls.omitted, omitted);
    }
}

#[test]
fn ruby_unicode_constants_and_literal_contents_keep_their_source_meaning() {
    let text = "module Café\n VERSION = 'ready?'\n def prêt?(value)\n  phrase = %q[not_a_call(foo)]\n  pattern = /not_a_call\\(foo\\)/\n  symbol = :'not a call'\n  résultat(value)\n end\nend\n";
    let mut parser = Languages::builtin()
        .unwrap()
        .select("ruby")
        .unwrap()
        .create()
        .unwrap();
    let analysis = parser.analyze(text.as_bytes()).unwrap();
    assert_eq!(analysis.coverage, Coverage::Syntax);
    assert!(
        analysis
            .facts
            .iter()
            .any(|f| f.kind == "constant" && f.name == "VERSION")
    );
    assert_eq!(
        analysis
            .calls
            .unwrap()
            .sites
            .iter()
            .filter_map(|c| c.name.as_deref())
            .collect::<Vec<_>>(),
        ["résultat"]
    );
    let spans = analysis
        .coloring
        .unwrap()
        .spans(0..text.len(), 4096)
        .unwrap();
    assert!(
        spans
            .iter()
            .any(|span| span.kind == vizzi_core::coloring::Style::Type
                && &text[span.start..span.end] == "Café")
    );
}

#[test]
fn ruby_source_is_selected_without_a_runtime_or_build() {
    let languages = Languages::builtin().unwrap();
    let policy = SourcePolicy::new(
        PolicyConfig::default(),
        languages.names().map(str::to_owned),
    )
    .unwrap();
    for path in [
        "app.rb",
        "tasks/build.rake",
        "project.gemspec",
        "config.ru",
        "Gemfile",
        "Rakefile",
    ] {
        assert_eq!(
            policy.classify(path).unwrap().unwrap().assigned_language(),
            Some("ruby"),
            "{path}"
        );
    }
    let mut analyzer = languages
        .select("ruby")
        .expect("Ruby adapter")
        .create()
        .unwrap();
    let text = b"module Audio\n  class Meter\n    def process(value) = value\n  end\nend\n";
    let analysis = analyzer.analyze(text).unwrap();
    assert_eq!(analysis.coverage, Coverage::Syntax);
    assert_eq!(
        analysis
            .facts
            .iter()
            .map(|f| (f.kind.as_str(), f.name.as_str()))
            .collect::<Vec<_>>(),
        [
            ("module", "Audio"),
            ("class", "Meter"),
            ("function", "process")
        ]
    );
    assert_eq!(analysis.facts[2].parent, Some(1));
    for fact in &analysis.facts {
        assert_eq!(&text[fact.name_span.clone().unwrap()], fact.name.as_bytes());
    }
    assert!(analysis.coloring.as_ref().unwrap().available());
}
