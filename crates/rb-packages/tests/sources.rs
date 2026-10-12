mod common;
use rb_packages::{LockMode, PrepareOptions, PreparedBundle, prepare};
use rb_task::TaskEvents;
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn source(root: &Path) -> PathBuf {
    let directory = root.join("source-repository");
    fs::create_dir_all(directory.join("lib/fixture")).unwrap();
    fs::write(
        directory.join("fixture.gemspec"),
        include_str!("fixtures/sources/fixture.gemspec"),
    )
    .unwrap();
    fs::write(
        directory.join("lib/fixture.rb"),
        include_str!("fixtures/sources/lib/fixture.rb"),
    )
    .unwrap();
    fs::write(
        directory.join("lib/fixture/version.rb"),
        include_str!("fixtures/sources/lib/fixture/version.rb"),
    )
    .unwrap();
    directory
}

fn git(directory: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(arguments)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}

fn commit(directory: &Path) -> String {
    git(directory, &["add", "."]);
    git(
        directory,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.test",
            "commit",
            "-m",
            "Fixture",
        ],
    );
    git(directory, &["rev-parse", "HEAD"])
}

fn options(root: &Path) -> PrepareOptions {
    let mut options = PrepareOptions::new(root, common::ruby(), root.join(".rb/cache"));
    options
        .install
        .build
        .environment
        .insert("CCACHE_DISABLE".into(), "1".into());
    options
}

fn require(bundle: &PreparedBundle, code: &str) -> String {
    let output = Command::new(common::ruby())
        .envs(bundle.installed.environment())
        .env_remove("RUBYOPT")
        .env_remove("RUBYLIB")
        .env("BUNDLE_FROZEN", "true")
        .current_dir(&bundle.input.root)
        .args(["-rbundler/setup", "-e", code])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    common::stdout(&output)
}

#[test]
fn git_source_locks_commit_installs_for_bundler_and_reuses_offline() {
    let root = tempfile::tempdir().unwrap();
    let source = source(root.path());
    git(&source, &["init", "--initial-branch=main"]);
    let revision = commit(&source);
    fs::write(
        root.path().join("Gemfile"),
        format!(
            "gem 'fixture', git: {}, branch: 'main'\n",
            serde_json::to_string(&source.to_string_lossy()).unwrap()
        ),
    )
    .unwrap();
    let first = prepare(options(root.path()), TaskEvents::default()).unwrap();
    let git_source = first
        .plan
        .lockfile
        .sources
        .iter()
        .find(|s| s.kind == rb_lockfile::SourceKind::Git)
        .unwrap();
    assert_eq!(git_source.options["revision"], revision);
    assert_eq!(git_source.options["branch"], "main");
    assert_eq!(
        require(&first, "require 'fixture'; puts Fixture::VERSION"),
        "1.0.0\n"
    );
    let mut offline = options(root.path());
    offline.offline = true;
    offline.plan.mode = LockMode::Locked;
    let second = prepare(offline, TaskEvents::default()).unwrap();
    assert_eq!(second.installed.reused, 1);
    assert!(!second.plan.lock_changed);
    assert_eq!(
        require(&second, "require 'fixture'; puts Fixture::VERSION"),
        "1.0.0\n"
    );
    fs::write(
        source.join("lib/fixture/version.rb"),
        "module Fixture; VERSION='1.0.1'; end\n",
    )
    .unwrap();
    let changed_revision = commit(&source);
    let preferred = prepare(options(root.path()), TaskEvents::default()).unwrap();
    assert_eq!(
        require(&preferred, "require 'fixture'; puts Fixture::VERSION"),
        "1.0.0\n"
    );
    let mut update = options(root.path());
    update.plan.mode = LockMode::Update;
    let changed = prepare(update, TaskEvents::default()).unwrap();
    assert_eq!(
        require(&changed, "require 'fixture'; puts Fixture::VERSION"),
        "1.0.1\n"
    );
    assert_eq!(
        changed
            .plan
            .lockfile
            .sources
            .iter()
            .find(|s| s.kind == rb_lockfile::SourceKind::Git)
            .unwrap()
            .options["revision"],
        changed_revision
    );
}

#[test]
fn path_source_is_snapshotted_without_mutating_project_and_refreshes_on_edit() {
    let root = tempfile::tempdir().unwrap();
    let source = source(root.path());
    fs::write(
        root.path().join("Gemfile"),
        "gem 'fixture', path: 'source-repository'\n",
    )
    .unwrap();
    let first = prepare(options(root.path()), TaskEvents::default()).unwrap();
    assert_eq!(
        require(&first, "require 'fixture'; puts Fixture::VERSION"),
        "1.0.0\n"
    );
    assert!(first.input.lockfile.is_file());
    assert_eq!(
        first
            .plan
            .lockfile
            .sources
            .iter()
            .find(|s| s.kind == rb_lockfile::SourceKind::Path)
            .unwrap()
            .remotes,
        [dunce::canonicalize(&source)
            .unwrap()
            .to_string_lossy()
            .into_owned()]
    );
    let second = prepare(options(root.path()), TaskEvents::default()).unwrap();
    assert_eq!(second.installed.reused, 1);
    fs::write(
        source.join("lib/fixture.rb"),
        "require_relative 'fixture/version'; CHANGED=true\n",
    )
    .unwrap();
    let changed = prepare(options(root.path()), TaskEvents::default()).unwrap();
    assert_eq!(changed.installed.installed, 1);
    assert_eq!(
        require(&changed, "require 'fixture'; puts CHANGED"),
        "true\n"
    );
    assert!(!source.join("installation.json").exists());
}

#[cfg(unix)]
#[test]
fn native_sources_compile_and_load_with_frozen_bundler() {
    for kind in ["git", "path"] {
        let root = tempfile::tempdir().unwrap();
        let source = source(root.path());
        fs::create_dir_all(source.join("ext/fixture")).unwrap();
        fs::write(
            source.join("ext/fixture/extconf.rb"),
            include_str!("fixtures/sources/extconf.rb"),
        )
        .unwrap();
        fs::write(
            source.join("ext/fixture/source_native.c"),
            include_str!("fixtures/sources/source_native.c"),
        )
        .unwrap();
        fs::write(
            source.join("lib/fixture.rb"),
            "require_relative 'fixture/version'; require 'source_native'\n",
        )
        .unwrap();
        if kind == "git" {
            git(&source, &["init", "--initial-branch=main"]);
            commit(&source);
        }
        fs::write(
            root.path().join("Gemfile"),
            format!(
                "gem 'fixture', {kind}: {}\n",
                serde_json::to_string(&source.to_string_lossy()).unwrap()
            ),
        )
        .unwrap();
        let bundle = prepare(options(root.path()), TaskEvents::default()).unwrap();
        assert_eq!(
            require(
                &bundle,
                "require 'fixture'; raise if Gem.loaded_specs.fetch('fixture').missing_extensions?; puts Fixture.answer"
            ),
            "42\n",
            "{kind}"
        );
        assert!(!source.join("ext/fixture/Makefile").exists());
    }
}

#[test]
fn git_ref_and_tag_select_exact_commits_and_custom_glob() {
    let root = tempfile::tempdir().unwrap();
    let source = source(root.path());
    git(&source, &["init", "--initial-branch=main"]);
    let old = commit(&source);
    git(&source, &["tag", "v1"]);
    fs::write(
        source.join("lib/fixture/version.rb"),
        "module Fixture; VERSION='2.0.0'; end\n",
    )
    .unwrap();
    commit(&source);
    for selector in ["tag: 'v1'".to_owned(), format!("ref: '{old}'")] {
        fs::write(
            root.path().join("Gemfile"),
            format!(
                "gem 'fixture', git: {}, {selector}, glob: 'fixture.gemspec'\n",
                serde_json::to_string(&source.to_string_lossy()).unwrap()
            ),
        )
        .unwrap();
        let bundle = prepare(options(root.path()), TaskEvents::default()).unwrap();
        assert_eq!(
            require(&bundle, "require 'fixture'; puts Fixture::VERSION"),
            "1.0.0\n"
        );
        let lock = bundle
            .plan
            .lockfile
            .sources
            .iter()
            .find(|s| s.kind == rb_lockfile::SourceKind::Git)
            .unwrap();
        assert_eq!(lock.options["revision"], old);
        assert_eq!(lock.options["glob"], "fixture.gemspec");
    }
}

#[test]
fn source_gems_from_one_repository_resolve_transitive_siblings() {
    let root = tempfile::tempdir().unwrap();
    let source = source(root.path());
    let spec = fs::read_to_string(source.join("fixture.gemspec"))
        .unwrap()
        .replace(
            "spec.name = 'fixture'",
            "spec.name = 'fixture'; spec.add_runtime_dependency('helper', '>= 1')",
        );
    fs::write(source.join("fixture.gemspec"), spec).unwrap();
    fs::write(
        source.join("helper.gemspec"),
        include_str!("fixtures/sources/helper.gemspec"),
    )
    .unwrap();
    fs::write(source.join("lib/helper.rb"), "HELPER=42\n").unwrap();
    fs::write(
        source.join("lib/fixture.rb"),
        "require 'helper'; require_relative 'fixture/version'\n",
    )
    .unwrap();
    git(&source, &["init", "--initial-branch=main"]);
    commit(&source);
    fs::write(
        root.path().join("Gemfile"),
        format!(
            "gem 'fixture', git: {}\n",
            serde_json::to_string(&source.to_string_lossy()).unwrap()
        ),
    )
    .unwrap();
    let bundle = prepare(options(root.path()), TaskEvents::default()).unwrap();
    assert_eq!(bundle.plan.packages.len(), 2);
    assert_eq!(require(&bundle, "require 'fixture'; puts HELPER"), "42\n");
    assert!(
        bundle
            .plan
            .packages
            .iter()
            .all(|p| matches!(p.record.source, rb_packages::PackageSource::Git { .. }))
    );
}

#[cfg(unix)]
#[test]
fn source_symlink_cannot_escape_the_package_directory() {
    let root = tempfile::tempdir().unwrap();
    let source = source(root.path());
    fs::write(root.path().join("outside"), "private\n").unwrap();
    std::os::unix::fs::symlink(root.path().join("outside"), source.join("leak")).unwrap();
    fs::write(
        root.path().join("Gemfile"),
        "gem 'fixture', path: 'source-repository'\n",
    )
    .unwrap();
    let error = prepare(options(root.path()), TaskEvents::default())
        .err()
        .unwrap();
    assert!(format!("{error:#}").contains("Source link escapes"));
}

#[test]
fn snapshots_omit_unlisted_project_files_and_repair_missing_runtime_files() {
    let root = tempfile::tempdir().unwrap();
    let source = source(root.path());
    fs::write(source.join(".env"), "TOKEN=fixture-secret\n").unwrap();
    fs::write(
        root.path().join("Gemfile"),
        "gem 'fixture', path: 'source-repository'\n",
    )
    .unwrap();
    let first = prepare(options(root.path()), TaskEvents::default()).unwrap();
    assert!(
        !first
            .installed
            .gem_home
            .join("gems/fixture-1.0.0/.env")
            .exists()
    );
    assert_eq!(
        require(
            &first,
            "puts File.exist?(File.join(Gem.loaded_specs.fetch('fixture').full_gem_path, '.env'))"
        ),
        "false\n"
    );
    let loaded = require(
        &first,
        "puts Gem.loaded_specs.fetch('fixture').full_gem_path",
    );
    fs::remove_file(Path::new(loaded.trim()).join("lib/fixture.rb")).unwrap();
    let repaired = prepare(options(root.path()), TaskEvents::default()).unwrap();
    assert_eq!(
        require(&repaired, "require 'fixture'; puts Fixture::VERSION"),
        "1.0.0\n"
    );
}

#[test]
fn locked_path_rejects_changed_dependency_metadata_without_fetching_new_packages() {
    let root = tempfile::tempdir().unwrap();
    let source = source(root.path());
    fs::write(
        root.path().join("Gemfile"),
        "gem 'fixture', path: 'source-repository'\n",
    )
    .unwrap();
    prepare(options(root.path()), TaskEvents::default()).unwrap();
    let old = fs::read_to_string(source.join("fixture.gemspec")).unwrap();
    fs::write(
        source.join("fixture.gemspec"),
        old.replace(
            "spec.name = 'fixture'",
            "spec.name = 'fixture'; spec.add_runtime_dependency('unavailable', '>= 0')",
        ),
    )
    .unwrap();
    let mut locked = options(root.path());
    locked.offline = true;
    locked.plan.mode = LockMode::Locked;
    let error = prepare(locked, TaskEvents::default()).err().unwrap();
    assert!(format!("{error:#}").contains("Locked dependency metadata changed"));
}

#[test]
fn cached_git_checkout_modifications_are_rejected() {
    let root = tempfile::tempdir().unwrap();
    let source = source(root.path());
    git(&source, &["init", "--initial-branch=main"]);
    commit(&source);
    fs::write(
        root.path().join("Gemfile"),
        format!(
            "gem 'fixture', git: {}\n",
            serde_json::to_string(&source.to_string_lossy()).unwrap()
        ),
    )
    .unwrap();
    let first = prepare(options(root.path()), TaskEvents::default()).unwrap();
    let cache = root.path().join(".rb/cache/git");
    let repository = fs::read_dir(cache).unwrap().next().unwrap().unwrap().path();
    let checkout = fs::read_dir(repository.join("checkouts"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    fs::write(checkout.join("lib/fixture.rb"), "raise 'tampered'\n").unwrap();
    let mut offline = options(root.path());
    offline.offline = true;
    let error = prepare(offline, TaskEvents::default()).err().unwrap();
    assert!(format!("{error:#}").contains("Cached Git checkout has been modified"));
    assert_eq!(
        require(&first, "require 'fixture'; puts Fixture::VERSION"),
        "1.0.0\n"
    );
}

#[path = "common/git_server.rs"]
mod git_server;

#[test]
fn git_submodules_are_materialized_and_reused_offline() {
    let root = tempfile::tempdir().unwrap();
    let module = root.path().join("metadata-module");
    fs::create_dir(&module).unwrap();
    git(&module, &["init", "--initial-branch=main"]);
    fs::write(
        module.join("version.rb"),
        include_str!("fixtures/sources/lib/fixture/version.rb"),
    )
    .unwrap();
    commit(&module);
    let bare = root.path().join("metadata-module.git");
    git(
        root.path(),
        &[
            "clone",
            "--bare",
            module.to_str().unwrap(),
            bare.to_str().unwrap(),
        ],
    );
    git(&bare, &["update-server-info"]);
    let server = git_server::GitServer::start(&bare);
    let source = source(root.path());
    git(&source, &["init", "--initial-branch=main"]);
    git(&source, &["submodule", "add", &server.url, "vendor"]);
    fs::write(
        source.join("lib/fixture/version.rb"),
        "require_relative '../../vendor/version'\n",
    )
    .unwrap();
    let spec = fs::read_to_string(source.join("fixture.gemspec"))
        .unwrap()
        .replace("{lib,ext}", "{lib,ext,vendor}");
    fs::write(source.join("fixture.gemspec"), spec).unwrap();
    commit(&source);
    fs::write(
        root.path().join("Gemfile"),
        format!(
            "gem 'fixture', git: {}, submodules: true\n",
            serde_json::to_string(&source.to_string_lossy()).unwrap()
        ),
    )
    .unwrap();
    let first = prepare(options(root.path()), TaskEvents::default()).unwrap();
    assert_eq!(
        require(&first, "require 'fixture'; puts Fixture::VERSION"),
        "1.0.0\n"
    );
    drop(server);
    let mut offline = options(root.path());
    offline.offline = true;
    offline.plan.mode = LockMode::Locked;
    let second = prepare(offline, TaskEvents::default()).unwrap();
    assert_eq!(second.installed.reused, 1);
    assert_eq!(
        require(&second, "require 'fixture'; puts Fixture::VERSION"),
        "1.0.0\n"
    );
}

#[test]
fn path_sources_with_distinct_globs_keep_distinct_compatibility_locations() {
    let root = tempfile::tempdir().unwrap();
    let source = source(root.path());
    fs::write(
        source.join("helper.gemspec"),
        include_str!("fixtures/sources/helper.gemspec"),
    )
    .unwrap();
    fs::write(source.join("lib/helper.rb"), "HELPER=42\n").unwrap();
    fs::write(root.path().join("Gemfile"), "gem 'fixture', path: 'source-repository', glob: 'fixture.gemspec'\ngem 'helper', path: 'source-repository', glob: 'helper.gemspec'\n").unwrap();
    let bundle = prepare(options(root.path()), TaskEvents::default()).unwrap();
    assert_eq!(
        require(&bundle, "require 'fixture'; require 'helper'; puts HELPER"),
        "42\n"
    );
}

#[test]
fn platform_specific_git_source_is_rejected_before_acquisition() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("Gemfile"),
        "gem 'fixture', git: 'https://unavailable.example.test/fixture.git', platforms: :jruby\n",
    )
    .unwrap();
    let mut options = options(root.path());
    options.offline = true;
    let error = prepare(options, TaskEvents::default()).err().unwrap();
    assert!(format!("{error:#}").contains("Platform-specific Git/path sources"));
    assert!(!root.path().join(".rb/cache/git").exists());
}
