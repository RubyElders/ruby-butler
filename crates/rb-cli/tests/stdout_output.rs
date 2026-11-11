#![cfg(unix)]

#[path = "common/ruby_install.rs"]
mod support;
use std::{fs, process::Output};
use support::{Fixture, success};

fn report(output: Output, command: &str) -> serde_json::Value {
    let output = success(output);
    assert!(!output.stdout.contains(&0x1b));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["command"], command);
    value["data"].clone()
}

fn normalize_project_paths(output: &[u8], fixture: &Fixture) -> String {
    let root = fixture.root.path();
    std::str::from_utf8(output)
        .unwrap()
        .replace(root.canonicalize().unwrap().to_str().unwrap(), "<PROJECT>")
        .replace(root.to_str().unwrap(), "<PROJECT>")
}

#[test]
fn format_is_global_and_cli_overrides_environment() {
    let fixture = Fixture::new();
    for args in [
        vec!["-F", "json", "version"],
        vec!["version", "--format", "json"],
        vec!["version"],
    ] {
        let data = report(
            fixture
                .command()
                .env("RB_FORMAT", "json")
                .args(args)
                .output()
                .unwrap(),
            "version",
        );
        assert_eq!(data["version"], env!("CARGO_PKG_VERSION"));
    }
    let output = success(
        fixture
            .command()
            .env("RB_FORMAT", "json")
            .args(["version", "-F", "text"])
            .output()
            .unwrap(),
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("Ruby Butler"));
    assert!(serde_json::from_slice::<serde_json::Value>(&output.stdout).is_err());
    for invalid in ["toml", "", "JSON"] {
        let output = fixture
            .command()
            .env("RB_FORMAT", invalid)
            .arg("version")
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("--format"));
    }
}

#[test]
fn configuration_project_and_scripts_have_semantic_data() {
    let fixture = Fixture::new();
    fs::write(
        fixture.root.path().join("rbproject.toml"),
        include_str!("fixtures/stdout/project.toml"),
    )
    .unwrap();
    let run = |args: &[&str]| {
        fixture
            .command()
            .env("RB_FORMAT", "json")
            .env("CLICOLOR_FORCE", "1")
            .args(args)
            .output()
            .unwrap()
    };
    let config = report(run(&["info", "config"]), "info.config");
    assert_eq!(config["rubies_dir"]["source"], "cli");
    assert_eq!(
        config["rubies_dir"]["value"],
        fixture.root.path().join("rubies").to_str().unwrap()
    );
    let project = report(run(&["info", "project"]), "info.project");
    assert_eq!(project["project"]["name"], "Tea room");
    assert_eq!(
        project["project"]["scripts"]["serve"]["command"],
        "ruby app.rb"
    );
    assert_eq!(
        project["project"]["scripts"]["serve"]["description"],
        "Serve tea"
    );
    let scripts = report(run(&["run"]), "run");
    assert_eq!(scripts["scripts"], project["project"]["scripts"]);
    let output = success(
        fixture
            .command()
            .env("NO_COLOR", "1")
            .arg("run")
            .output()
            .unwrap(),
    );
    assert_eq!(
        normalize_project_paths(&output.stdout, &fixture),
        include_str!("fixtures/stdout/scripts.txt")
    );
}

#[test]
fn help_shell_and_project_creation_use_the_same_envelope() {
    let fixture = Fixture::new();
    let help = report(
        fixture
            .command()
            .args(["-F", "json", "help"])
            .output()
            .unwrap(),
        "help",
    );
    let format = help["arguments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|arg| arg["long"] == "format")
        .unwrap();
    assert_eq!(format["short"], "F");
    assert_eq!(format["environment"], "RB_FORMAT");
    assert_eq!(
        format["possible_values"],
        serde_json::json!(["text", "json"])
    );
    assert!(
        !help["commands"]
            .as_array()
            .unwrap()
            .iter()
            .any(|cmd| cmd["name"] == "__bash_complete")
    );
    let shell = report(
        fixture
            .command()
            .args(["-F", "json", "shell-integration", "bash"])
            .output()
            .unwrap(),
        "shell-integration",
    );
    let text = success(
        fixture
            .command()
            .args(["shell-integration", "bash"])
            .output()
            .unwrap(),
    );
    assert_eq!(shell["source"].as_str().unwrap().as_bytes(), text.stdout);
    let created = report(
        fixture
            .command()
            .args(["-F", "json", "new"])
            .output()
            .unwrap(),
        "new",
    );
    assert_eq!(
        created["path"],
        fixture
            .root
            .path()
            .canonicalize()
            .unwrap()
            .join("rbproject.toml")
            .to_str()
            .unwrap()
    );
    assert!(fixture.root.path().join("rbproject.toml").is_file());
    let failed = fixture
        .command()
        .args(["-F", "json", "new"])
        .output()
        .unwrap();
    assert!(!failed.status.success());
    assert!(failed.stdout.is_empty());
}

#[test]
fn completion_remains_raw_and_completes_each_format_option() {
    let fixture = Fixture::new();
    for (line, expected) in [
        ("rb -F ", "text\njson\n"),
        ("rb --format j", "json\n"),
        ("rb export-gemfile --project-format ", "toml\nkdl\n"),
    ] {
        let output = success(
            fixture
                .command()
                .env("RB_FORMAT", "json")
                .args(["__bash_complete", line, &line.len().to_string()])
                .output()
                .unwrap(),
        );
        assert_eq!(String::from_utf8(output.stdout).unwrap(), expected);
    }
}

#[test]
fn install_reports_json_and_keeps_progress_on_stderr() {
    let fixture = Fixture::new();
    fixture.seed();
    let output = success(
        fixture
            .command()
            .args(["-F", "json", "install", "ruby", "3.4.10"])
            .output()
            .unwrap(),
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("Selected Ruby 3.4.10"));
    let data = report(output, "install.ruby");
    assert_eq!(data["version"], "3.4.10");
    assert!(fixture.installed());
    let versions = report(
        fixture
            .command()
            .args(["-F", "json", "info", "runtime", "--available"])
            .output()
            .unwrap(),
        "info.runtime.available",
    );
    assert_eq!(versions[0]["version"], "3.4.10");
    assert_eq!(versions[0]["installed"], true);
}

#[test]
fn ordinary_text_keeps_existing_configuration_and_shell_layouts() {
    let fixture = Fixture::new();
    for (args, expected) in [
        (
            vec!["info", "config"],
            include_str!("fixtures/stdout/config.txt"),
        ),
        (
            vec!["shell-integration"],
            include_str!("fixtures/stdout/shells.txt"),
        ),
    ] {
        let output = success(
            fixture
                .command()
                .env("NO_COLOR", "1")
                .args(args)
                .output()
                .unwrap(),
        );
        let actual = normalize_project_paths(&output.stdout, &fixture)
            .replace(home::home_dir().unwrap().to_str().unwrap(), "<HOME>");
        assert_eq!(actual, expected);
    }
}

#[test]
fn json_does_not_wrap_child_streams_or_change_their_exit_status() {
    let fixture = Fixture::new();
    let ruby = which::which(std::env::var_os("RB_GEM_BUILD_RUBY").unwrap_or_else(|| "ruby".into()))
        .unwrap();
    let version = success(
        std::process::Command::new(&ruby)
            .args(["-e", "print RUBY_VERSION"])
            .output()
            .unwrap(),
    );
    let version = String::from_utf8(version.stdout).unwrap();
    let bin = fixture
        .root
        .path()
        .join(format!("rubies/ruby-{version}/bin"));
    fs::create_dir_all(&bin).unwrap();
    std::os::unix::fs::symlink(&ruby, bin.join("ruby")).unwrap();
    let output = fixture
        .command()
        .args([
            "-F",
            "json",
            "-B",
            "-r",
            &version,
            "x",
            "ruby",
            "-e",
            "STDOUT.write('child output'); STDERR.write('child error'); exit 7",
        ])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(7));
    assert_eq!(output.stdout, b"child output");
    assert_eq!(output.stderr, b"child error");
    let data = report(
        fixture
            .command()
            .args(["-F", "json", "-B", "-r", &version, "info", "runtime"])
            .output()
            .unwrap(),
        "info.runtime",
    );
    assert_eq!(data["installations"][0]["version"], version);
    assert_eq!(data["selected"], 0);
    let data = report(
        fixture
            .command()
            .args(["-F", "json", "-B", "-r", &version, "info", "env"])
            .output()
            .unwrap(),
        "info.env",
    );
    assert_eq!(data["ruby"]["installed"], true);
}

#[test]
fn forced_color_preserves_styles_padding_and_newlines() {
    let fixture = Fixture::new();
    fs::write(
        fixture.root.path().join("rbproject.toml"),
        include_str!("fixtures/stdout/project.toml"),
    )
    .unwrap();
    let output = success(
        fixture
            .command()
            .env_remove("NO_COLOR")
            .env("CLICOLOR_FORCE", "1")
            .arg("run")
            .output()
            .unwrap(),
    );
    assert!(output.stdout.contains(&0x1b));
    let actual = normalize_project_paths(&output.stdout, &fixture);
    assert_eq!(
        actual,
        include_str!("fixtures/stdout/scripts-ansi.txt").replace("<ESC>", "\x1b")
    );
}

#[test]
fn failed_runtime_inspection_preserves_partial_text_without_partial_json() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.root.path().join("rubies")).unwrap();
    let text = fixture
        .command()
        .env("NO_COLOR", "1")
        .args(["info", "runtime"])
        .output()
        .unwrap();
    assert!(!text.status.success());
    assert_eq!(text.stdout, "💎 Ruby Environment Survey\n\n".as_bytes());
    let json = fixture
        .command()
        .env("NO_COLOR", "1")
        .args(["-F", "json", "info", "runtime"])
        .output()
        .unwrap();
    assert_eq!(json.status.code(), text.status.code());
    assert!(json.stdout.is_empty());
    assert_eq!(json.stderr, text.stderr);
}

#[test]
fn empty_scripts_keep_indentation_outside_colored_spans() {
    let fixture = Fixture::new();
    fs::write(fixture.root.path().join("rbproject.toml"), "[scripts]\n").unwrap();
    let output = success(
        fixture
            .command()
            .env_remove("NO_COLOR")
            .env("CLICOLOR_FORCE", "1")
            .arg("run")
            .output()
            .unwrap(),
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        include_str!("fixtures/stdout/scripts-empty-ansi.txt").replace("<ESC>", "\x1b")
    );
}

#[test]
fn root_help_preserves_existing_option_styles_and_spacing() {
    let fixture = Fixture::new();
    for args in [vec!["help"], vec![]] {
        let output = success(
            fixture
                .command()
                .env_remove("NO_COLOR")
                .env("CLICOLOR_FORCE", "1")
                .args(args)
                .output()
                .unwrap(),
        );
        let actual = String::from_utf8(output.stdout).unwrap();
        assert!(actual.contains("--format"));
        let preserved = actual
            .split_inclusive('\n')
            .filter(|line| !line.contains("--format"))
            .collect::<String>();
        assert_eq!(
            preserved,
            include_str!("fixtures/stdout/help-ansi.txt").replace("<ESC>", "\x1b")
        );
    }
}
