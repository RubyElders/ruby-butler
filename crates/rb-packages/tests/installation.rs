mod common;
use common::*;
use rb_packages::{InstallOptions, MemoryIndex, PlanOptions, install, plan};
use rb_task::TaskEvents;
use std::{fs, sync::Arc};

#[test]
fn installed_bundle_runs_with_rubygems_bundler_and_executable_and_reuses() {
    let root = tempfile::tempdir().unwrap();
    let mut index = MemoryIndex::default();
    add_archive(
        &mut index,
        root.path(),
        "teacup",
        &[],
        &[],
        &[
            ("lib/teacup.rb", b"TEACUP = 42\n"),
            ("bin/teacup", b"require 'teacup'; puts TEACUP\n"),
        ],
    );
    fs::write(
        root.path().join("Gemfile"),
        "source 'https://rubygems.org'\ngem 'teacup'\n",
    )
    .unwrap();
    let runtime = runtime();
    let plan = plan(
        &manifest("https://rubygems.org", "teacup = '>= 0'"),
        &runtime,
        None,
        &index,
        &PlanOptions::default(),
    )
    .unwrap();
    let index = Arc::new(index);
    let bundle = install(
        root.path(),
        &plan,
        &runtime,
        index.clone(),
        InstallOptions::new(&runtime.executable),
        TaskEvents::default(),
    )
    .unwrap();
    fs::write(
        root.path().join("Gemfile.lock"),
        plan.lockfile.render().unwrap(),
    )
    .unwrap();
    for code in [
        "require 'rubygems'; require 'teacup'; puts TEACUP",
        "require 'bundler/setup'; require 'teacup'; puts TEACUP",
    ] {
        let output = run(&runtime, &bundle, root.path(), code);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(stdout(&output), "42\n");
    }
    let code = format!(
        "load {}",
        serde_json::to_string(&bundle.gem_home.join("bin/teacup").to_string_lossy()).unwrap()
    );
    let output = run(&runtime, &bundle, root.path(), &code);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(stdout(&output), "42\n");
    let reused = install(
        root.path(),
        &plan,
        &runtime,
        index,
        InstallOptions::new(&runtime.executable),
        TaskEvents::default(),
    )
    .unwrap();
    assert_eq!(bundle.installed, 1);
    assert_eq!(reused.installed, 0);
    assert_eq!(reused.reused, 1);
}

#[test]
fn failed_build_does_not_publish_a_bundle() {
    let root = tempfile::tempdir().unwrap();
    let mut index = MemoryIndex::default();
    add_archive(
        &mut index,
        root.path(),
        "broken",
        &[],
        &["ext/extconf.rb"],
        &[("ext/extconf.rb", b"raise 'missing prerequisite'\n")],
    );
    let runtime = runtime();
    let plan = plan(
        &manifest("https://rubygems.org", "broken = '>= 0'"),
        &runtime,
        None,
        &index,
        &PlanOptions::default(),
    )
    .unwrap();
    let result = install(
        root.path(),
        &plan,
        &runtime,
        Arc::new(index),
        InstallOptions::new(&runtime.executable),
        TaskEvents::default(),
    );
    assert!(format!("{:#}", result.unwrap_err()).contains("missing prerequisite"));
    assert!(
        !root
            .path()
            .join(".rb/bundle/ruby")
            .join(runtime.abi)
            .exists()
    );
}

#[test]
fn runtime_dependency_cycles_do_not_become_task_cycles() {
    let root = tempfile::tempdir().unwrap();
    let mut index = MemoryIndex::default();
    add_archive(
        &mut index,
        root.path(),
        "one",
        &[("two", ">= 0")],
        &[],
        &[("lib/one.rb", b"ONE = 1")],
    );
    add_archive(
        &mut index,
        root.path(),
        "two",
        &[("one", ">= 0")],
        &[],
        &[("lib/two.rb", b"TWO = 2")],
    );
    let runtime = runtime();
    let plan = plan(
        &manifest("https://rubygems.org", "one = '>= 0'"),
        &runtime,
        None,
        &index,
        &PlanOptions::default(),
    )
    .unwrap();
    let result = install(
        root.path(),
        &plan,
        &runtime,
        Arc::new(index),
        InstallOptions::new(&runtime.executable),
        TaskEvents::default(),
    )
    .unwrap();
    assert_eq!(result.installed, 2);
}

#[test]
fn excluded_groups_can_be_consumed_by_bundler() {
    let root = tempfile::tempdir().unwrap();
    let mut index = MemoryIndex::default();
    add_archive(
        &mut index,
        root.path(),
        "teacup",
        &[],
        &[],
        &[("lib/teacup.rb", b"TEACUP=42\n")],
    );
    add_archive(
        &mut index,
        root.path(),
        "test-helper",
        &[],
        &[],
        &[("lib/test-helper.rb", b"TEST_HELPER=true\n")],
    );
    fs::write(
        root.path().join("Gemfile"),
        include_str!("fixtures/dependencies.rb"),
    )
    .unwrap();
    let runtime = runtime();
    let plan = plan(
        &manifest(
            "https://rubygems.org",
            "teacup='>= 0'\ntest-helper={groups=['test']}",
        ),
        &runtime,
        None,
        &index,
        &PlanOptions {
            without: ["test".into()].into(),
            ..Default::default()
        },
    )
    .unwrap();
    let bundle = install(
        root.path(),
        &plan,
        &runtime,
        Arc::new(index),
        InstallOptions::new(&runtime.executable),
        TaskEvents::default(),
    )
    .unwrap();
    fs::write(
        root.path().join("Gemfile.lock"),
        plan.lockfile.render().unwrap(),
    )
    .unwrap();
    let output = run(
        &runtime,
        &bundle,
        root.path(),
        "require 'bundler/setup'; require 'teacup'; raise if Gem.loaded_specs.key?('test-helper'); puts TEACUP",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(stdout(&output), "42\n");
}

#[cfg(unix)]
#[test]
fn native_bundle_uses_transitive_dependencies_and_rubygems_extension_layout() {
    let root = tempfile::tempdir().unwrap();
    let mut index = MemoryIndex::default();
    add_archive(
        &mut index,
        root.path(),
        "leaf",
        &[],
        &[],
        &[("lib/leaf.rb", b"LEAF=42\n")],
    );
    add_archive(
        &mut index,
        root.path(),
        "middle",
        &[("leaf", ">= 0")],
        &[],
        &[("lib/middle.rb", b"require 'leaf'\n")],
    );
    add_archive(
        &mut index,
        root.path(),
        "native",
        &[("middle", ">= 0")],
        &["ext/extconf.rb"],
        &[
            (
                "ext/extconf.rb",
                include_bytes!("fixtures/native/extconf.rb"),
            ),
            (
                "ext/package_native.c",
                include_bytes!("fixtures/native/package_native.c"),
            ),
            ("lib/native.rb", b"require 'package_native'\n"),
        ],
    );
    let runtime = runtime();
    let plan = plan(
        &manifest("https://rubygems.org", "native='>= 0'"),
        &runtime,
        None,
        &index,
        &PlanOptions::default(),
    )
    .unwrap();
    let mut options = InstallOptions::new(&runtime.executable);
    options
        .build
        .environment
        .insert("CCACHE_DISABLE".into(), "1".into());
    let index = Arc::new(index);
    let bundle = install(
        root.path(),
        &plan,
        &runtime,
        index.clone(),
        options.clone(),
        TaskEvents::default(),
    )
    .unwrap();
    let marker = bundle
        .gem_home
        .join("extensions")
        .join(&runtime.platform)
        .join(&runtime.abi)
        .join("native-1.0.0/gem.build_complete");
    assert!(marker.is_file());
    let output = run(
        &runtime,
        &bundle,
        root.path(),
        "require 'rubygems'; require 'native'; raise if Gem.loaded_specs.fetch('native').missing_extensions?; puts PackageNative.answer",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(stdout(&output), "42\n");
    fs::remove_file(marker).unwrap();
    let repaired = install(
        root.path(),
        &plan,
        &runtime,
        index,
        options,
        TaskEvents::default(),
    )
    .unwrap();
    assert_eq!(repaired.installed, 1);
    assert_eq!(repaired.reused, 2);
}

#[test]
fn native_project_generates_a_gemfile_that_bundler_can_consume() {
    let root = tempfile::tempdir().unwrap();
    let mut index = MemoryIndex::default();
    add_archive(
        &mut index,
        root.path(),
        "teacup",
        &[],
        &[],
        &[("lib/teacup.rb", b"TEACUP=42\n")],
    );
    fs::write(
        root.path().join("rbproject.toml"),
        "[dependencies]\nteacup='>= 0'\n",
    )
    .unwrap();
    fs::create_dir(root.path().join(".bundle")).unwrap();
    fs::write(
        root.path().join(".bundle/config"),
        "---\nBUNDLE_PATH: missing-bundle\n",
    )
    .unwrap();
    let input = rb_packages::ProjectInput::load(root.path(), ruby()).unwrap();
    assert!(input.gemfile.is_none());
    let runtime = runtime();
    let plan = plan(
        &input.manifest,
        &runtime,
        None,
        &index,
        &PlanOptions::default(),
    )
    .unwrap();
    let mut bundle = install(
        root.path(),
        &plan,
        &runtime,
        Arc::new(index),
        InstallOptions::new(&runtime.executable),
        TaskEvents::default(),
    )
    .unwrap();
    bundle.gemfile = Some(input.bundler_gemfile);
    fs::write(input.lockfile, plan.lockfile.render().unwrap()).unwrap();
    let output = std::process::Command::new(&runtime.executable)
        .envs(bundle.environment())
        .env_remove("RUBYOPT")
        .current_dir(root.path())
        .args(["-rbundler/setup", "-rteacup", "-e", "puts TEACUP"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(stdout(&output), "42\n");
}

#[test]
fn failed_replacement_preserves_existing_bundle() {
    let root = tempfile::tempdir().unwrap();
    let mut index = MemoryIndex::default();
    add_archive(
        &mut index,
        root.path(),
        "teacup",
        &[],
        &[],
        &[("lib/teacup.rb", b"TEACUP=42\n")],
    );
    let runtime = runtime();
    let initial = plan(
        &manifest("https://rubygems.org", "teacup='>= 0'"),
        &runtime,
        None,
        &index,
        &PlanOptions::default(),
    )
    .unwrap();
    let bundle = install(
        root.path(),
        &initial,
        &runtime,
        Arc::new(index),
        InstallOptions::new(&runtime.executable),
        TaskEvents::default(),
    )
    .unwrap();
    let original = fs::read(bundle.gem_home.join("gems/teacup-1.0.0/lib/teacup.rb")).unwrap();
    let mut index = MemoryIndex::default();
    add_archive(
        &mut index,
        root.path(),
        "teacup",
        &[],
        &["ext/extconf.rb"],
        &[
            ("lib/teacup.rb", b"TEACUP=99\n"),
            ("ext/extconf.rb", b"raise 'failed replacement'\n"),
        ],
    );
    let replacement = plan(
        &manifest("https://rubygems.org", "teacup='>= 0'"),
        &runtime,
        None,
        &index,
        &PlanOptions::default(),
    )
    .unwrap();
    assert!(
        install(
            root.path(),
            &replacement,
            &runtime,
            Arc::new(index),
            InstallOptions::new(&runtime.executable),
            TaskEvents::default()
        )
        .is_err()
    );
    assert_eq!(
        fs::read(bundle.gem_home.join("gems/teacup-1.0.0/lib/teacup.rb")).unwrap(),
        original
    );
}

#[test]
fn build_configuration_changes_invalidate_reuse_without_persisting_secret_values() {
    let root = tempfile::tempdir().unwrap();
    let mut index = MemoryIndex::default();
    add_archive(
        &mut index,
        root.path(),
        "teacup",
        &[],
        &[],
        &[("lib/teacup.rb", b"TEACUP=42\n")],
    );
    let runtime = runtime();
    let plan = plan(
        &manifest("https://rubygems.org", "teacup='>= 0'"),
        &runtime,
        None,
        &index,
        &PlanOptions::default(),
    )
    .unwrap();
    let index = Arc::new(index);
    let first = install(
        root.path(),
        &plan,
        &runtime,
        index.clone(),
        InstallOptions::new(&runtime.executable),
        TaskEvents::default(),
    )
    .unwrap();
    let mut options = InstallOptions::new(&runtime.executable);
    options
        .build
        .environment
        .insert("BUILD_TOKEN".into(), "fixture-secret-value".into());
    let changed = install(
        root.path(),
        &plan,
        &runtime,
        index,
        options,
        TaskEvents::default(),
    )
    .unwrap();
    assert_eq!(first.installed, 1);
    assert_eq!(changed.installed, 1);
    assert_eq!(changed.reused, 0);
    let receipt =
        fs::read_to_string(changed.gem_home.join("gems/teacup-1.0.0/installation.json")).unwrap();
    assert!(!receipt.contains("fixture-secret-value"));
}

#[test]
fn declared_ruby_lock_is_accepted_by_frozen_bundler_without_edits() {
    let root = tempfile::tempdir().unwrap();
    let mut index = MemoryIndex::default();
    add_archive(
        &mut index,
        root.path(),
        "teacup",
        &[],
        &[],
        &[("lib/teacup.rb", b"TEACUP=42\n")],
    );
    let runtime = runtime();
    let mut manifest = manifest("https://rubygems.org", "teacup='>= 0'");
    manifest.bundle.ruby = vec![">= 3.3".into()];
    let plan = plan(&manifest, &runtime, None, &index, &PlanOptions::default()).unwrap();
    fs::write(
        root.path().join("Gemfile"),
        "source 'https://rubygems.org'\nruby '>= 3.3'\ngem 'teacup'\n",
    )
    .unwrap();
    let lock = plan.lockfile.render().unwrap();
    fs::write(root.path().join("Gemfile.lock"), &lock).unwrap();
    let bundle = install(
        root.path(),
        &plan,
        &runtime,
        Arc::new(index),
        InstallOptions::new(&runtime.executable),
        TaskEvents::default(),
    )
    .unwrap();
    let output = std::process::Command::new(&runtime.executable)
        .envs(bundle.environment())
        .env_remove("RUBYOPT")
        .env("BUNDLE_FROZEN", "true")
        .env("BUNDLE_IGNORE_CONFIG", "true")
        .current_dir(root.path())
        .args(["-rbundler/setup", "-rteacup", "-e", "puts TEACUP"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(stdout(&output), "42\n");
    assert_eq!(
        fs::read_to_string(root.path().join("Gemfile.lock")).unwrap(),
        lock
    );
}

#[test]
fn ruby_output_assertions_accept_windows_line_endings() {
    let output = std::process::Command::new(ruby())
        .env_remove("RUBYOPT")
        .env_remove("RUBYLIB")
        .args(["-e", "STDOUT.binmode; print \"42\\r\\n\""])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(stdout(&output), "42\n");
}
