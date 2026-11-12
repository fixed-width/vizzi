//! Persisted launch settings are tested through the executable's public interface.
use std::{fs, process::Command};

#[test]
fn config_paths_and_cli_overrides_control_capture() {
    let temp = tempfile::tempdir().unwrap();
    let settings = temp.path().join("settings");
    let root = settings.join("project");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("keep.py"), "def keep(): pass\n").unwrap();
    fs::write(root.join("skip.py"), "def skip(): pass\n").unwrap();
    fs::write(
        settings.join("vizzi.toml"),
        "root = 'project'\ncache = '../cache'\nworkers = 0\nexclude-source = ['skip.py']\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_vizzi"))
        .current_dir(temp.path())
        .args([
            "--config",
            "settings/vizzi.toml",
            "--once",
            "--workers",
            "1",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let status: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(status["measurements"]["files"], 1);
    assert_eq!(
        status["name"],
        root.canonicalize().unwrap().to_str().unwrap()
    );
    assert!(temp.path().join("cache").exists());
}

#[test]
fn help_does_not_open_the_selected_config() {
    let output = Command::new(env!("CARGO_BIN_EXE_vizzi"))
        .args(["--config", "/missing/vizzi.toml", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(help.contains("--config FILE"));
    assert!(help.contains("--vscode-connection FILE"));
}

#[test]
fn invalid_configurations_fail_before_creating_analysis_storage() {
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("vizzi.toml");
    let cache = temp.path().join("cache");
    let root = temp.path().join("project");
    fs::create_dir(&root).unwrap();
    for invalid in [
        "unknown = 1",
        "workers = 'one'",
        "workers = 0",
        "workers = 33",
        "port = 65536",
        "once = true",
        "stdio = true",
        "syntax-language = ['.=rust']",
        "[language-servers]\nunknown = 'tool'",
        "[language-servers]\npython-path = 'python'",
        "[language-servers]\ncompile-commands = 'commands.json'",
        "[language-servers]\nvscode-connection = 'bridge.json'\nrust-analyzer = 'rust-analyzer'",
        "root = [",
    ] {
        fs::write(&config, invalid).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_vizzi"))
            .arg("--root")
            .arg(&root)
            .arg("--config")
            .arg(&config)
            .arg("--cache")
            .arg(&cache)
            .arg("--once")
            .output()
            .unwrap();
        assert!(!output.status.success(), "accepted {invalid}");
        assert!(!cache.exists());
    }
    fs::write(&config, " ".repeat(1024 * 1024 + 1)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_vizzi"))
        .arg("--root")
        .arg(&root)
        .arg("--cache")
        .arg(&cache)
        .arg("--config")
        .arg(&config)
        .arg("--once")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("1 MiB"));
    fs::write(&config, "#".repeat(1024 * 1024)).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_vizzi"))
        .arg("--root")
        .arg(&root)
        .arg("--cache")
        .arg(&cache)
        .arg("--config")
        .arg(&config)
        .arg("--once")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn file_and_cli_source_selection_rules_are_combined() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    fs::create_dir(&root).unwrap();
    for name in ["keep.py", "file-skip.py", "cli-skip.py"] {
        fs::write(root.join(name), "def item(): pass").unwrap();
    }
    let config = temp.path().join("vizzi.toml");
    fs::write(
        &config,
        "root = 'project'\ncache = 'cache'\nexclude-source = ['file-skip.py']\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_vizzi"))
        .arg("--config")
        .arg(&config)
        .args(["--once", "--exclude-source", "cli-skip.py"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let status: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(status["measurements"]["files"], 1);
}

#[test]
fn editor_configuration_is_lazy_and_never_reads_repository_settings_implicitly() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("project");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("a.py"), "def item(): pass").unwrap();
    fs::write(root.join("vizzi.toml"), "this is invalid").unwrap();
    let config = temp.path().join("vizzi.toml");
    fs::write(
        &config,
        "root = 'project'\ncache = 'cache'\n[language-servers]\nvscode-connection = 'missing.json'",
    )
    .unwrap();
    for args in [vec!["--config", config.to_str().unwrap()], vec![]] {
        let output = Command::new(env!("CARGO_BIN_EXE_vizzi"))
            .current_dir(&root)
            .args(args)
            .arg("--cache")
            .arg(temp.path().join("cache"))
            .arg("--once")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
