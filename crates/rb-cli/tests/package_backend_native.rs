#![cfg(unix)]

#[path = "common/package_backend.rs"]
mod package_backend;
use package_backend::{Fixture, success};
use std::{fs, path::PathBuf};

#[test]
fn native_sync_exec_and_run_use_the_same_bundler_compatible_installation() {
    let fixture = Fixture::new("native");
    let bundle = fixture.root.path().join(".rb/bundle");
    fs::create_dir_all(&bundle).unwrap();
    let _socket = std::os::unix::net::UnixListener::bind(bundle.join("runtime.sock")).unwrap();
    let output = success(fixture.command().arg("sync").output().unwrap());
    assert!(output.stdout.is_empty());
    let report = String::from_utf8_lossy(&output.stderr);
    for phase in [
        "check dependency inputs",
        "check local gem files",
        "record prepared environment",
    ] {
        assert!(report.contains(&format!("started: {phase}")), "{report}");
    }
    assert!(report.contains("Your bundle is ready"));
    assert!(!report.contains("validate prepared environment"));
    assert!(
        !fixture
            .root
            .path()
            .join(".rb/installed-files.json")
            .exists()
    );
    assert!(fixture.root.path().join("Gemfile.lock").is_file());
    assert!(!fixture.root.path().join(".rb/vendor/bundler").exists());
    assert!(!fixture.root.path().join(".rb/cache").exists());
    assert!(
        fixture
            .rubies
            .path()
            .join("cache/rb/source-archives")
            .is_dir()
    );
    let identity_path = fixture.root.path().join(".rb/bundle/fingerprint");
    let identity = fs::read(&identity_path).unwrap();
    let second = success(fixture.command().arg("sync").output().unwrap());
    assert!(String::from_utf8_lossy(&second.stderr).contains("Everything is already in order"));
    assert!(!String::from_utf8_lossy(&second.stderr).contains("resolve dependencies"));
    assert_eq!(fs::read(&identity_path).unwrap(), identity);
    let lock = fs::read(fixture.root.path().join("Gemfile.lock")).unwrap();
    let probe = "require 'backend_fixture'; puts BackendFixture::VALUE; puts Gem.loaded_specs.fetch('backend_fixture').full_gem_path; puts Bundler::VERSION";
    let output = success(
        fixture
            .command()
            .args(["x", "ruby", "-e", probe])
            .output()
            .unwrap(),
    );
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(
        text.contains("backend-installed, Bundler-activated"),
        "{text}"
    );
    assert!(text.contains(".rb/bundle"), "{text}");
    assert_eq!(
        fs::read(fixture.root.path().join("Gemfile.lock")).unwrap(),
        lock
    );
    let output = success(fixture.command().args(["run", "probe"]).output().unwrap());
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("backend-installed, Bundler-activated")
    );
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn explicit_project_file_and_no_bundler_are_respected() {
    let fixture = Fixture::new("native");
    fs::rename(
        fixture.root.path().join("rbproject.toml"),
        fixture.root.path().join("custom.toml"),
    )
    .unwrap();
    let output = success(
        fixture
            .command()
            .args(["-P", "custom.toml", "run", "probe"])
            .output()
            .unwrap(),
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("backend-installed, Bundler-activated")
    );
    let output = success(
        fixture
            .command()
            .args(["-B", "x", "ruby", "-e", "puts defined?(Bundler).inspect"])
            .output()
            .unwrap(),
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "nil");
    assert!(output.stderr.is_empty());
}

#[test]
fn preparation_failure_prevents_execution_without_fallback() {
    let fixture = Fixture::new("native");
    fs::write(
        fixture.root.path().join("Gemfile"),
        "raise 'fixture evaluation failed'\n",
    )
    .unwrap();
    let output = fixture
        .command()
        .args(["x", "ruby", "-e", "puts 'must not execute'"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("fixture evaluation failed"));
    assert!(!fixture.root.path().join(".rb/vendor/bundler").exists());
}

#[test]
fn native_project_without_gemfile_activates_bundler_for_execution() {
    let fixture = Fixture::new("native");
    fs::remove_file(fixture.root.path().join("Gemfile")).unwrap();
    fs::write(
        fixture.root.path().join("rbproject.toml"),
        "[project]\nname='native'\n[dependencies]\nbackend_fixture={path='./local'}\n",
    )
    .unwrap();
    let output = success(
        fixture
            .command()
            .args([
                "x",
                "ruby",
                "-rbackend_fixture",
                "-e",
                "puts BackendFixture::VALUE; puts Bundler::VERSION",
            ])
            .output()
            .unwrap(),
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("backend-installed, Bundler-activated")
    );
    assert!(fixture.root.path().join(".rb/Gemfile").is_file());
    assert!(fixture.root.path().join(".rb/Gemfile.lock").is_file());
}

#[test]
fn relative_bundle_gemfile_and_commands_outside_projects_work() {
    let fixture = Fixture::new("native");
    fs::remove_file(fixture.root.path().join("rbproject.toml")).unwrap();
    fs::create_dir(fixture.root.path().join("config")).unwrap();
    fs::remove_file(fixture.root.path().join("Gemfile")).unwrap();
    fs::write(
        fixture.root.path().join("config/Dependencies"),
        "gem 'backend_fixture', path: '../local'\n",
    )
    .unwrap();
    let output = success(
        fixture
            .command()
            .env("BUNDLE_GEMFILE", "config/Dependencies")
            .args([
                "x",
                "ruby",
                "-rbackend_fixture",
                "-e",
                "puts BackendFixture::VALUE",
            ])
            .output()
            .unwrap(),
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("backend-installed, Bundler-activated")
    );
    let output = success(
        fixture
            .command()
            .args(["x", "ruby", "-e", "puts defined?(Bundler).inspect"])
            .output()
            .unwrap(),
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "nil");
    assert!(output.stderr.is_empty());
}

#[test]
fn ready_commands_skip_ruby_helpers_and_dependency_changes_or_missing_installations_trigger_preparation()
 {
    let fixture = Fixture::new("native");
    let calls = fixture.instrument_ruby();
    let _socket =
        std::os::unix::net::UnixListener::bind(fixture.root.path().join("application.sock"))
            .unwrap();
    success(fixture.command().arg("sync").output().unwrap());
    assert!(fixture.root.path().join(".rb/prepared.json").is_file());
    fs::write(&calls, "").unwrap();
    let probe = [
        "x",
        "ruby",
        "-rbackend_fixture",
        "-e",
        "puts Gem.loaded_specs.fetch('backend_fixture').full_gem_path",
    ];
    let output = success(fixture.command().args(probe).output().unwrap());
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let started = fs::read_to_string(&calls).unwrap();
    assert!(!started.contains("-rjson"), "{started}");
    assert!(!started.contains("rb-evaluate-gemfile"), "{started}");
    for directory in ["log", "tmp"] {
        fs::create_dir(fixture.root.path().join(directory)).unwrap();
        fs::write(
            fixture.root.path().join(directory).join("generated"),
            "application output",
        )
        .unwrap();
    }
    fs::create_dir(fixture.root.path().join("app")).unwrap();
    fs::write(
        fixture.root.path().join("app/model.rb"),
        "unrelated application code",
    )
    .unwrap();
    let quiet = success(
        fixture
            .command()
            .env("UNRELATED_PROCESS_VALUE", "changed")
            .args(probe)
            .output()
            .unwrap(),
    );
    assert!(
        quiet.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&quiet.stderr)
    );
    let snapshot = PathBuf::from(String::from_utf8(output.stdout).unwrap().trim());
    assert!(snapshot.starts_with(fixture.root.path().canonicalize().unwrap()));
    let home =
        rb_packages::BundleLayout::new(fixture.root.path(), "ruby", &fixture.version).gem_home;
    let removed = home.with_extension("removed");
    fs::rename(&home, &removed).unwrap();
    let output = success(fixture.command().args(probe).output().unwrap());
    assert!(home.is_dir());
    fs::remove_dir_all(removed).unwrap();
    assert!(String::from_utf8_lossy(&output.stderr).contains("Preparing your bundle"));
    fs::write(
        fixture.root.path().join("local/lib/backend_fixture.rb"),
        "module BackendFixture; VALUE='updated source'; end\n",
    )
    .unwrap();
    let output = success(fixture.command().args(["run", "probe"]).output().unwrap());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        "updated source"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("Preparing your bundle"));
    fs::write(fixture.root.path().join(".rb/prepared.json"), "corrupt").unwrap();
    let output = success(fixture.command().args(probe).output().unwrap());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Preparing your bundle"));
    let output = success(
        fixture
            .command()
            .env("BUNDLE_WITHOUT", "test")
            .args(probe)
            .output()
            .unwrap(),
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("Preparing your bundle"));
}

#[test]
fn external_path_source_edits_invalidate_the_ready_environment() {
    let fixture = Fixture::new("native");
    let external = tempfile::tempdir().unwrap();
    let source = external.path().join("local");
    fs::rename(fixture.root.path().join("local"), &source).unwrap();
    fs::write(
        fixture.root.path().join("Gemfile"),
        format!(
            "gem 'backend_fixture', path: {:?}\n",
            source.to_str().unwrap()
        ),
    )
    .unwrap();
    success(fixture.command().arg("sync").output().unwrap());
    let output = success(fixture.command().args(["run", "probe"]).output().unwrap());
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::write(
        source.join("lib/backend_fixture.rb"),
        "module BackendFixture; VALUE='external source changed'; end\n",
    )
    .unwrap();
    let output = success(fixture.command().args(["run", "probe"]).output().unwrap());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        "external source changed"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("Preparing your bundle"));
}

#[test]
fn native_completion_and_info_share_the_installed_environment() {
    let fixture = Fixture::new("native");
    let calls = fixture.instrument_ruby();
    success(fixture.command().arg("sync").output().unwrap());
    let abi = fixture
        .version
        .split('.')
        .take(2)
        .collect::<Vec<_>>()
        .join(".")
        + ".0";
    let home = fixture
        .root
        .path()
        .canonicalize()
        .unwrap()
        .join(".rb/bundle/ruby")
        .join(abi);
    let bin = home.join("bin");
    fs::create_dir_all(&bin).unwrap();
    fs::write(bin.join("native_fixture"), "#!/usr/bin/env ruby\n").unwrap();
    std::os::unix::fs::symlink("native_fixture", bin.join("native_alias")).unwrap();
    fs::write(&calls, "").unwrap();
    for alias in ["x", "exec"] {
        let line = format!("rb {alias} native_");
        let output = success(
            fixture
                .command()
                .args(["__bash_complete", &line, &line.len().to_string()])
                .output()
                .unwrap(),
        );
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(text.lines().any(|line| line == "native_fixture"), "{text}");
        assert!(text.lines().any(|line| line == "native_alias"), "{text}");
        assert!(output.stderr.is_empty());
    }
    assert_eq!(fs::read_to_string(&calls).unwrap(), "");
    fs::remove_file(bin.join("native_alias")).unwrap();
    fs::remove_file(bin.join("native_fixture")).unwrap();
    let output = success(fixture.command().args(["info", "env"]).output().unwrap());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Native Package Environment"), "{text}");
    assert!(text.contains(home.to_str().unwrap()), "{text}");
    assert!(!text.contains(".rb/vendor/bundler"), "{text}");
    assert!(!text.contains("Out of sync"), "{text}");
    let output = success(
        fixture
            .command()
            .args(["-B", "__bash_complete", "rb x native_", "12"])
            .output()
            .unwrap(),
    );
    assert!(output.stdout.is_empty());
}

#[test]
fn declared_external_gemfile_inputs_allow_reuse_and_invalidate_on_changes() {
    let fixture = Fixture::new("native");
    let shared = tempfile::tempdir().unwrap();
    let dependency = shared.path().join("ruby.rb");
    fs::write(&dependency, "ruby RUBY_VERSION\n").unwrap();
    let gemfile = fixture.root.path().join("Gemfile");
    let original = fs::read_to_string(&gemfile).unwrap();
    fs::write(
        &gemfile,
        format!(
            "eval_gemfile {:?}\n{original}",
            dependency.to_str().unwrap()
        ),
    )
    .unwrap();
    success(fixture.command().arg("sync").output().unwrap());
    assert!(
        fixture.root.path().join(".rb/prepared.json").is_file(),
        "Declared external dependency inputs must not disable reuse"
    );
    let probe = ["x", "ruby", "-e", "puts 'ready'"];
    let output = success(fixture.command().args(probe).output().unwrap());
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "ready");
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    fs::write(&dependency, "ruby RUBY_VERSION.strip\n").unwrap();
    let output = success(fixture.command().args(probe).output().unwrap());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Preparing your bundle"));
    let output = success(fixture.command().args(probe).output().unwrap());
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn ready_state_survives_launcher_replacement_and_unread_path_changes() {
    let mut fixture = Fixture::new("native");
    success(fixture.command().arg("sync").output().unwrap());
    let probe = ["x", "ruby", "-e", "puts 'ready'"];
    let path = std::env::join_paths(
        std::env::split_paths(&std::env::var_os("PATH").unwrap())
            .chain(std::iter::once(fixture.rubies.path().join("unrelated-bin"))),
    )
    .unwrap();
    let changed_path = success(
        fixture
            .command()
            .env("PATH", path)
            .args(probe)
            .output()
            .unwrap(),
    );
    let binary = fixture.rubies.path().join("rebuilt-rb");
    fs::copy(&fixture.binary, &binary).unwrap();
    fixture.binary = binary;
    let changed_binary = success(fixture.command().args(probe).output().unwrap());
    fs::File::open(&fixture.binary)
        .unwrap()
        .set_modified(std::time::SystemTime::now() + std::time::Duration::from_secs(1))
        .unwrap();
    let rebuilt_binary = success(fixture.command().args(probe).output().unwrap());
    for output in [changed_path, changed_binary, rebuilt_binary] {
        assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "ready");
        assert!(
            output.stderr.is_empty(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn gemfile_path_reads_still_invalidate_prepared_state() {
    let fixture = Fixture::new("native");
    let gemfile = fixture.root.path().join("Gemfile");
    let original = fs::read_to_string(&gemfile).unwrap();
    fs::write(&gemfile, format!("ENV.fetch('PATH')\n{original}")).unwrap();
    success(fixture.command().arg("sync").output().unwrap());
    let path = std::env::join_paths(
        std::env::split_paths(&std::env::var_os("PATH").unwrap())
            .chain(std::iter::once(fixture.rubies.path().join("new-tools"))),
    )
    .unwrap();
    let probe = ["x", "ruby", "-e", "puts 'ready'"];
    let first = success(
        fixture
            .command()
            .env("PATH", &path)
            .args(probe)
            .output()
            .unwrap(),
    );
    assert!(String::from_utf8_lossy(&first.stderr).contains("Preparing your bundle"));
    let second = success(
        fixture
            .command()
            .env("PATH", path)
            .args(probe)
            .output()
            .unwrap(),
    );
    assert!(
        second.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );
}

#[test]
fn missing_prepared_state_does_not_stage_an_unchanged_installation() {
    let fixture = Fixture::new("native");
    success(fixture.command().arg("sync").output().unwrap());
    let home = fixture.bin_dir().parent().unwrap().to_owned();
    let _socket = std::os::unix::net::UnixListener::bind(home.join("live.sock")).unwrap();
    fs::remove_file(fixture.root.path().join(".rb/prepared.json")).unwrap();
    let output = success(fixture.command().args(["run", "probe"]).output().unwrap());
    assert!(String::from_utf8_lossy(&output.stdout).contains("backend-installed"));
    let report = String::from_utf8_lossy(&output.stderr);
    assert!(
        report.contains("Everything is already in order"),
        "{report}"
    );
    assert!(!report.contains("prepare installation"), "{report}");
    let ready = success(fixture.command().args(["run", "probe"]).output().unwrap());
    assert!(
        ready.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&ready.stderr)
    );
}

#[test]
fn git_worktrees_reuse_global_compilation_during_automatic_preparation() {
    let fixture = Fixture::new("native");
    let counter = fixture.rubies.path().join("build-count");
    let extconf = fixture
        .root
        .path()
        .join("local/ext/backend_fixture/extconf.rb");
    let script = fs::read_to_string(&extconf).unwrap();
    fs::write(
        &extconf,
        format!(
            "File.open(ENV.fetch('BUILD_COUNTER'), 'a') {{ |file| file.puts 'build' }}\n{script}"
        ),
    )
    .unwrap();
    success(
        fixture
            .command()
            .env("BUILD_COUNTER", &counter)
            .arg("sync")
            .output()
            .unwrap(),
    );
    assert_eq!(fs::read_to_string(&counter).unwrap(), "build\n");
    let git = |arguments: &[&str]| {
        success(
            std::process::Command::new("git")
                .current_dir(fixture.root.path())
                .args(arguments)
                .output()
                .unwrap(),
        );
    };
    git(&["init", "--initial-branch=main"]);
    git(&["add", "Gemfile", "Gemfile.lock", "rbproject.toml", "local"]);
    git(&[
        "-c",
        "user.name=Fixture",
        "-c",
        "user.email=fixture@example.test",
        "commit",
        "-m",
        "Fixture",
    ]);
    for name in ["feature-one", "feature two"] {
        let worktree = fixture.root.path().join(".trees").join(name);
        git(&[
            "worktree",
            "add",
            "--detach",
            worktree.to_str().unwrap(),
            "HEAD",
        ]);
        let output = success(
            fixture
                .command_in(&worktree, fixture.rubies.path())
                .env("BUILD_COUNTER", &counter)
                .args([
                    "x",
                    "ruby",
                    "-rbackend_fixture",
                    "-rbackend_fixture_native",
                    "-e",
                    "puts BackendFixture::VALUE; puts Dir.pwd",
                ])
                .output()
                .unwrap(),
        );
        let text = String::from_utf8(output.stdout).unwrap();
        assert!(
            text.contains("backend-installed, Bundler-activated"),
            "{text}"
        );
        assert!(
            text.contains(worktree.canonicalize().unwrap().to_str().unwrap()),
            "{text}"
        );
        assert_eq!(fs::read_to_string(&counter).unwrap(), "build\n");
        assert_eq!(
            fs::read(worktree.join("Gemfile.lock")).unwrap(),
            fs::read(fixture.root.path().join("Gemfile.lock")).unwrap()
        );
        assert!(String::from_utf8_lossy(&output.stderr).contains("reuse 1/1 backend_fixture"));
    }
}
