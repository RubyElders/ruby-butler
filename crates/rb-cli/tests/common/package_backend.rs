#![allow(dead_code)]

use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

#[path = "registry.rs"]
mod registry;

pub struct Fixture {
    pub binary: PathBuf,
    pub root: tempfile::TempDir,
    pub rubies: tempfile::TempDir,
    pub version: String,
    backend: String,
    registry: Option<registry::Registry>,
}

impl Fixture {
    pub fn backend(&self) -> &str {
        &self.backend
    }

    pub fn selected() -> Self {
        Self::new(&std::env::var("RB_TEST_PACKAGE_BACKEND").unwrap_or_else(|_| "bundler".into()))
    }

    pub fn configured() -> Self {
        let mut fixture = Self::selected();
        let directory = fixture.rubies.path().join("registry");
        let output = Command::new(
            fixture
                .rubies
                .path()
                .join(format!("ruby-{}", fixture.version))
                .join("bin/ruby"),
        )
        .arg(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/package-backend/build_registry.rb"),
        )
        .arg(fixture.root.path().join("local"))
        .arg(&directory)
        .output()
        .unwrap();
        success(output);
        let registry = registry::Registry::serve(&directory);
        fs::write(
            fixture.root.path().join("Gemfile"),
            format!("source '{}'\ngem 'backend_fixture'\n", registry.url),
        )
        .unwrap();
        fixture.registry = Some(registry);
        fixture
    }

    pub fn bin_dir(&self) -> PathBuf {
        let abi = self
            .version
            .split('.')
            .take(2)
            .collect::<Vec<_>>()
            .join(".")
            + ".0";
        let root = self.root.path().canonicalize().unwrap();
        let bundle = match self.backend.as_str() {
            "bundler" => root.join(".rb/vendor/bundler"),
            "native" => root.join(".rb/bundle"),
            _ => unreachable!(),
        };
        bundle.join("ruby").join(abi).join("bin")
    }

    pub fn new(backend: &str) -> Self {
        assert!(
            matches!(backend, "bundler" | "native"),
            "Unknown test backend: {backend}"
        );
        let root = tempfile::tempdir().unwrap();
        let ruby = std::env::var_os("RB_GEM_BUILD_RUBY")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                which::which("ruby").expect("Ruby is required for package integration tests")
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
            "local/exe/backend_context",
            "local/ext/backend_fixture/extconf.rb",
            "local/ext/backend_fixture/backend_fixture_native.c",
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
        fs::write(
            rubies.path().join("rb.toml"),
            format!("package-backend = '{backend}'\n"),
        )
        .unwrap();
        Self {
            binary: env!("CARGO_BIN_EXE_rb").into(),
            root,
            rubies,
            version,
            backend: backend.into(),
            registry: None,
        }
    }

    pub fn instrument_ruby(&self) -> PathBuf {
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

    pub fn bare_command(&self) -> Command {
        let mut command = Command::new(&self.binary);
        command.current_dir(self.root.path());
        command.env("XDG_CACHE_HOME", self.rubies.path().join("cache"));
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
        for key in [
            "RB_PROJECT",
            "RB_WORK_DIR",
            "RB_RUBY_VERSION",
            "RB_RUBIES_DIR",
            "RB_GEM_HOME",
        ] {
            command.env_remove(key);
        }
        command
    }

    pub fn command(&self) -> Command {
        self.command_in(self.root.path(), self.rubies.path())
    }

    pub fn command_in(&self, directory: &Path, rubies: &Path) -> Command {
        let mut command = self.bare_command();
        command.args([
            "-R",
            rubies.to_str().unwrap(),
            "-r",
            &self.version,
            "-C",
            directory.to_str().unwrap(),
            "--config",
            self.rubies.path().join("rb.toml").to_str().unwrap(),
        ]);

        command
    }
}

pub fn success(output: Output) -> Output {
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}
