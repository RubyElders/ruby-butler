#![cfg(unix)]

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

struct Fixture {
    root: tempfile::TempDir,
    rubies: tempfile::TempDir,
    version: String,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let ruby = std::env::var_os("RB_GEM_BUILD_RUBY")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                which::which("ruby").expect("Ruby is required for native package integration tests")
            });
        let output = Command::new(&ruby)
            .args([
                "-rrbconfig",
                "-rbundler",
                "-e",
                "puts RbConfig::CONFIG['prefix']; puts RUBY_VERSION",
            ])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "A Ruby with Bundler is required: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8(output.stdout).unwrap();
        let mut lines = text.lines();
        let prefix = lines.next().unwrap();
        let version = lines.next().unwrap().to_owned();
        let rubies = tempfile::tempdir().unwrap();
        let installation = rubies.path().join(format!("ruby-{version}"));
        fs::create_dir(&installation).unwrap();
        std::os::unix::fs::symlink(Path::new(prefix).join("bin"), installation.join("bin"))
            .unwrap();
        for file in [
            "Gemfile",
            "rbproject.toml",
            "local/backend_fixture.gemspec",
            "local/lib/backend_fixture.rb",
        ] {
            let source_file = if file == "rbproject.toml" {
                "project.toml"
            } else {
                file
            };
            let source = Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/package-backend")
                .join(source_file);
            let destination = root.path().join(file);
            fs::create_dir_all(destination.parent().unwrap()).unwrap();
            fs::copy(source, destination).unwrap();
        }
        Self {
            root,
            rubies,
            version,
        }
    }

    fn instrument_ruby(&self) -> PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let bin = self
            .rubies
            .path()
            .join(format!("ruby-{}", self.version))
            .join("bin");
        let original = fs::read_link(&bin).unwrap();
        fs::remove_file(&bin).unwrap();
        fs::create_dir(&bin).unwrap();
        let calls = self.rubies.path().join("ruby-calls");
        let quote = |path: &Path| format!("'{}'", path.to_str().unwrap().replace('\'', "'\\''"));
        for name in ["ruby", "bundle"] {
            let log = if name == "ruby" {
                format!("printf '%s\\n' \"$*\" >> {}\n", quote(&calls))
            } else {
                String::new()
            };
            let path = bin.join(name);
            fs::write(
                &path,
                format!(
                    "#!/bin/sh\n{log}exec {} \"$@\"\n",
                    quote(&original.join(name))
                ),
            )
            .unwrap();
            fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
        }
        calls
    }

    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_rb"));
        command.args([
            "-R",
            self.rubies.path().to_str().unwrap(),
            "-r",
            &self.version,
            "-C",
            self.root.path().to_str().unwrap(),
            "--package-backend",
            "native",
        ]);
        for key in [
            "RB_CONFIG",
            "RB_PACKAGE_BACKEND",
            "RB_NO_BUNDLER",
            "BUNDLE_GEMFILE",
            "RUBYOPT",
            "RUBYLIB",
            "BUNDLE_FROZEN",
            "BUNDLE_DEPLOYMENT",
            "BUNDLE_WITH",
            "BUNDLE_WITHOUT",
        ] {
            command.env_remove(key);
        }
        command
    }
}

fn success(output: Output) -> Output {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

#[test]
fn native_sync_exec_and_run_use_the_same_bundler_compatible_installation() {
    let fixture = Fixture::new();
    let output = success(fixture.command().arg("sync").output().unwrap());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("Your bundle is ready"));
    assert!(fixture.root.path().join("Gemfile.lock").is_file());
    assert!(!fixture.root.path().join(".rb/vendor/bundler").exists());
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
        text.contains("native-installed, Bundler-activated"),
        "{text}"
    );
    assert!(text.contains(".rb/bundle"), "{text}");
    assert_eq!(
        fs::read(fixture.root.path().join("Gemfile.lock")).unwrap(),
        lock
    );
    let output = success(fixture.command().args(["run", "probe"]).output().unwrap());
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("native-installed, Bundler-activated")
    );
    assert!(
        output.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn explicit_project_file_and_no_bundler_are_respected() {
    let fixture = Fixture::new();
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
        String::from_utf8_lossy(&output.stdout).contains("native-installed, Bundler-activated")
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
    let fixture = Fixture::new();
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
    let fixture = Fixture::new();
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
        String::from_utf8_lossy(&output.stdout).contains("native-installed, Bundler-activated")
    );
    assert!(fixture.root.path().join(".rb/Gemfile").is_file());
    assert!(fixture.root.path().join(".rb/Gemfile.lock").is_file());
}

#[test]
fn relative_bundle_gemfile_and_commands_outside_projects_work() {
    let fixture = Fixture::new();
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
        String::from_utf8_lossy(&output.stdout).contains("native-installed, Bundler-activated")
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
fn ready_commands_skip_ruby_helpers_and_edits_or_missing_files_trigger_preparation() {
    let fixture = Fixture::new();
    let calls = fixture.instrument_ruby();
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
    let quiet = success(fixture.command().args(probe).output().unwrap());
    assert!(
        quiet.stderr.is_empty(),
        "{}",
        String::from_utf8_lossy(&quiet.stderr)
    );
    let snapshot = PathBuf::from(String::from_utf8(output.stdout).unwrap().trim());
    assert!(snapshot.starts_with(fixture.root.path().canonicalize().unwrap()));
    let library = snapshot.join("lib/backend_fixture.rb");
    fs::remove_file(&library).unwrap();
    let output = success(fixture.command().args(probe).output().unwrap());
    assert!(library.is_file());
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
    let fixture = Fixture::new();
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
