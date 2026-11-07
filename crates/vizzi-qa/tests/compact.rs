//! Source roles, lexical scope and compact evidence through the corpus interface.
use vizzi_qa::{Corpus, SourceFile, SourceRole};
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
fn production_retry_evidence_is_distinct_from_its_test_double_and_historical_reports() {
    let text = "pub fn retry_read() { loop { break; } }\n#[cfg(test)]\nmod tests {\n async fn retry_regression() { struct RecoveringReader { failures: usize } impl RecoveringReader { fn retry_read(&self) {} } }\n}\n";
    let c = corpus(vec![
        file("src/compiler.rs", text),
        file(
            "experiments/old/RESULTS.md",
            "retry retry failures counter controls retry retry",
        ),
    ]);
    let context = c
        .retrieve_compact("How does retry_read control retries?", 3, 2500)
        .unwrap();
    assert_eq!(context.evidence()[0].path, "src/compiler.rs");
    assert_eq!(context.evidence()[0].role, SourceRole::Code);
    assert!(context.evidence()[0].text.contains("pub fn retry_read"));
    assert!(!context.evidence()[0].text.contains("RecoveringReader"));
    let tests = c
        .retrieve_compact(
            "Which retry regression test uses RecoveringReader?",
            3,
            2500,
        )
        .unwrap();
    assert_eq!(tests.evidence()[0].role, SourceRole::Test);
    assert!(
        tests.evidence()[0]
            .scope
            .as_ref()
            .unwrap()
            .name
            .contains("retry")
            || tests.evidence()[0]
                .scope
                .as_ref()
                .unwrap()
                .parents
                .iter()
                .any(|p| p.contains("retry"))
    );
}
#[test]
fn overview_policy_and_configuration_queries_find_their_authoritative_documents() {
    let c = corpus(vec![
        file("README.md", "A compiler and networked world runtime."),
        file(
            "AGENTS.md",
            "Contributors validate changes with formatting, workspace tests and Clippy.",
        ),
        file("Cargo.toml", "[workspace.package]\nedition = \"2024\"\n"),
        file(
            "experiments/old/RESULTS.md",
            "project intended purpose workspace edition validate checks compiler compiler",
        ),
    ]);
    assert_eq!(
        c.retrieve_compact("What is this project intended for?", 3, 2500)
            .unwrap()
            .evidence()[0]
            .path,
        "README.md"
    );
    assert_eq!(
        c.retrieve_compact(
            "Which checks must a contributor run before submitting a change?",
            3,
            2500
        )
        .unwrap()
        .evidence()[0]
            .path,
        "AGENTS.md"
    );
    assert_eq!(
        c.retrieve_compact(
            "Which language edition is declared for the workspace?",
            3,
            2500
        )
        .unwrap()
        .evidence()[0]
            .path,
        "Cargo.toml"
    );
}
#[test]
fn a_named_component_question_does_not_become_a_project_overview_query() {
    let c = corpus(vec![
        file("README.md", "The project is intended to manage traffic."),
        file(
            "src/bucket.rs",
            "/// Controls request admission.\npub struct TokenBucket;\n",
        ),
    ]);
    let context = c
        .retrieve_compact("What is TokenBucket intended for?", 3, 2500)
        .unwrap();
    assert_eq!(context.evidence()[0].path, "src/bucket.rs");
}

#[test]
fn purpose_queries_distinguish_named_types_functions_and_the_project_itself() {
    let c = corpus(vec![
        file("README.md", "This project is a transport service."),
        file(
            "src/bucket.rs",
            "/// project purpose project purpose\npub struct TokenBucket;\npub fn retry_read() {}\n",
        ),
    ]);
    for question in [
        "What is the purpose of TokenBucket?",
        "What is the purpose of retry_read?",
    ] {
        assert_eq!(
            c.retrieve_compact(question, 3, 2500).unwrap().evidence()[0].path,
            "src/bucket.rs"
        );
    }
    assert_eq!(
        c.retrieve_compact("What is this project's purpose?", 3, 2500)
            .unwrap()
            .evidence()[0]
            .path,
        "README.md"
    );
}
#[test]
fn package_description_connects_an_executable_role_to_its_main_source() {
    let c = corpus(vec![
        file("README.md", "Compiler workspace."),
        file(
            "driver/Cargo.toml",
            "[package]\nname = \"driver\"\ndescription = \"A game server executable\"\n",
        ),
        file("driver/src/main.rs", "fn main() { boot_vm(); }\n"),
        file(
            "utility/Cargo.toml",
            "[package]\nname = \"utility\"\ndescription = \"Certificate helper\"\n",
        ),
        file("utility/src/main.rs", "fn main() { certificate(); }\n"),
    ]);
    assert_eq!(
        c.retrieve_compact("Where does the game server executable start?", 3, 2500)
            .unwrap()
            .evidence()[0]
            .path,
        "driver/src/main.rs"
    );
}
#[test]
fn compact_excerpts_are_literal_utf8_safe_and_obey_the_total_allowance() {
    let text = format!(
        "{}\n/// needle reader handles bytes\npub fn load_reader() {{}}\n{}",
        "// filler\n".repeat(250),
        "// 界\n".repeat(100)
    );
    let c = corpus(vec![
        file("src/reader.rs", &text),
        file(
            "docs/reader.md",
            "The reader handles bytes and reports errors.",
        ),
    ]);
    let context = c
        .retrieve_compact("How does the reader handle bytes?", 3, 2500)
        .unwrap();
    assert!(!context.evidence().is_empty());
    assert!(context.evidence().len() <= 3);
    assert!(
        context
            .evidence()
            .iter()
            .map(|e| e.text.len())
            .sum::<usize>()
            <= 2500
    );
    for source in context.evidence() {
        if source.path == "src/reader.rs" {
            assert!(text.contains(&source.text));
        }
    }
    assert!(c.retrieve_compact("reader", 0, 2500).is_err());
    assert!(c.retrieve_compact("reader", 3, 0).is_err());
}

#[test]
fn overview_uses_the_readme_introduction_instead_of_repeated_later_headings() {
    let readme = format!(
        "# Atlas\nA compiler and persistent world runtime.\n{}{}",
        "More background.\n".repeat(180),
        "\n## Application purposes\nThis application is intended for running applications.\n"
            .repeat(80)
    );
    let c = corpus(vec![file("README.md", &readme)]);
    let context = c
        .retrieve_compact(
            "What kind of applications is this project intended to run?",
            3,
            2500,
        )
        .unwrap();
    assert_eq!(context.evidence()[0].start_line, 1);
    assert!(
        context.evidence()[0]
            .text
            .contains("persistent world runtime")
    );
}

#[test]
fn path_roles_and_lexical_test_attributes_are_exposed_as_hints() {
    for (path, role) in [
        ("test/helpers.rs", SourceRole::Test),
        ("tests/helpers.rs", SourceRole::Test),
        ("src/test_reader.py", SourceRole::Test),
        ("src/reader_test.rs", SourceRole::Test),
        ("web/reader.test.js", SourceRole::Test),
        ("experiments/past/notes.md", SourceRole::Historical),
        ("docs/history.md", SourceRole::Historical),
        ("docs/results.md", SourceRole::Historical),
        ("docs/worklog.md", SourceRole::Historical),
        ("CONTRIBUTING.md", SourceRole::Policy),
        ("DESIGN.md", SourceRole::Policy),
        ("spec/PRODUCT.md", SourceRole::Policy),
        ("spec/TECH.md", SourceRole::Policy),
        ("docs/prototype/BOUNDARIES.md", SourceRole::Policy),
        ("docs/guide.md", SourceRole::Guide),
        ("config.yaml", SourceRole::Configuration),
        ("config.yml", SourceRole::Configuration),
        ("Cargo.toml", SourceRole::Configuration),
        ("src/lib.rs", SourceRole::Code),
    ] {
        let c = corpus(vec![file(path, "// sentinel\n")]);
        let context = c.retrieve_compact("sentinel", 1, 2500).unwrap();
        assert_eq!(context.evidence()[0].role, role, "{path}");
    }
    for attribute in ["#[test]", "#[tokio::test]", "#[cfg(test)]"] {
        let text = format!("{attribute}\n/// sentinel\nfn check_reader() {{}}\n");
        let c = corpus(vec![file("src/lib.rs", &text)]);
        let context = c.retrieve_compact("check_reader", 1, 2500).unwrap();
        assert_eq!(context.evidence()[0].role, SourceRole::Test, "{attribute}");
        assert_eq!(
            context.evidence()[0].scope.as_ref().unwrap().name,
            "check_reader"
        );
    }
}

#[test]
fn role_boundaries_and_parent_scopes_survive_unchanged_replacement() {
    let text = "pub fn production() {}\n#[cfg(test)]\nmod checks {\n fn nested_sentinel() {}\n}\n";
    let mut c = corpus(vec![file("src/lib.rs", text)]);
    let whole = c.retrieve("production", 1, 2500).unwrap();
    assert_eq!(whole.evidence()[0].role, SourceRole::Mixed);
    assert!(whole.evidence()[0].text.contains("mod checks"));
    for _ in 0..2 {
        let context = c.retrieve_compact("nested_sentinel", 1, 2500).unwrap();
        assert_eq!(context.evidence()[0].role, SourceRole::Test);
        let scope = context.evidence()[0].scope.as_ref().unwrap();
        assert_eq!(scope.name, "nested_sentinel");
        assert_eq!(scope.kind, "function");
        assert_eq!(scope.parents, ["checks"]);
        assert_eq!(c.replace(vec![file("src/lib.rs", text)]).unwrap().reused, 1);
    }
}

#[test]
fn long_multibyte_scope_names_are_bounded_without_cutting_utf8() {
    let name = "界".repeat(60);
    let c = corpus(vec![file(
        "src/lib.rs",
        &format!("pub fn {name}() {{ let sentinel = 0; }}"),
    )]);
    let context = c.retrieve_compact("sentinel", 1, 2500).unwrap();
    let scope = context.evidence()[0].scope.as_ref().unwrap();
    assert_eq!(scope.name, "界".repeat(42));
    assert!(scope.name.len() <= 128);
}

#[test]
fn common_query_word_forms_find_lexically_named_source_scopes() {
    for (question, name) in [
        ("configuration", "config"),
        ("configured", "config"),
        ("environment", "env"),
        ("failed", "error"),
        ("validation", "valid"),
        ("policies", "policy"),
        ("running", "run"),
        ("loading", "load"),
        ("packages", "package"),
    ] {
        let text = format!("pub fn {name}() {{}}\n");
        let c = corpus(vec![file("src/lib.rs", &text)]);
        let context = c.retrieve_compact(question, 1, 2500).unwrap();
        assert_eq!(context.evidence().len(), 1, "{question}");
        assert_eq!(
            context.evidence()[0].scope.as_ref().unwrap().name,
            name,
            "{question}"
        );
    }
}

#[test]
fn compact_selection_keeps_the_guard_around_a_match_and_complete_short_lines() {
    let text = format!(
        "fn decode_bundle() {{\n{}    if payload_has_magic {{\n        // preserve the guard around this call\n        parse_payload(incoming_bytes);\n    }}\n{}}}\n",
        "    // padding before the interesting branch\n".repeat(30),
        "    // padding after the interesting branch\n".repeat(30)
    );
    let c = corpus(vec![file("src/decode.rs", &text)]);
    let context = c
        .retrieve_compact(
            "How does payload magic control parsing incoming bytes?",
            3,
            2500,
        )
        .unwrap();
    let first = &context.evidence()[0];
    assert!(first.text.contains("if payload_has_magic {"));
    assert!(first.text.contains("parse_payload(incoming_bytes);"));
    assert!(first.text.ends_with('\n'));
    let original_lines = text.lines().collect::<Vec<_>>();
    assert_eq!(
        first.text.lines().next().unwrap(),
        original_lines[first.start_line - 1]
    );
    assert_eq!(
        first.text.lines().last().unwrap(),
        original_lines[first.end_line - 1]
    );
}

#[test]
fn unanalyzed_text_does_not_claim_a_raw_lexical_scope_basis() {
    let c = corpus(vec![file(
        "web/reader.js",
        "export const sentinel = true;\n",
    )]);
    let context = c.retrieve_compact("sentinel", 1, 2500).unwrap();
    assert_eq!(context.evidence()[0].role_basis, "path_convention");
    assert!(context.evidence()[0].scope.is_none());
    let rust = corpus(vec![file("src/lib.rs", "fn sentinel() {}\n")]);
    assert_eq!(
        rust.retrieve_compact("sentinel", 1, 2500)
            .unwrap()
            .evidence()[0]
            .role_basis,
        "raw_lexical_scope"
    );
}

#[test]
fn an_adjacent_production_scope_is_not_part_of_the_preceding_test() {
    let c = corpus(vec![file(
        "src/lib.rs",
        "#[test]\nfn check_left() {}fn production_right() {}\n",
    )]);
    let context = c.retrieve_compact("production_right", 1, 2500).unwrap();
    assert_eq!(context.evidence()[0].role, SourceRole::Code);
    assert_eq!(
        context.evidence()[0].scope.as_ref().unwrap().name,
        "production_right"
    );
}
