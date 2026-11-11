#![cfg(unix)]

#[path = "common/package_backend.rs"]
mod package_backend;
use package_backend::{Fixture, success};
use std::fs;

const PROBE: &[&str] = &[
    "x",
    "ruby",
    "-rbackend_fixture",
    "-rbackend_fixture_native",
    "-e",
    "puts BackendFixture::VALUE",
];

#[test]
fn sync_exec_run_and_info_use_the_configured_installation() {
    let fixture = Fixture::configured();
    let first = success(
        fixture
            .command()
            .args(["-F", "json", "sync"])
            .output()
            .unwrap(),
    );
    let report: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(report["schema_version"], 1);
    assert_eq!(report["command"], "sync");
    assert_eq!(report["data"]["changed"], true);
    assert!(String::from_utf8_lossy(&first.stderr).contains("Your bundle is ready"));
    assert!(fixture.root.path().join("Gemfile.lock").is_file());
    let lock = fs::read(fixture.root.path().join("Gemfile.lock")).unwrap();
    let second = success(fixture.command().arg("sync").output().unwrap());
    assert!(String::from_utf8_lossy(&second.stderr).contains("Everything is already in order"));
    assert_eq!(
        fs::read(fixture.root.path().join("Gemfile.lock")).unwrap(),
        lock
    );
    for args in [PROBE, &["run", "probe"], &["r", "probe"]] {
        let output = success(
            fixture
                .command()
                .env("RB_FORMAT", "json")
                .args(args)
                .output()
                .unwrap(),
        );
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            "backend-installed, Bundler-activated"
        );
        assert!(
            output.stderr.is_empty(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let output = success(fixture.command().args(["info", "env"]).output().unwrap());
    let text = String::from_utf8(output.stdout).unwrap();
    for (label, value) in [("Configured", "Yes"), ("Synchronized", "Synchronized")] {
        assert!(
            text.lines().any(|line| line
                .split_once(':')
                .is_some_and(|(key, found)| key.trim() == label && found.contains(value))),
            "{text}"
        );
    }
    assert!(text.contains(fixture.bin_dir().to_str().unwrap()), "{text}");
    assert!(!text.contains("Out of sync"), "{text}");
    let output = success(
        fixture
            .command()
            .args(["info", "runtime"])
            .output()
            .unwrap(),
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains(&fixture.version));
    let output = success(
        fixture
            .command()
            .args(["info", "project"])
            .output()
            .unwrap(),
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("backend-fixture"));
}

#[test]
fn exec_and_run_prepare_automatically_and_no_bundler_bypasses_preparation() {
    for args in [PROBE, &["run", "probe"]] {
        let fixture = Fixture::configured();
        let output = success(fixture.command().args(args).output().unwrap());
        assert_eq!(
            String::from_utf8(output.stdout).unwrap().trim(),
            "backend-installed, Bundler-activated"
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("Your bundle is ready"));
    }
    let fixture = Fixture::configured();
    let output = success(
        fixture
            .command()
            .args(["-B", "x", "ruby", "-e", "puts defined?(Bundler).inspect"])
            .output()
            .unwrap(),
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "nil");
    assert!(output.stderr.is_empty());
    assert!(!fixture.root.path().join(".rb").exists());
}

#[test]
fn completion_uses_execution_paths_without_starting_ruby() {
    let fixture = Fixture::configured();
    let calls = fixture.instrument_ruby();
    success(fixture.command().arg("sync").output().unwrap());
    let bin = fixture.bin_dir();
    fs::create_dir_all(&bin).unwrap();
    fs::write(bin.join("contract_tool"), "#!/usr/bin/env ruby\n").unwrap();
    std::os::unix::fs::symlink("contract_tool", bin.join("contract_alias")).unwrap();
    fs::write(&calls, "").unwrap();
    for alias in ["x", "exec"] {
        let line = format!("rb {alias} contract_");
        let output = success(
            fixture
                .command()
                .args(["__bash_complete", &line, &line.len().to_string()])
                .output()
                .unwrap(),
        );
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            "contract_alias\ncontract_tool\n"
        );
        assert!(output.stderr.is_empty());
    }
    let output = success(
        fixture
            .command()
            .args(["__bash_complete", "rb x backend_c", "14"])
            .output()
            .unwrap(),
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        "backend_context"
    );
    assert_eq!(fs::read_to_string(&calls).unwrap(), "");
    let output = success(
        fixture
            .command()
            .args(["-B", "__bash_complete", "rb x contract_", "14"])
            .output()
            .unwrap(),
    );
    assert!(output.stdout.is_empty());
    let output = success(
        fixture
            .command()
            .args(["x", "ruby", "-e", "puts ENV.fetch('PATH')"])
            .output()
            .unwrap(),
    );
    assert!(
        std::env::split_paths(String::from_utf8(output.stdout).unwrap().trim())
            .any(|path| path == bin)
    );
}

#[test]
fn selected_project_and_symlinked_worktree_keep_the_same_backend() {
    let fixture = Fixture::configured();
    fs::rename(
        fixture.root.path().join("rbproject.toml"),
        fixture.root.path().join("custom.toml"),
    )
    .unwrap();
    let nested = fixture.root.path().join(".trees/feature/app");
    fs::create_dir_all(&nested).unwrap();
    let alias = fixture.rubies.path().join("worktree-alias");
    std::os::unix::fs::symlink(&nested, &alias).unwrap();
    let output = success(
        fixture
            .command_in(&alias, fixture.rubies.path())
            .args([
                "-P",
                fixture.root.path().join("custom.toml").to_str().unwrap(),
                "run",
                "probe",
            ])
            .output()
            .unwrap(),
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        "backend-installed, Bundler-activated"
    );
    assert!(!nested.join(".rb").exists());
}

#[test]
fn dependency_changes_reconcile_lockfiles_and_failures_prevent_execution() {
    let fixture = Fixture::configured();
    success(fixture.command().arg("sync").output().unwrap());
    fs::write(
        fixture.root.path().join("Gemfile"),
        "source 'https://rubygems.org'\n",
    )
    .unwrap();
    success(
        fixture
            .command()
            .args(["x", "ruby", "-e", "puts 'ready'"])
            .output()
            .unwrap(),
    );
    let lock = fs::read_to_string(fixture.root.path().join("Gemfile.lock")).unwrap();
    assert!(!lock.contains("backend_fixture"), "{lock}");
    fs::write(
        fixture.root.path().join("Gemfile"),
        "raise 'contract preparation failed'\n",
    )
    .unwrap();
    for args in [PROBE, &["run", "probe"]] {
        let output = fixture.command().args(args).output().unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(String::from_utf8_lossy(&output.stderr).contains("contract preparation failed"));
    }
}

#[test]
fn missing_ruby_completion_keeps_command_and_flag_suggestions() {
    let fixture = Fixture::configured();
    let missing = fixture.rubies.path().join("missing");
    for (line, expected) in [("rb sy", "sync"), ("rb --pack", "--package-backend")] {
        let output = success(
            fixture
                .command_in(fixture.root.path(), &missing)
                .args(["__bash_complete", line, &line.len().to_string()])
                .output()
                .unwrap(),
        );
        assert!(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .any(|line| line == expected)
        );
    }
    assert!(!missing.exists());
}

#[test]
fn generated_bash_shim_uses_typed_global_options_with_the_same_backend() {
    let fixture = Fixture::selected();
    let config = fixture.rubies.path().join("configuration with spaces.toml");
    fs::write(&config, "package-backend = 'bundler'\n").unwrap();
    let project = fixture.root.path().join("selected.toml");
    fs::write(&project, "[scripts]\nselected_probe='ruby -e 1'\n").unwrap();
    let bin = fixture.bin_dir();
    fs::create_dir_all(&bin).unwrap();
    fs::write(bin.join("shim_tool"), "#!/usr/bin/env ruby\n").unwrap();
    let shim = success(
        fixture
            .command()
            .args(["shell-integration", "bash"])
            .output()
            .unwrap(),
    );
    let complete = |backend_option: &[String], suffix: &[&str]| {
        let output = std::process::Command::new("bash")
            .args([
                "--noprofile",
                "--norc",
                "-c",
                include_str!("fixtures/package-backend/complete.bash"),
                "completion-test",
            ])
            .arg(env!("CARGO_BIN_EXE_rb"))
            .arg(String::from_utf8_lossy(&shim.stdout).as_ref())
            .args([
                "--config",
                config.to_str().unwrap(),
                "-R",
                fixture.rubies.path().to_str().unwrap(),
                "-r",
                &fixture.version,
                "-C",
                fixture.root.path().to_str().unwrap(),
                "-P",
                project.to_str().unwrap(),
            ])
            .args(backend_option)
            .args(suffix)
            .env_remove("RB_PACKAGE_BACKEND")
            .env_remove("RB_NO_BUNDLER")
            .output()
            .unwrap();
        String::from_utf8(success(output).stdout)
            .unwrap()
            .trim()
            .to_owned()
    };
    for option in [
        vec!["--package-backend".into(), fixture.backend().into()],
        vec![format!("--package-backend={}", fixture.backend())],
    ] {
        assert_eq!(complete(&option, &["x", "shim_"]), "shim_tool");
        assert_eq!(complete(&option, &["-B", "x", "shim_"]), "");
        assert_eq!(complete(&option, &["-vB", "x", "shim_"]), "");
        assert_eq!(complete(&option, &["r", "selected_"]), "selected_probe");
    }
    assert_eq!(
        complete(&[], &["x", "--package-backend", fixture.backend(), "shim_"]),
        "shim_tool"
    );
    assert_eq!(
        complete(
            &[format!("--package-backend={}", fixture.backend())],
            &["x", "ruby", "--pack"]
        ),
        ""
    );
}

#[test]
fn execution_preserves_worktree_and_nested_working_directories() {
    let fixture = Fixture::configured();
    let worktree = fixture.root.path().join(".trees/feature");
    let nested = worktree.join("nested");
    fs::create_dir_all(&nested).unwrap();
    fs::copy(
        fixture.root.path().join("Gemfile"),
        worktree.join("Gemfile"),
    )
    .unwrap();
    let probe = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/package-backend/context_probe.rb");
    let project = worktree.join("rbproject.toml");
    fs::write(
        &project,
        format!("[scripts]\ncontext = 'ruby \"{}\"'\n", probe.display()),
    )
    .unwrap();
    fs::write(fixture.root.path().join("parent_only.rb"), "").unwrap();
    fs::write(worktree.join("worktree_only.rb"), "").unwrap();
    fs::write(nested.join("nested_only.rb"), "").unwrap();
    let alias = fixture.rubies.path().join("worktree alias");
    std::os::unix::fs::symlink(&worktree, &alias).unwrap();
    for (directory, file) in [
        (&worktree, "worktree_only.rb"),
        (&nested, "nested_only.rb"),
        (&alias, "worktree_only.rb"),
    ] {
        for args in [
            vec!["x", "ruby", probe.to_str().unwrap()],
            vec!["x", "backend_context"],
            vec!["-P", project.to_str().unwrap(), "run", "context"],
        ] {
            let output = success(
                fixture
                    .command_in(directory, fixture.rubies.path())
                    .args(args)
                    .output()
                    .unwrap(),
            );
            let text = String::from_utf8(output.stdout).unwrap();
            assert_eq!(
                text.lines().collect::<Vec<_>>(),
                [
                    directory.canonicalize().unwrap().to_str().unwrap(),
                    worktree.canonicalize().unwrap().to_str().unwrap(),
                    file
                ]
            );
        }
    }
}

#[test]
fn completion_and_execution_apply_configured_relative_working_directory_once() {
    let fixture = Fixture::selected();
    let child = fixture.root.path().join("child");
    fs::create_dir(&child).unwrap();
    fs::write(child.join("Gemfile"), "source 'https://rubygems.org'\n").unwrap();
    let relative = fixture
        .bin_dir()
        .strip_prefix(fixture.root.path().canonicalize().unwrap())
        .unwrap()
        .to_owned();
    let bin = child.join(relative);
    fs::create_dir_all(&bin).unwrap();
    fs::write(bin.join("context_tool"), "#!/usr/bin/env ruby\n").unwrap();
    let config = fixture.rubies.path().join("relative.toml");
    fs::write(
        &config,
        format!(
            "package-backend = '{}'\nrubies-dir = {:?}\nruby-version = '{}'\nwork-dir = 'child'\n",
            fixture.backend(),
            fixture.rubies.path().to_str().unwrap(),
            fixture.version
        ),
    )
    .unwrap();
    let output = success(
        fixture
            .bare_command()
            .env("RB_CONFIG", &config)
            .args(["-B", "x", "ruby", "-e", "puts Dir.pwd"])
            .output()
            .unwrap(),
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        child.canonicalize().unwrap().to_str().unwrap()
    );
    let line = format!("rb --config {:?} x context_", config.to_str().unwrap());
    let output = success(
        fixture
            .bare_command()
            .env("RB_CONFIG", &config)
            .args(["__bash_complete", &line, &line.len().to_string()])
            .output()
            .unwrap(),
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        "context_tool"
    );
}
