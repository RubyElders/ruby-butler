mod common;

use common::server::Server;
use rb_packages::{LockMode, PrepareOptions, prepare};
use rb_task::TaskEvents;
use std::{collections::BTreeMap, fs, path::Path, process::Command};

const FIXTURES: &str = "tests/fixtures/lockfile_equivalence";

fn success(command: &mut Command) {
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{command:?}\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn fixture(root: &Path, name: &str) {
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(FIXTURES)
            .join(name),
        root.join(name),
    )
    .unwrap();
}

#[test]
fn native_and_bundler_resolve_identical_locks_with_registry_git_and_path_sources() {
    let root = tempfile::tempdir().unwrap();
    let registry = root.path().join("registry");
    success(
        Command::new(common::ruby())
            .arg(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join(FIXTURES)
                    .join("registry.rb"),
            )
            .arg(&registry),
    );
    let mut responses = BTreeMap::new();
    responses.insert(
        "/versions".into(),
        fs::read(registry.join("versions")).unwrap(),
    );
    for directory in ["info", "gems"] {
        for entry in fs::read_dir(registry.join(directory)).unwrap() {
            let path = entry.unwrap().path();
            responses.insert(
                format!(
                    "/{directory}/{}",
                    path.file_name().unwrap().to_str().unwrap()
                ),
                fs::read(path).unwrap(),
            );
        }
    }
    let server = Server::new(responses);
    let git = root.path().join("silver");
    let path = root.path().join("napkins");
    let project = root.path().join("project");
    for directory in [&git, &path, &project] {
        fs::create_dir(directory).unwrap();
    }
    fixture(&git, "silver.gemspec");
    fixture(&path, "napkins.gemspec");
    fixture(&project, "service.gemspec");
    fixture(&project, "Gemfile");
    for args in [
        vec!["init", "--initial-branch=main"],
        vec!["add", "."],
        vec![
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.test",
            "commit",
            "-m",
            "Fixture",
        ],
    ] {
        success(Command::new("git").arg("-C").arg(&git).args(args));
    }
    let gemfile = fs::read_to_string(project.join("Gemfile"))
        .unwrap()
        .replace(
            "ENV.fetch('FIXTURE_REGISTRY')",
            &serde_json::to_string(&server.url).unwrap(),
        )
        .replace(
            "ENV.fetch('FIXTURE_GIT')",
            &serde_json::to_string(git.to_str().unwrap()).unwrap(),
        );
    fs::write(project.join("Gemfile"), gemfile).unwrap();
    let options = PrepareOptions::new(&project, common::ruby(), root.path().join("native-cache"));
    let native = prepare(options.clone(), TaskEvents::default()).unwrap();
    let native_lock = fs::read_to_string(&native.input.lockfile).unwrap();
    fs::remove_file(&native.input.lockfile).unwrap();
    let mut bundler = Command::new(common::ruby());
    for (key, _) in std::env::vars_os() {
        if key
            .to_string_lossy()
            .to_ascii_uppercase()
            .starts_with("BUNDLE_")
        {
            bundler.env_remove(key);
        }
    }
    success(
        bundler
            .env_remove("RUBYOPT")
            .env_remove("RUBYLIB")
            .env("BUNDLE_IGNORE_CONFIG", "true")
            .env("BUNDLE_PATH", root.path().join("bundler-install"))
            .env("BUNDLE_USER_HOME", root.path().join("bundler-home"))
            .current_dir(&project)
            .args([
                "-rbundler",
                "-rbundler/cli",
                "-e",
                "Bundler::CLI.start(ARGV)",
                "--",
                "lock",
                "--add-checksums",
            ]),
    );
    assert!(
        server
            .requests
            .lock()
            .unwrap()
            .iter()
            .any(|path| path == "/versions")
    );
    let bundler_lock = fs::read_to_string(project.join("Gemfile.lock")).unwrap();
    assert_eq!(native_lock, bundler_lock);
    assert!(native_lock.contains("    porcelain (1.5.0)\n"));
    assert!(!native_lock.contains("    porcelain (2.0.0)\n"));
    let mut frozen = options;
    frozen.offline = true;
    frozen.plan.mode = LockMode::Locked;
    let repeated = prepare(frozen, TaskEvents::default()).unwrap();
    assert!(!repeated.plan.lock_changed);
    assert_eq!(
        fs::read_to_string(repeated.input.lockfile).unwrap(),
        bundler_lock
    );
}
