#![cfg(unix)]

mod common;
use common::source::*;
use rb_packages::{LockMode, PrepareOptions, prepare};
use rb_task::TaskEvents;
use std::{fs, process::Command};

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

#[test]
fn git_and_path_installations_run_from_restored_rb_without_original_sources_or_global_cache() {
    for kind in ["git", "path"] {
        let root = tempfile::tempdir().unwrap();
        let source = native_source(root.path());
        if kind == "git" {
            git(&source, &["init", "--initial-branch=main"]);
            commit(&source);
        }
        fs::write(
            root.path().join("Gemfile"),
            format!("gem 'fixture', {kind}: {:?}\n", source.to_string_lossy()),
        )
        .unwrap();
        let mut settings = options(root.path());
        settings.cache = root.path().join("global-cache");
        settings.install.cache = Some(settings.cache.clone());
        let prepared = prepare(settings.clone(), TaskEvents::default()).unwrap();
        assert_eq!(prepared.installed.installed, 1);
        assert!(!settings.cache.join("source-archives").exists());
        assert!(
            prepared
                .installed
                .gem_home
                .join("gems/fixture-1.0.0/installation.json")
                .is_file()
        );
        let mut writer = tar::Builder::new(flate2::write::GzEncoder::new(
            Vec::new(),
            flate2::Compression::fast(),
        ));
        writer
            .append_dir_all(".rb", root.path().join(".rb"))
            .unwrap();
        let archive = writer.into_inner().unwrap().finish().unwrap();
        fs::remove_dir_all(root.path().join(".rb")).unwrap();
        fs::remove_dir_all(&settings.cache).unwrap();
        fs::remove_dir_all(source).unwrap();
        tar::Archive::new(flate2::read::GzDecoder::new(archive.as_slice()))
            .unpack(root.path())
            .unwrap();
        assert_eq!(
            require(&prepared, "require 'fixture'; puts Fixture.answer"),
            "42\n"
        );
    }
}

fn counted_source(kind: &str) -> (tempfile::TempDir, PrepareOptions) {
    let root = tempfile::tempdir().unwrap();
    let source = native_source(root.path());
    fs::write(
        source.join("ext/fixture/extconf.rb"),
        include_str!("fixtures/sources/counted_extconf.rb"),
    )
    .unwrap();
    fs::write(
        source.join("ext/fixture/source_native.c"),
        include_str!("fixtures/sources/counted_source_native.c"),
    )
    .unwrap();
    if kind == "git" {
        git(&source, &["init", "--initial-branch=main"]);
        commit(&source);
    }
    fs::write(
        root.path().join("Gemfile"),
        format!("gem 'fixture', {kind}: {:?}\n", source.to_string_lossy()),
    )
    .unwrap();
    let mut settings = options(root.path());
    settings.cache = root.path().join("global-cache");
    settings.install.cache = Some(settings.cache.clone());
    settings.install.build.environment.insert(
        "BUILD_COUNTER".into(),
        root.path().join("builds").into_os_string(),
    );
    (root, settings)
}

#[test]
fn invalidated_source_installations_recompile_despite_existing_extension_markers() {
    for kind in ["git", "path"] {
        let (root, mut settings) = counted_source(kind);
        settings.install.cache = None;
        let first = prepare(settings.clone(), TaskEvents::default()).unwrap();
        let marker = require(
            &first,
            "puts File.join(Gem.loaded_specs.fetch('fixture').extension_dir, 'gem.build_complete')",
        );
        assert!(std::path::Path::new(marker.trim()).is_file());
        assert_eq!(
            fs::read_to_string(root.path().join("builds")).unwrap(),
            "build\n"
        );
        fs::remove_file(
            first
                .installed
                .gem_home
                .join("gems/fixture-1.0.0/installation.json"),
        )
        .unwrap();
        settings.offline = true;
        settings.plan.mode = LockMode::Locked;
        let repaired = prepare(settings, TaskEvents::default()).unwrap();
        assert_eq!(repaired.installed.installed, 1, "{kind}");
        assert_eq!(
            fs::read_to_string(root.path().join("builds")).unwrap(),
            "build\nbuild\n"
        );
        assert_eq!(
            require(&repaired, "require 'fixture'; puts Fixture.answer"),
            "43\n"
        );
        assert!(std::path::Path::new(marker.trim()).is_file());
    }
}

#[test]
fn source_extensions_reinstall_offline_from_only_the_global_cache_without_recompiling() {
    for kind in ["git", "path"] {
        let (root, mut settings) = counted_source(kind);
        let first = prepare(settings.clone(), TaskEvents::default()).unwrap();
        assert_eq!(first.installed.installed, 1);
        let lock = fs::read(root.path().join("Gemfile.lock")).unwrap();
        fs::remove_dir_all(root.path().join(".rb")).unwrap();
        settings.offline = true;
        settings.plan.mode = LockMode::Locked;
        let restored = prepare(settings, TaskEvents::default()).unwrap();
        assert_eq!(
            (restored.installed.installed, restored.installed.reused),
            (0, 1),
            "{kind}"
        );
        assert_eq!(
            fs::read_to_string(root.path().join("builds")).unwrap(),
            "build\n"
        );
        assert_eq!(fs::read(root.path().join("Gemfile.lock")).unwrap(), lock);
        assert_eq!(
            require(
                &restored,
                "require 'fixture'; raise if Gem.loaded_specs.fetch('fixture').missing_extensions?; puts Fixture.answer"
            ),
            "42\n"
        );
    }
}

#[test]
fn bundler_implicit_and_explicit_cleanup_preserve_native_git_extensions() {
    let (root, settings) = counted_source("git");
    let prepared = prepare(settings, TaskEvents::default()).unwrap();
    let marker = require(
        &prepared,
        "puts File.join(Gem.loaded_specs.fetch('fixture').extension_dir, 'gem.build_complete')",
    );
    let stale = prepared
        .installed
        .gem_home
        .join("bundler/gems/unused-0123456789ab");
    for arguments in [["install", "--local"], ["clean", "--force"]] {
        fs::create_dir_all(&stale).unwrap();
        let output = Command::new(common::ruby())
            .envs(prepared.installed.environment())
            .env_remove("RUBYOPT")
            .env_remove("RUBYLIB")
            .env("BUNDLE_FROZEN", "true")
            .env("BUNDLE_CLEAN", "true")
            .current_dir(root.path())
            .args([
                "-rbundler",
                "-rbundler/cli",
                "-e",
                "Bundler::CLI.start(ARGV)",
                "--",
            ])
            .args(arguments)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!stale.exists());
        assert!(std::path::Path::new(marker.trim()).is_file());
        assert_eq!(
            require(
                &prepared,
                "require 'fixture'; raise if Gem.loaded_specs.fetch('fixture').missing_extensions?; puts Fixture.answer"
            ),
            "42\n"
        );
        assert_eq!(
            fs::read_to_string(root.path().join("builds")).unwrap(),
            "build\n"
        );
    }
}
