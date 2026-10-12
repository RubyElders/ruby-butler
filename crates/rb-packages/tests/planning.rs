mod common;
use common::*;
use rb_packages::{LockMode, MemoryIndex, PlanOptions, plan};

#[test]
fn locks_prefer_existing_versions_updates_choose_new_versions_and_removal_prunes() {
    let mut index = MemoryIndex::default();
    add(&mut index, record("tea", "1.0", &[]));
    add(&mut index, record("tea", "2.0", &[]));
    let runtime = runtime();
    let request = manifest("https://rubygems.org", "tea = '= 1.0'");
    let first = plan(&request, &runtime, None, &index, &PlanOptions::default()).unwrap();
    let request = manifest("https://rubygems.org", "tea = '>= 0'");
    let preferred = plan(
        &request,
        &runtime,
        Some(&first.lockfile),
        &index,
        &PlanOptions::default(),
    )
    .unwrap();
    assert_eq!(
        preferred.packages[0].record.package.id.version.to_string(),
        "1.0"
    );
    let updated = plan(
        &request,
        &runtime,
        Some(&first.lockfile),
        &index,
        &PlanOptions {
            mode: LockMode::Update,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        updated.packages[0].record.package.id.version.to_string(),
        "2.0"
    );
    assert!(
        plan(
            &request,
            &runtime,
            Some(&first.lockfile),
            &index,
            &PlanOptions {
                mode: LockMode::Locked,
                ..Default::default()
            }
        )
        .is_err()
    );
    let empty = manifest("https://rubygems.org", "");
    let removed = plan(
        &empty,
        &runtime,
        Some(&first.lockfile),
        &index,
        &PlanOptions::default(),
    )
    .unwrap();
    assert!(removed.packages.is_empty());
    assert_eq!(removed.lockfile.sources.len(), 1);
    assert_eq!(
        removed.lockfile.sources[0].remotes,
        ["https://rubygems.org/"]
    );
    assert!(removed.lockfile.sources[0].packages.is_empty());
    assert!(removed.lock_changed);
}

#[test]
fn excluded_groups_stay_in_lock_but_not_installation() {
    let mut index = MemoryIndex::default();
    add(&mut index, record("tea", "1.0", &[("water", ">= 1")]));
    add(&mut index, record("water", "1.0", &[]));
    add(&mut index, record("test-helper", "1.0", &[]));
    let manifest = manifest(
        "https://rubygems.org",
        "tea = '>= 0'\ntest-helper = {groups=['test']}",
    );
    let plan = plan(
        &manifest,
        &runtime(),
        None,
        &index,
        &PlanOptions {
            without: ["test".into()].into(),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(plan.packages.len(), 3);
    assert!(
        !plan
            .packages
            .iter()
            .find(|p| p.record.package.id.name == "test-helper")
            .unwrap()
            .install
    );
    assert!(
        plan.packages
            .iter()
            .filter(|p| p.record.package.id.name != "test-helper")
            .all(|p| p.install)
    );
    assert_eq!(
        rb_lockfile::Lockfile::parse(&plan.lockfile.render().unwrap()).unwrap(),
        plan.lockfile
    );
}

#[test]
fn locked_mode_keeps_original_lock_and_rejects_missing_candidates() {
    let mut index = MemoryIndex::default();
    add(&mut index, record("tea", "1.0", &[]));
    let request = manifest("https://rubygems.org", "tea = '>= 0'");
    let runtime = runtime();
    let first = plan(&request, &runtime, None, &index, &PlanOptions::default()).unwrap();
    let options = PlanOptions {
        mode: LockMode::Locked,
        ..Default::default()
    };
    let locked = plan(&request, &runtime, Some(&first.lockfile), &index, &options).unwrap();
    assert_eq!(locked.lockfile, first.lockfile);
    assert!(!locked.lock_changed);
    assert!(
        plan(
            &request,
            &runtime,
            Some(&first.lockfile),
            &MemoryIndex::default(),
            &options
        )
        .is_err()
    );
}

#[test]
fn source_ambiguity_never_substitutes_package_provenance() {
    let mut index = MemoryIndex::default();
    let mut first = record("tea", "1.0", &[]);
    first.source = "https://one.example/".into();
    let mut second = first.clone();
    second.source = "https://two.example/".into();
    add(&mut index, first);
    add(&mut index, second);
    let request = manifest("https://one.example", "tea = '>= 0'");
    let result = plan(&request, &runtime(), None, &index, &PlanOptions::default()).unwrap();
    assert_eq!(
        result.packages[0].record.source,
        "https://one.example/".into()
    );
}

#[test]
fn locked_source_and_checksum_are_enforced_for_transitive_packages() {
    let mut index = MemoryIndex::default();
    add(&mut index, record("tea", "1.0", &[("water", ">= 1")]));
    add(&mut index, record("water", "1.0", &[]));
    let runtime = runtime();
    let manifest = manifest("https://rubygems.org", "tea='>= 0'");
    let first = plan(&manifest, &runtime, None, &index, &PlanOptions::default()).unwrap();
    let options = PlanOptions {
        mode: LockMode::Locked,
        ..Default::default()
    };
    index.packages.get_mut("water").unwrap()[0].source = "https://other.example/".into();
    assert!(plan(&manifest, &runtime, Some(&first.lockfile), &index, &options).is_err());
    index.packages.get_mut("water").unwrap()[0].source = "https://rubygems.org/".into();
    index.packages.get_mut("water").unwrap()[0].checksum = "b".repeat(64);
    assert!(plan(&manifest, &runtime, Some(&first.lockfile), &index, &options).is_err());
}

#[test]
fn valid_multi_platform_lock_is_preserved_without_dropping_other_targets() {
    let mut index = MemoryIndex::default();
    add(&mut index, record("tea", "1.0", &[]));
    let runtime = runtime();
    let manifest = manifest("https://rubygems.org", "tea='>= 0'");
    let mut first = plan(&manifest, &runtime, None, &index, &PlanOptions::default()).unwrap();
    first.lockfile.platforms.push("arm64-darwin-24".into());
    let result = plan(
        &manifest,
        &runtime,
        Some(&first.lockfile),
        &index,
        &PlanOptions::default(),
    )
    .unwrap();
    assert_eq!(result.lockfile, first.lockfile);
    assert!(!result.lock_changed);
}

#[test]
fn preferred_lock_does_not_silently_accept_changed_archive_checksums() {
    let mut index = MemoryIndex::default();
    add(&mut index, record("tea", "1.0", &[]));
    let runtime = runtime();
    let manifest = manifest("https://rubygems.org", "tea='>= 0'");
    let original = plan(&manifest, &runtime, None, &index, &PlanOptions::default()).unwrap();
    index.packages.get_mut("tea").unwrap()[0].checksum = "b".repeat(64);
    let error = plan(
        &manifest,
        &runtime,
        Some(&original.lockfile),
        &index,
        &PlanOptions::default(),
    )
    .unwrap_err();
    assert!(format!("{error:#}").contains("Locked checksum changed"));
}

#[test]
fn declared_ruby_and_engine_are_checked_and_recorded_in_lock() {
    let mut runtime = runtime();
    runtime.version = "3.4.10".parse().unwrap();
    runtime.engine_version = runtime.version.clone();
    runtime.patchlevel = 1;
    let mut manifest = manifest("https://rubygems.org", "");
    manifest.bundle.ruby = vec![">= 3.3".into()];
    let result = plan(
        &manifest,
        &runtime,
        None,
        &MemoryIndex::default(),
        &PlanOptions::default(),
    )
    .unwrap();
    assert_eq!(
        result.lockfile.ruby_version.as_deref(),
        Some("ruby 3.4.10p1")
    );
    manifest.bundle.engine_version = Some("3.3.7".into());
    assert!(
        plan(
            &manifest,
            &runtime,
            None,
            &MemoryIndex::default(),
            &PlanOptions::default()
        )
        .is_err()
    );
    manifest.bundle.engine_version = None;
    manifest.bundle.patchlevel = Some("0".into());
    assert!(
        plan(
            &manifest,
            &runtime,
            None,
            &MemoryIndex::default(),
            &PlanOptions::default()
        )
        .is_err()
    );
}

#[test]
fn archive_names_from_custom_providers_cannot_escape_installation() {
    let mut index = MemoryIndex::default();
    let mut package = record("tea", "1.0", &[]);
    package.archive_version = "1.0/../../outside".into();
    add(&mut index, package);
    assert!(
        plan(
            &manifest("https://rubygems.org", "tea='>= 0'"),
            &runtime(),
            None,
            &index,
            &PlanOptions::default()
        )
        .is_err()
    );
}

#[test]
fn platform_and_major_version_selectors_choose_only_applicable_roots() {
    let mut index = MemoryIndex::default();
    add(&mut index, record("tea", "1.0", &[]));
    add(&mut index, record("windows-helper", "1.0", &[]));
    let request = manifest(
        "https://rubygems.org",
        "tea={platforms=['ruby_3']}\nwindows-helper={platforms=['mswin']}",
    );
    let mut runtime = runtime();
    runtime.version = "3.4.10".parse().unwrap();
    runtime.platform = "x86_64-linux".into();
    let unix = plan(&request, &runtime, None, &index, &PlanOptions::default()).unwrap();
    assert_eq!(unix.packages[0].record.package.id.name, "tea");
    assert_eq!(unix.packages.len(), 1);
    runtime.platform = "x64-mingw-ucrt".into();
    let windows = plan(&request, &runtime, None, &index, &PlanOptions::default()).unwrap();
    assert_eq!(windows.packages[0].record.package.id.name, "windows-helper");
    assert_eq!(windows.packages.len(), 1);
}
