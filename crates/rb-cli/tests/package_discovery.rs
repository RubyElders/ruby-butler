#![cfg(unix)]

use rb_tests::RubySandbox;
use std::{fs, path::Path, process::Command};

fn complete(rubies: &Path, directory: &Path, arguments: &[&str], gemfile: Option<&str>) -> String {
    let mut command = Command::new(env!("CARGO_BIN_EXE_rb"));
    command
        .current_dir(directory)
        .args([
            "-R",
            rubies.to_str().unwrap(),
            "-r",
            "3.4.8",
            "--package-backend",
            "native",
        ])
        .args(arguments)
        .args(["__bash_complete", "rb x native_", "12"])
        .env_remove("BUNDLE_GEMFILE")
        .env_remove("RB_CONFIG")
        .env_remove("RB_NO_BUNDLER");
    if let Some(file) = gemfile {
        command.env("BUNDLE_GEMFILE", file);
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    String::from_utf8(output.stdout).unwrap()
}

fn installation(root: &Path, name: &str) {
    let bin = root.join(".rb/bundle/ruby/3.4.0/bin");
    fs::create_dir_all(&bin).unwrap();
    fs::write(bin.join(name), "#!/usr/bin/env ruby\n").unwrap();
}

#[test]
fn completion_discovers_worktrees_selected_projects_and_environment_gemfiles_without_preparing() {
    let rubies = RubySandbox::new().unwrap();
    rubies.add_ruby_dir("3.4.8").unwrap();
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("Gemfile"), "raise 'must not evaluate'\n").unwrap();
    installation(root.path(), "native_parent");
    let worktree = root.path().join(".trees/feature");
    let nested = worktree.join("app/models");
    fs::create_dir_all(&nested).unwrap();
    fs::write(worktree.join("Gemfile"), "raise 'must not evaluate'\n").unwrap();
    fs::create_dir(worktree.join(".rb")).unwrap();
    fs::write(worktree.join(".rb/prepared.json"), "invalid cache").unwrap();
    installation(&worktree, "native_worktree");
    let alias = root.path().join("alias");
    std::os::unix::fs::symlink(&nested, &alias).unwrap();
    let text = complete(rubies.root(), &alias, &[], None);
    assert_eq!(text.trim(), "native_worktree");
    assert_eq!(
        fs::read_to_string(worktree.join(".rb/prepared.json")).unwrap(),
        "invalid cache"
    );
    assert!(!worktree.join(".rb/prepare.lock").exists());
    assert!(complete(rubies.root(), &alias, &["-B"], None).is_empty());
    let selected = root.path().join("selected");
    fs::create_dir(&selected).unwrap();
    fs::write(selected.join("custom.toml"), "[dependencies]\n").unwrap();
    fs::write(selected.join("Dependencies"), "raise 'must not evaluate'\n").unwrap();
    installation(&selected, "native_selected");
    assert_eq!(
        complete(
            rubies.root(),
            root.path(),
            &["-P", "selected/custom.toml"],
            None
        )
        .trim(),
        "native_selected"
    );
    assert_eq!(
        complete(
            rubies.root(),
            root.path(),
            &[],
            Some("selected/Dependencies")
        )
        .trim(),
        "native_selected"
    );
    assert!(!selected.join(".rb/prepared.json").exists());
}
