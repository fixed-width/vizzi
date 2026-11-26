//! Landmark projection from captured source and language conventions.

use std::sync::Arc;
use vizzi_core::{
    analysis::Languages,
    capture::CapturePlan,
    model::{Document, WorkspaceId},
    policy::PolicyConfig,
    query::{LandmarkKind, Projection, ViewQuery},
};

#[test]
fn file_tiles_recognize_project_files_and_execution_entry_points() {
    use LandmarkKind::{EntryPoint, Manifest, Readme};
    assert_landmarks(&[
        ("README.md", "# Guide", Some(Readme)),
        ("Cargo.toml", "", Some(Manifest)),
        ("Cargo.lock", "", None),
        ("Gemfile", "gem 'rake'", Some(Manifest)),
        ("Rakefile", "task :default", None),
        (
            "ruby/tools-extra.gemspec",
            "Gem::Specification.new do |s|\n  s.name = 'tools'\nend",
            Some(Manifest),
        ),
        ("ruby/another.gemspec", "", Some(Manifest)),
        ("ruby/example.gemspec.rb", "", None),
        (
            "launch.rs",
            "fn main() -> std::io::Result<()> { Ok(()) }",
            Some(EntryPoint),
        ),
        (
            "async.rs",
            "#[tokio::main]\nasync fn main() {}",
            Some(EntryPoint),
        ),
        (
            "inner.rs",
            "mod x { fn main() {} }\nfn outer() { fn main() {} }",
            None,
        ),
        ("wrong.rs", "fn main(arg: u32) {}", None),
        ("generic.rs", "fn main<T>() {}", None),
        ("custom_entry.rs", "#![no_main]\nfn main() {}", None),
        ("invalid.rs", "fn main() { let = ; }", None),
        ("partial.rs", "fn main() {}\nfn other( {", Some(EntryPoint)),
        ("rust/main.rs", "// fn main() {}", None),
        (
            "launch.c",
            "#ifdef APP\nint main(void) { return 0; }\n#endif",
            Some(EntryPoint),
        ),
        (
            "launch.cpp",
            "int main(int argc, char *argv[]) { return argc; }",
            Some(EntryPoint),
        ),
        ("pointer.c", "int *main(void) { return 0; }", None),
        ("prototype.c", "int main(void);", None),
        ("static.c", "static int main(void) { return 0; }", None),
        ("wrong.c", "void main(void) {}", None),
        ("argument.c", "int main(double value) { return 0; }", None),
        (
            "inner.cpp",
            "namespace tools { int main() { return 0; } }\nstruct Tool { int main() { return 0; } };",
            None,
        ),
        (
            "Launcher.java",
            "class Launcher { public static void main(String[] args) {} }",
            Some(EntryPoint),
        ),
        (
            "VarArgs.java",
            "class VarArgs { public static void main(java.lang.String... args) {} }",
            Some(EntryPoint),
        ),
        ("Modern.java", "void main() {}", Some(EntryPoint)),
        (
            "Private.java",
            "class Private { private void main() {} }",
            None,
        ),
        (
            "Overload.java",
            "class Overload { public static void main(Object[] args) {} }",
            None,
        ),
        (
            "Wrong.java",
            "class Wrong { public int main() { return 0; } }",
            None,
        ),
        (
            "launch.py",
            "if ('__main__' == __name__):\n    run()\n",
            Some(EntryPoint),
        ),
        ("pkg/__main__.py", "start()\n", Some(EntryPoint)),
        (
            "ordinary.py",
            "def main(): pass\nif __name__ != '__main__':\n    main()\n",
            None,
        ),
        (
            "nested.py",
            "def f():\n    if __name__ == '__main__':\n        run()\n",
            None,
        ),
        (
            "run.rb",
            "if $PROGRAM_NAME == __FILE__\n  run\nend",
            Some(EntryPoint),
        ),
        ("short.rb", "run if __FILE__ == $0", Some(EntryPoint)),
        (
            "ordinary.rb",
            "def main\n  run if $0 == __FILE__\nend",
            None,
        ),
        (
            "run.cjs",
            "if (module === require /* comment */ .main) run();",
            Some(EntryPoint),
        ),
        ("run.mjs", "if (import.meta.main) run();", Some(EntryPoint)),
        ("start.ts", "if (import.meta.main) run();", Some(EntryPoint)),
        (
            "start.tsx",
            "if (import.meta.main) render(<App />);",
            Some(EntryPoint),
        ),
        (
            "helper.js",
            "function main() { if (require.main === module) run(); }",
            None,
        ),
        ("negated.js", "if (!import.meta.main) run();", None),
    ]);
}

#[test]
fn java_instance_main_requires_a_class_without_an_enclosing_instance() {
    use LandmarkKind::EntryPoint;
    assert_landmarks(&[
        (
            "Inner.java",
            "class Outer { class Inner { void main() {} } }",
            None,
        ),
        (
            "Local.java",
            "class Outer { void f() { class Local { void main() {} } } }",
            None,
        ),
        (
            "Anonymous.java",
            "class Outer { Object o = new Object() { void main() {} }; }",
            None,
        ),
        (
            "TopLevel.java",
            "class TopLevel { void main() {} }",
            Some(EntryPoint),
        ),
        (
            "Nested.java",
            "class Outer { static class Nested { void main() {} } }",
            Some(EntryPoint),
        ),
        (
            "InterfaceMember.java",
            "interface Outer { class Member { void main() {} } }",
            Some(EntryPoint),
        ),
        (
            "StaticMethod.java",
            "class Outer { class Inner { static void main() {} } }",
            Some(EntryPoint),
        ),
    ]);
}

#[test]
fn ruby_launch_guards_allow_single_expression_groups() {
    use LandmarkKind::EntryPoint;
    assert_landmarks(&[
        (
            "grouped.rb",
            "if ($PROGRAM_NAME == __FILE__)\n  run\nend",
            Some(EntryPoint),
        ),
        (
            "grouped_modifier.rb",
            "run if ((__FILE__ == $0))",
            Some(EntryPoint),
        ),
        (
            "multiple_statements.rb",
            "run if ($0 == __FILE__; true)",
            None,
        ),
    ]);
}

#[test]
fn python_launch_guards_can_be_required_by_a_conjunction() {
    use LandmarkKind::EntryPoint;
    assert_landmarks(&[
        (
            "and.py",
            "if __name__ == '__main__' and len(sys.argv) > 1:\n    run()\n",
            Some(EntryPoint),
        ),
        (
            "nested_and.py",
            "if enabled and (ready and ('__main__' == __name__)):\n    run()\n",
            Some(EntryPoint),
        ),
        (
            "optional.py",
            "if (__name__ == '__main__' and enabled) or fallback:\n    run()\n",
            None,
        ),
        (
            "or_under_and.py",
            "if (__name__ == '__main__' or fallback) and enabled:\n    run()\n",
            None,
        ),
        (
            "negated.py",
            "if not (__name__ == '__main__') and enabled:\n    run()\n",
            None,
        ),
    ]);
}

#[test]
fn rust_raw_identifier_can_name_main() {
    assert_landmarks(&[
        ("raw.rs", "fn r#main() {}", Some(LandmarkKind::EntryPoint)),
        ("raw_helper.rs", "fn r#helper() {}", None),
    ]);
}

#[test]
fn c_family_main_accepts_grouping_around_the_declarator() {
    use LandmarkKind::EntryPoint;
    assert_landmarks(&[
        (
            "grouped.c",
            "int (main)(int argc, char **argv) { return argc; }",
            Some(EntryPoint),
        ),
        (
            "grouped.cpp",
            "int ((main))(int argc, char **argv) { return argc; }",
            Some(EntryPoint),
        ),
        (
            "grouped_function.c",
            "int (main(void)) { return 0; }",
            Some(EntryPoint),
        ),
        ("grouped_pointer.c", "int (*main)(void);", None),
        (
            "grouped_pointer_return.c",
            "int *(main)(void) { return 0; }",
            None,
        ),
    ]);
}

fn assert_landmarks(cases: &[(&str, &str, Option<LandmarkKind>)]) {
    let workspace = WorkspaceId::derive(&[b"landmarks"]);
    let languages = Languages::builtin().unwrap();
    let plan = CapturePlan::new(
        PolicyConfig {
            c_family_roots: vec![".".into()],
            ..Default::default()
        },
        &languages,
    )
    .unwrap();
    let documents = cases
        .iter()
        .map(|(path, source, _)| {
            let classification = plan.policy().classify(path).unwrap().unwrap();
            let analyses = classification
                .assigned_language()
                .map(|name| {
                    languages
                        .select(name)
                        .unwrap()
                        .create()
                        .unwrap()
                        .analyze(source.as_bytes())
                        .unwrap()
                })
                .into_iter()
                .collect();
            Arc::new(
                Document::capture(
                    workspace.clone(),
                    (*path).into(),
                    source.as_bytes(),
                    analyses,
                    classification,
                )
                .unwrap(),
            )
        })
        .collect();
    let view = plan
        .snapshot(workspace, documents)
        .unwrap()
        .view(&ViewQuery {
            projection: Some(Projection::Summary),
            ..Default::default()
        })
        .unwrap();
    for &(path, _, expected) in cases {
        let file = view.files.iter().find(|file| file.path == path).unwrap();
        assert_eq!(file.landmark, expected, "{path}");
    }
}
