//! Isolated Ruby LSP startup; project Ruby configuration is never evaluated.
use crate::{definitions::Position, lsp::Setup};
use serde_json::json;
use std::{collections::BTreeSet, ffi::OsString, fs, path::Path};

pub(super) fn matches_definition(
    locations: Vec<(Option<std::path::PathBuf>, Position)>,
    expected: &Path,
    position: Position,
) -> bool {
    let expected = fs::canonicalize(expected).unwrap_or_else(|_| expected.to_owned());
    let targets = locations
        .into_iter()
        .filter(|(path, _)| {
            !path
                .as_ref()
                .is_some_and(|path| path.extension().is_some_and(|ext| ext == "rbs"))
        })
        .map(|(path, p)| {
            (
                path.map(|path| fs::canonicalize(&path).unwrap_or(path)),
                p.line,
                p.character,
            )
        })
        .collect::<BTreeSet<_>>();
    targets.len() == 1
        && targets.iter().next().is_some_and(|(path, line, column)| {
            path.as_ref() == Some(&expected)
                && *line == position.line
                && *column == position.character
        })
}

fn lockfile(bytes: &[u8]) -> Result<String, String> {
    let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
    let mut specs = BTreeSet::new();
    let mut gems = false;
    for line in text.lines() {
        if !line.starts_with(' ') && !line.is_empty() {
            gems = line == "GEM";
            continue;
        }
        if !gems || !line.starts_with("    ") || line.starts_with("     ") {
            continue;
        }
        let Some((name, version)) = line.trim().split_once(" (") else {
            return Err("Malformed Ruby gem specification in Gemfile.lock".into());
        };
        let version = version
            .strip_suffix(')')
            .ok_or("Malformed Ruby gem version in Gemfile.lock")?;
        let valid = |value: &str| {
            !value.is_empty()
                && value.len() <= 128
                && value
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
        };
        if !valid(name) || !valid(version) {
            return Err("Unsupported Ruby gem name/version in Gemfile.lock".into());
        }
        specs.insert(format!("    {name} ({version})\n"));
        if specs.len() > 2048 {
            return Err("Ruby gem list exceeds allowance".into());
        }
    }
    Ok(format!(
        "GEM\n  specs:\n{}\nPLATFORMS\n  ruby\n\nDEPENDENCIES\n",
        specs.into_iter().collect::<String>()
    ))
}
pub(super) fn prepare(mirror: &Path, lock: Option<&[u8]>) -> Result<Setup, String> {
    let control = mirror.join(".vizzi-ruby");
    fs::create_dir_all(&control).map_err(|e| e.to_string())?;
    // Ruby Indexer's empty-directory glob otherwise registers root files as //file.rb.
    fs::create_dir_all(mirror.join("vizzi-ruby-context")).map_err(|e| e.to_string())?;
    fs::write(control.join("Gemfile"), b"").map_err(|e| e.to_string())?;
    fs::write(
        control.join("Gemfile.lock"),
        lockfile(lock.unwrap_or_default())?,
    )
    .map_err(|e| e.to_string())?;
    fs::write(
        control.join("vizzi_ruby_lsp.rb"),
        include_str!("ruby-bootstrap.rb"),
    )
    .map_err(|e| e.to_string())?;
    let mut environment = std::env::vars_os()
        .filter(|(key, _)| {
            key.to_str().is_some_and(|key| {
                key.starts_with("BUNDLE_") || matches!(key, "RUBYOPT" | "RUBYLIB")
            })
        })
        .map(|(key, _)| (key, None))
        .collect::<Vec<_>>();
    environment.extend([
        (
            OsString::from("BUNDLE_GEMFILE"),
            Some(control.join("Gemfile").into_os_string()),
        ),
        (OsString::from("BUNDLE_IGNORE_CONFIG"), Some("1".into())),
        (
            OsString::from("RUBY_LSP_BYPASS_TYPECHECKER"),
            Some("1".into()),
        ),
        (OsString::from("RUBYOPT"), Some("-rvizzi_ruby_lsp".into())),
        (
            OsString::from("RUBYLIB"),
            Some(std::env::join_paths([control]).map_err(|e| e.to_string())?),
        ),
    ]);
    Ok(Setup {
        arguments: vec![],
        environment,
        initialization: json!({"enabledFeatures":["definition","documentSymbols"],"formatter":"none","linters":[],"indexing":{"includedPatterns":["**/*.rb","**/*.rake","**/*.gemspec","**/*.ru","**/Gemfile","**/Rakefile"],"excludedPatterns":[]}}),
        configuration: json!({}),
        status_notifications: false,
        progress_token: Some("indexing-progress".into()),
        reopen_document_sync: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicate_uris_and_type_signatures_do_not_create_false_ambiguity() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("app.rb");
        fs::write(&file, b"def process; end").unwrap();
        let position = Position {
            line: 0,
            character: 4,
        };
        let same = Some(file.clone());
        let alias = Some(root.path().join("./app.rb"));
        let signature = Some(root.path().join("app.rbs"));
        assert!(matches_definition(
            vec![
                (same.clone(), position),
                (alias, position),
                (signature, position)
            ],
            &file,
            position
        ));
        assert!(!matches_definition(
            vec![
                (same.clone(), position),
                (
                    same.clone(),
                    Position {
                        line: 1,
                        character: 4
                    }
                )
            ],
            &file,
            position
        ));
        assert!(!matches_definition(
            vec![(same.clone(), position), (None, position)],
            &file,
            position
        ));
        assert!(!matches_definition(vec![(None, position)], &file, position));
        assert!(!matches_definition(vec![], &file, position));
        for wrong in [
            Position {
                line: 1,
                character: 4,
            },
            Position {
                line: 0,
                character: 5,
            },
        ] {
            assert!(!matches_definition(
                vec![(same.clone(), wrong)],
                &file,
                position
            ));
        }
        assert!(!matches_definition(
            vec![(Some(root.path().join("other.rb")), position)],
            &file,
            position
        ));
        assert!(!matches_definition(
            vec![(Some(root.path().join("app.rbs")), position)],
            &file,
            position
        ));
    }
    #[test]
    fn inherited_bundle_settings_are_removed_without_mutating_the_parent_environment() {
        const FILTER: &str = "language_servers::ruby::tests::inherited_bundle_settings_are_removed_without_mutating_the_parent_environment";
        if std::env::var_os("VIZZI_RUBY_ENV_CHILD").is_none() {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", FILTER])
                .env("VIZZI_RUBY_ENV_CHILD", "1")
                .env("BUNDLE_PATH", "untrusted-bundle")
                .env("BUNDLE_APP_CONFIG", "untrusted-settings")
                .env("RUBYOPT", "-runtrusted")
                .env("RUBYLIB", "untrusted-libraries")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return;
        }
        let root = tempfile::tempdir().unwrap();
        let env = prepare(root.path(), None)
            .unwrap()
            .environment
            .into_iter()
            .collect::<std::collections::BTreeMap<_, _>>();
        for name in ["BUNDLE_PATH", "BUNDLE_APP_CONFIG"] {
            assert_eq!(env[&OsString::from(name)], None);
        }
        assert_eq!(
            env[&OsString::from("RUBYOPT")],
            Some("-rvizzi_ruby_lsp".into())
        );
        assert_ne!(
            env[&OsString::from("RUBYLIB")],
            Some("untrusted-libraries".into())
        );
    }
    #[test]
    fn startup_uses_a_private_bundle_and_sanitized_registry_gem_metadata() {
        let root = tempfile::tempdir().unwrap();
        let lock=b"GIT\n  remote: ../untrusted\n  specs:\n    ignored (1.0)\n\nGEM\n  remote: https://example.invalid\n  specs:\n    audio (1.2.3)\n      helper (>= 0)\n    helper (2.0.0-x86_64-linux)\n\nPATH\n  remote: ../local\n  specs:\n    local (1.0)\n\nDEPENDENCIES\n  audio\n";
        let setup = prepare(root.path(), Some(lock)).unwrap();
        let control = root.path().join(".vizzi-ruby");
        assert!(root.path().join("vizzi-ruby-context").is_dir());
        assert_eq!(fs::read(control.join("Gemfile")).unwrap(), b"");
        let saved = fs::read_to_string(control.join("Gemfile.lock")).unwrap();
        assert!(saved.contains("audio (1.2.3)"));
        assert!(saved.contains("helper (2.0.0-x86_64-linux)"));
        for absent in ["untrusted", "https:", "local (", "ignored ("] {
            assert!(!saved.contains(absent));
        }
        assert!(setup.arguments.is_empty());
        assert!(setup.reopen_document_sync);
        assert_eq!(setup.progress_token.as_deref(), Some("indexing-progress"));
        let environment = setup
            .environment
            .into_iter()
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(
            environment[&OsString::from("BUNDLE_GEMFILE")],
            Some(control.join("Gemfile").into_os_string())
        );
        assert_eq!(
            environment[&OsString::from("RUBYOPT")],
            Some("-rvizzi_ruby_lsp".into())
        );
        assert_eq!(
            environment[&OsString::from("BUNDLE_IGNORE_CONFIG")],
            Some("1".into())
        );
        assert_eq!(
            environment[&OsString::from("RUBY_LSP_BYPASS_TYPECHECKER")],
            Some("1".into())
        );
        assert_eq!(
            environment[&OsString::from("RUBYLIB")],
            Some(control.into_os_string())
        );
        assert_eq!(setup.initialization["formatter"], "none");
        assert_eq!(setup.initialization["linters"], json!([]));
    }
    #[test]
    fn lock_metadata_bounds_and_invalid_data_are_explicit() {
        let root = tempfile::tempdir().unwrap();
        for (length, accepted) in [(128, true), (129, false)] {
            let lock = format!(
                "GEM\n  specs:\n    {} ({})\n",
                "a".repeat(length),
                "1".repeat(length)
            );
            assert_eq!(
                prepare(root.path(), Some(lock.as_bytes())).is_ok(),
                accepted
            );
        }
        for (count, accepted) in [(2048, true), (2049, false)] {
            let lock = format!(
                "GEM\n  specs:\n{}",
                (0..count)
                    .map(|n| format!("    gem_{n} (1.0)\n"))
                    .collect::<String>()
            );
            assert_eq!(
                prepare(root.path(), Some(lock.as_bytes())).is_ok(),
                accepted
            );
        }
        for invalid in [
            "GEM\n    malformed\n",
            "GEM\n    evil#{code} (1)\n",
            "GEM\n    okay (bad/version)\n",
            "GEM\n    a ()\n",
            "GEM\n    (1)\n",
            "GEM\n    a (1\n",
        ] {
            assert!(
                prepare(root.path(), Some(invalid.as_bytes())).is_err(),
                "{invalid}"
            );
        }
        assert!(prepare(root.path(), Some(&[0xff])).is_err());
        let file = root.path().join("not-a-directory");
        fs::write(&file, b"").unwrap();
        assert!(prepare(&file, None).is_err());
    }
}
