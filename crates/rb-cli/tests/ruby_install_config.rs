use clap::Parser;
use rb_cli::{
    Cli,
    config::{RbConfig, TrackedConfig},
};
use std::process::Command;

#[test]
fn parses_paired_flags_in_global_positions() {
    for args in [
        vec!["rb", "--auto-install-ruby", "sync"],
        vec!["rb", "sync", "--auto-install-ruby"],
        vec![
            "rb",
            "--no-auto-install-ruby",
            "--auto-install-ruby",
            "sync",
        ],
    ] {
        let cli = Cli::try_parse_from(args).unwrap();
        let config = TrackedConfig::from_merged(&cli.config, &RbConfig::default());
        assert!(*config.auto_install_ruby.get());
    }
    for args in [
        vec!["rb", "--no-auto-install-ruby", "sync"],
        vec!["rb", "sync", "--no-auto-install-ruby"],
        vec!["rb", "--auto-install-ruby=false", "sync"],
        vec![
            "rb",
            "--auto-install-ruby",
            "--no-auto-install-ruby",
            "sync",
        ],
    ] {
        let cli = Cli::try_parse_from(args).unwrap();
        let config = TrackedConfig::from_merged(&cli.config, &RbConfig::default());
        assert!(!*config.auto_install_ruby.get());
    }
}

#[test]
fn file_environment_and_cli_use_the_same_setting() {
    let root = tempfile::tempdir().unwrap();
    for (filename, contents) in [
        ("config.toml", "auto-install-ruby = false\n"),
        ("config.kdl", "auto-install-ruby #false\n"),
    ] {
        let file = root.path().join(filename);
        std::fs::write(&file, contents).unwrap();
        for (environment, args, expected, source) in [
            (None, vec![], false, "config file"),
            (Some("true"), vec![], true, "environment"),
            (
                Some("invalid"),
                vec!["--no-auto-install-ruby"],
                false,
                "CLI argument",
            ),
            (
                Some("invalid"),
                vec!["--auto-install-ruby"],
                true,
                "CLI argument",
            ),
            (
                Some("false"),
                vec!["--auto-install-ruby"],
                true,
                "CLI argument",
            ),
            (
                Some("true"),
                vec!["--no-auto-install-ruby"],
                false,
                "CLI argument",
            ),
        ] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_rb"));
            command
                .args(["-c", file.to_str().unwrap()])
                .args(args)
                .args(["info", "config"])
                .env_remove("RB_AUTO_INSTALL_RUBY");
            if let Some(value) = environment {
                command.env("RB_AUTO_INSTALL_RUBY", value);
            }
            let output = command.output().unwrap();
            assert!(output.status.success(), "{:?}", output);
            let text = String::from_utf8(output.stdout).unwrap();
            assert!(
                text.contains(&format!(
                    "Auto-install Ruby: {expected}\n  Source: {source}"
                )),
                "{text}"
            );
        }
    }
}

#[test]
fn enabled_by_default_and_invalid_environment_value_is_rejected() {
    let root = tempfile::tempdir().unwrap();
    let file = root.path().join("empty.toml");
    std::fs::write(&file, "").unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_rb"));
    command
        .args(["-c", file.to_str().unwrap(), "info", "config"])
        .env_remove("RB_AUTO_INSTALL_RUBY");
    let output = command.output().unwrap();
    assert!(output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stdout)
            .contains("Auto-install Ruby: true\n  Source: default")
    );
    let output = command
        .env("RB_AUTO_INSTALL_RUBY", "maybe")
        .output()
        .unwrap();
    assert!(!output.status.success());
}
