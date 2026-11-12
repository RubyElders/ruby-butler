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

#[cfg(unix)]
fn native_source(root: &Path) -> PathBuf {
    let source = source(root);
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
    source
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
fn generated_path_locks_are_relative_and_local_gemspec_pins_match_bundler() {
    let root = tempfile::tempdir().unwrap();
    let source = source(root.path());
    let consumer = root.path().join("consumer");
    fs::create_dir(&consumer).unwrap();
    fs::write(
        consumer.join("Gemfile"),
        "gem 'fixture', path: '../source-repository'\n",
    )
    .unwrap();
    let first = prepare(options(&consumer), TaskEvents::default()).unwrap();
    let lock = fs::read_to_string(&first.input.lockfile).unwrap();
    assert!(
        lock.starts_with("PATH\n  remote: ../source-repository\n"),
        "{lock}"
    );
    let mut frozen = options(&consumer);
    frozen.plan.mode = LockMode::Locked;
    let same = prepare(frozen, TaskEvents::default()).unwrap();
    assert!(!same.plan.lock_changed);
    assert_eq!(fs::read_to_string(&same.input.lockfile).unwrap(), lock);

    fs::write(source.join("Gemfile"), "gemspec\n").unwrap();
    let local = prepare(options(&source), TaskEvents::default()).unwrap();
    assert_eq!(
        require(&local, "require 'fixture'; puts Fixture::VERSION"),
        "1.0.0\n"
    );
    let lock = fs::read_to_string(&local.input.lockfile).unwrap();
    assert!(lock.starts_with("PATH\n  remote: .\n"), "{lock}");
    assert!(lock.contains("\n  fixture!\n"), "{lock}");
    assert!(!lock.contains("fixture (= "), "{lock}");
    let mut frozen = options(&source);
    frozen.plan.mode = LockMode::Locked;
    let same = prepare(frozen, TaskEvents::default()).unwrap();
    assert!(!same.plan.lock_changed);
    assert_eq!(fs::read_to_string(&same.input.lockfile).unwrap(), lock);
}

#[test]
#[cfg(unix)]
fn path_gem_freshness_tracks_its_inputs_without_walking_unrelated_directories() {
    let root = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    let source = source(root.path());
    fs::create_dir(source.join(".trees")).unwrap();
    std::os::unix::fs::symlink(external.path(), source.join(".trees/unrelated")).unwrap();
    fs::write(
        root.path().join("Gemfile"),
        "gemspec path: 'source-repository'\n",
    )
    .unwrap();
    let mut options = options(root.path());
    options.record_state = true;
    let first = prepare(options.clone(), TaskEvents::default()).unwrap();
    assert!(rb_packages::prepared(&options).unwrap().is_some());
    fs::write(external.path().join("unrelated"), "changed").unwrap();
    fs::write(source.join("notes.txt"), "not packaged").unwrap();
    assert!(rb_packages::prepared(&options).unwrap().is_some());
    fs::write(source.join("lib/new_file.rb"), "NEW_FILE = 42\n").unwrap();
    assert!(rb_packages::prepared(&options).unwrap().is_none());
    let changed = prepare(options.clone(), TaskEvents::default()).unwrap();
    assert_ne!(first.fingerprint, changed.fingerprint);
    assert_eq!(
        require(&changed, "require 'new_file'; puts NEW_FILE"),
        "42\n"
    );
    fs::write(
        source.join("lib/fixture/version.rb"),
        "module Fixture; VERSION = '1.0.1'; end\n",
    )
    .unwrap();
    assert!(rb_packages::prepared(&options).unwrap().is_none());
    let changed = prepare(options, TaskEvents::default()).unwrap();
    assert_eq!(
        require(&changed, "require 'fixture'; puts Fixture::VERSION"),
        "1.0.1\n"
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
        ["source-repository"]
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
        let source = native_source(root.path());
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
        let marker = require(
            &bundle,
            "puts File.join(Gem.loaded_specs.fetch('fixture').extension_dir, 'gem.build_complete')",
        );
        fs::remove_file(marker.trim()).unwrap();
        let repaired = prepare(options(root.path()), TaskEvents::default()).unwrap();
        assert_eq!(
            require(
                &repaired,
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
fn packaged_symlink_cannot_escape_the_package_directory() {
    let root = tempfile::tempdir().unwrap();
    let source = source(root.path());
    fs::write(root.path().join("outside"), "private\n").unwrap();
    std::os::unix::fs::symlink(root.path().join("outside"), source.join("lib/leak")).unwrap();
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
fn inactive_platform_git_source_is_rejected_before_acquisition() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("Gemfile"),
        "gem 'fixture', git: 'https://unavailable.example.test/fixture.git', platforms: :jruby\n",
    )
    .unwrap();
    let mut options = options(root.path());
    options.offline = true;
    let error = prepare(options, TaskEvents::default()).err().unwrap();
    assert!(
        format!("{error:#}")
            .contains("Git/path source for fixture has no applicable resolution target")
    );
    assert!(!root.path().join(".rb/cache/git").exists());
}

#[test]
fn active_platform_path_source_is_prepared() {
    let root = tempfile::tempdir().unwrap();
    source(root.path());
    fs::write(
        root.path().join("Gemfile"),
        "gem 'fixture', path: 'source-repository', platforms: [:ruby, :windows]\n",
    )
    .unwrap();
    let bundle = prepare(options(root.path()), TaskEvents::default()).unwrap();
    assert_eq!(
        require(&bundle, "require 'fixture'; puts Fixture::VERSION"),
        "1.0.0\n"
    );
}

#[test]
fn platform_git_source_selects_a_pinned_local_repository() {
    let root = tempfile::tempdir().unwrap();
    let directory = source(root.path());
    git(&directory, &["init", "-q"]);
    let revision = commit(&directory);
    fs::write(
        root.path().join("Gemfile"),
        format!(
            "gem 'fixture', git: {:?}, ref: {revision:?}, platforms: [:ruby, :windows]\n",
            directory.to_string_lossy()
        ),
    )
    .unwrap();
    let bundle = prepare(options(root.path()), TaskEvents::default()).unwrap();
    assert_eq!(
        require(&bundle, "require 'fixture'; puts Fixture::VERSION"),
        "1.0.0\n"
    );
    assert!(
        bundle
            .plan
            .lockfile
            .sources
            .iter()
            .any(|source| source.options.get("revision") == Some(&revision))
    );
}

#[test]
fn different_sources_across_targets_are_rejected_before_acquisition() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("Gemfile"),
        "gem 'fixture', git: 'https://ruby.example.test/repo', platforms: [:ruby, :windows]\n",
    )
    .unwrap();
    let runtime = common::runtime();
    let mut input = rb_packages::ProjectInput::load(root.path(), &runtime.executable).unwrap();
    let mut alternate = input.manifest.dependencies["fixture"].clone();
    alternate.git = Some("https://java.example.test/repo".into());
    alternate.platforms = vec!["jruby".into()];
    input
        .manifest
        .dependencies
        .get_mut("fixture")
        .unwrap()
        .variants
        .push(alternate);
    let previous = rb_lockfile::Lockfile {
        platforms: vec![runtime.platform.clone(), "java".into()],
        ..Default::default()
    };
    let result = rb_packages::SourceIndex::new(
        &input,
        &runtime,
        &root.path().join(".rb/cache"),
        true,
        Some(&previous),
        LockMode::Update,
    );
    let error = result.err().unwrap();
    assert!(
        format!("{error:#}").contains("Different sources across targets for fixture"),
        "{error:#}"
    );
    assert!(!root.path().join(".rb/cache/git").exists());
}

#[test]
fn compatibility_gemfile_keeps_optional_groups_inactive() {
    let root = tempfile::tempdir().unwrap();
    let directory = source(root.path());
    fs::write(
        directory.join("helper.gemspec"),
        include_str!("fixtures/sources/helper.gemspec"),
    )
    .unwrap();
    fs::write(directory.join("lib/helper.rb"), "HELPER=42\n").unwrap();
    fs::write(
        root.path().join("Gemfile"),
        include_str!("fixtures/sources/optional.Gemfile"),
    )
    .unwrap();
    let bundle = prepare(options(root.path()), TaskEvents::default()).unwrap();
    assert_eq!(
        require(
            &bundle,
            "raise 'optional helper activated' if Gem.loaded_specs.key?('helper'); puts 'default only'"
        ),
        "default only\n"
    );
    let mut enabled = options(root.path());
    enabled.plan.with.insert("extras".into());
    let bundle = prepare(enabled, TaskEvents::default()).unwrap();
    assert_eq!(require(&bundle, "require 'helper'; puts HELPER"), "42\n");
}

#[test]
fn unlisted_source_files_preserve_path_snapshot_locations() {
    let root = tempfile::tempdir().unwrap();
    let source = source(root.path());
    fs::write(
        root.path().join("Gemfile"),
        "gem 'fixture', path: 'source-repository'\n",
    )
    .unwrap();
    let first = prepare(options(root.path()), TaskEvents::default()).unwrap();
    let before = require(
        &first,
        "puts Gem.loaded_specs.fetch('fixture').full_gem_path",
    );
    fs::write(source.join("notes.txt"), "unlisted source file").unwrap();
    let second = prepare(options(root.path()), TaskEvents::default()).unwrap();
    assert_eq!(first.fingerprint, second.fingerprint);
    let after = require(
        &second,
        "require 'fixture'; puts Gem.loaded_specs.fetch('fixture').full_gem_path",
    );
    assert_eq!(before, after);
    assert_eq!(
        require(&second, "require 'fixture'; puts Fixture::VERSION"),
        "1.0.0\n"
    );
}

#[test]
fn local_gemspec_environment_changes_invalidate_prepared_state() {
    let root = tempfile::tempdir().unwrap();
    let source = source(root.path());
    fs::write(source.join("fixture.gemspec"), "Gem::Specification.new { |s| s.name='fixture'; s.version=ENV.fetch('RAILS_ENV', '1.0.0'); s.summary='Fixture'; s.authors=['Fixture']; s.files=['lib/fixture.rb', 'lib/fixture/version.rb'] }\n").unwrap();
    fs::write(
        root.path().join("Gemfile"),
        "gem 'fixture', path: 'source-repository'\n",
    )
    .unwrap();
    let options_for = |version: &str| {
        let mut options = options(root.path());
        let mut environment = options.install.build.inherited_environment().clone();
        environment.insert("RAILS_ENV".into(), version.into());
        options.install.build = rb_gem_installer::BuildOptions::with_inputs(
            &options.ruby,
            root.path().to_owned(),
            environment,
        );
        options.record_state = true;
        options
    };
    let first = options_for("1.0.0");
    prepare(first.clone(), TaskEvents::default()).unwrap();
    assert!(rb_packages::prepared(&first).unwrap().is_some());
    let changed = options_for("2.0.0");
    assert!(rb_packages::prepared(&changed).unwrap().is_none());
    let prepared = prepare(changed, TaskEvents::default()).unwrap();
    assert_eq!(
        prepared.plan.packages[0]
            .record
            .package
            .id
            .version
            .to_string(),
        "2.0.0"
    );
}

#[test]
fn annotated_git_tags_keep_bundler_lock_identity_and_checkout_the_commit() {
    let root = tempfile::tempdir().unwrap();
    let source = source(root.path());
    git(&source, &["init", "--initial-branch=main"]);
    let commit = commit(&source);
    git(
        &source,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.test",
            "tag",
            "-a",
            "v1",
            "-m",
            "Fixture",
        ],
    );
    let tag = git(&source, &["rev-parse", "v1"]);
    assert_ne!(tag, commit);
    fs::write(
        root.path().join("Gemfile"),
        format!(
            "gem 'fixture', git: {:?}, tag: 'v1'\n",
            source.to_string_lossy()
        ),
    )
    .unwrap();
    let prepared = prepare(options(root.path()), TaskEvents::default()).unwrap();
    let source = prepared
        .plan
        .lockfile
        .sources
        .iter()
        .find(|source| source.kind == rb_lockfile::SourceKind::Git)
        .unwrap();
    assert_eq!(source.options["revision"], tag);
    assert_eq!(
        require(&prepared, "require 'fixture'; puts Fixture::VERSION"),
        "1.0.0\n"
    );
    let lock = fs::read_to_string(&prepared.input.lockfile).unwrap();
    let mut frozen = options(root.path());
    frozen.plan.mode = LockMode::Locked;
    let warm = prepare(frozen, TaskEvents::default()).unwrap();
    assert_eq!(fs::read_to_string(warm.input.lockfile).unwrap(), lock);
}

#[cfg(unix)]
#[test]
fn restored_git_extensions_reuse_builds_after_a_fresh_checkout() {
    let root = tempfile::tempdir().unwrap();
    let source = native_source(root.path());
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
    let settings = options(root.path());
    let runtime = common::runtime();
    let input = rb_packages::ProjectInput::load(root.path(), &runtime.executable).unwrap();
    rb_packages::SourceIndex::new(
        &input,
        &runtime,
        &settings.cache,
        false,
        None,
        LockMode::Prefer,
    )
    .unwrap();
    let repository = fs::read_dir(settings.cache.join("git"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let checkout = fs::read_dir(repository.join("checkouts"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    for file in git(&checkout, &["ls-files"]).lines() {
        fs::File::options()
            .write(true)
            .open(checkout.join(file))
            .unwrap()
            .set_modified(
                std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000),
            )
            .unwrap();
    }
    let first = prepare(settings.clone(), TaskEvents::default()).unwrap();
    assert_eq!(first.installed.installed, 1);
    let lock = fs::read(root.path().join("Gemfile.lock")).unwrap();
    let saved_home = root.path().join("saved-gem-home");
    fs::rename(&first.installed.gem_home, &saved_home).unwrap();
    fs::remove_dir_all(root.path().join(".rb")).unwrap();
    fs::create_dir_all(first.installed.gem_home.parent().unwrap()).unwrap();
    fs::rename(&saved_home, &first.installed.gem_home).unwrap();
    let mut locked = settings;
    locked.plan.mode = LockMode::Locked;
    let restored = prepare(locked, TaskEvents::default()).unwrap();
    assert_eq!(restored.installed.installed, 0);
    assert_eq!(restored.installed.reused, 1);
    assert_eq!(fs::read(root.path().join("Gemfile.lock")).unwrap(), lock);
    assert_eq!(
        require(&restored, "require 'fixture'; puts Fixture.answer"),
        "42\n"
    );
}
