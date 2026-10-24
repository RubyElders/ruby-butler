mod common;
use common::*;
use rb_packages::{LockMode, MemoryIndex, PackageIndex, PackageRecord, PlanOptions, plan};
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

struct LockedIndex {
    records: MemoryIndex,
    loads: AtomicUsize,
}

impl PackageIndex for LockedIndex {
    fn candidates(&self, _: &str) -> anyhow::Result<Vec<PackageRecord>> {
        anyhow::bail!("Candidate discovery must not run for an unchanged lock")
    }
    fn archive(&self, _: &PackageRecord) -> anyhow::Result<PathBuf> {
        anyhow::bail!("Planning must not download archives")
    }
    fn locked_records(&self, packages: &[PackageRecord]) -> anyhow::Result<Vec<PackageRecord>> {
        self.loads.fetch_add(1, Ordering::Relaxed);
        self.records.locked_records(packages)
    }
}

#[test]
fn unchanged_lock_loads_selected_metadata_without_candidate_discovery() {
    let runtime = runtime();
    let mut records = MemoryIndex::default();
    add(&mut records, record("tea", "1.0", &[("water", ">= 1")]));
    add(&mut records, record("water", "1.0", &[]));
    let manifest = manifest("https://rubygems.org", "tea='>= 0'");
    let first = plan(&manifest, &runtime, None, &records, &PlanOptions::default()).unwrap();
    let index = LockedIndex {
        records,
        loads: AtomicUsize::new(0),
    };
    for mode in [LockMode::Prefer, LockMode::Locked] {
        let result = plan(
            &manifest,
            &runtime,
            Some(&first.lockfile),
            &index,
            &PlanOptions {
                mode,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(result.lockfile, first.lockfile);
        assert!(!result.lock_changed);
        assert_eq!(result.packages.len(), 2);
        assert_eq!(result.packages[0].record.checksum, "a".repeat(64));
    }
    assert_eq!(index.loads.load(Ordering::Relaxed), 2);
}

#[test]
fn unchanged_locks_without_checksums_remain_unchanged() {
    let runtime = runtime();
    let mut records = MemoryIndex::default();
    add(&mut records, record("tea", "1.0", &[]));
    let manifest = manifest("https://rubygems.org", "tea='>= 0'");
    let mut first = plan(&manifest, &runtime, None, &records, &PlanOptions::default()).unwrap();
    first.lockfile.checksums = None;
    let index = LockedIndex {
        records,
        loads: AtomicUsize::new(0),
    };
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
    assert_eq!(result.packages[0].record.checksum, "a".repeat(64));
}

#[test]
fn implicit_bundler_dependency_uses_runtime_version_without_rewriting_lock() {
    let runtime = runtime();
    let version = runtime.bundler.as_ref().unwrap().to_string();
    let mut records = MemoryIndex::default();
    add(&mut records, record("tea", "1.0", &[("bundler", ">= 1")]));
    add(&mut records, record("bundler", &version, &[]));
    let manifest = manifest("https://rubygems.org", "tea='>= 0'");
    let mut first = plan(&manifest, &runtime, None, &records, &PlanOptions::default()).unwrap();
    for source in &mut first.lockfile.sources {
        source.packages.retain(|package| package.name != "bundler");
    }
    first.lockfile.checksums = None;
    let index = LockedIndex {
        records,
        loads: AtomicUsize::new(0),
    };
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
    assert!(
        result
            .packages
            .iter()
            .any(|p| p.record.package.id.name == "bundler"
                && p.record.package.id.version.to_string() == version)
    );
}

#[test]
fn changed_locked_metadata_or_ruby_compatibility_is_not_trusted() {
    let runtime = runtime();
    let mut records = MemoryIndex::default();
    add(&mut records, record("tea", "1.0", &[]));
    let manifest = manifest("https://rubygems.org", "tea='>= 0'");
    let first = plan(&manifest, &runtime, None, &records, &PlanOptions::default()).unwrap();
    let frozen = PlanOptions {
        mode: LockMode::Locked,
        ..Default::default()
    };
    let mut changed = records.packages["tea"][0].clone();
    changed.package.ruby = ">= 99".parse().unwrap();
    records.packages.insert("tea".into(), vec![changed]);
    assert!(
        plan(
            &manifest,
            &runtime,
            Some(&first.lockfile),
            &records,
            &frozen
        )
        .is_err()
    );
    let mut changed = record("tea", "1.0", &[("water", ">= 1")]);
    changed.rubygems = Some(">= 99".parse().unwrap());
    records.packages.insert("tea".into(), vec![changed.clone()]);
    assert!(
        plan(
            &manifest,
            &runtime,
            Some(&first.lockfile),
            &records,
            &frozen
        )
        .is_err()
    );
    changed.rubygems = None;
    records.packages.insert("tea".into(), vec![changed]);
    assert!(
        plan(
            &manifest,
            &runtime,
            Some(&first.lockfile),
            &records,
            &frozen
        )
        .is_err()
    );
    add(&mut records, record("water", "1.0", &[]));
    let updated = plan(
        &manifest,
        &runtime,
        Some(&first.lockfile),
        &records,
        &PlanOptions::default(),
    )
    .unwrap();
    assert!(updated.lock_changed);
    assert_eq!(updated.packages.len(), 2);
    assert_eq!(
        rb_lockfile::Lockfile::parse(&updated.lockfile.render().unwrap()).unwrap(),
        updated.lockfile
    );
}

#[test]
fn locked_checksums_keep_the_archives_original_platform_spelling() {
    let mut runtime = runtime();
    runtime.platform = "i686-linux".into();
    let mut records = MemoryIndex::default();
    let mut package = record("tea", "1.0", &[]);
    package.package.id.platform = rb_gem_types::Platform::parse("i686-linux");
    package.archive_version = "1.0-i686-linux".into();
    add(&mut records, package);
    let manifest = manifest("https://rubygems.org", "tea='>= 0'");
    let first = plan(&manifest, &runtime, None, &records, &PlanOptions::default()).unwrap();
    records.packages.get_mut("tea").unwrap()[0].checksum = "b".repeat(64);
    let error = plan(
        &manifest,
        &runtime,
        Some(&first.lockfile),
        &records,
        &PlanOptions {
            mode: LockMode::Locked,
            ..Default::default()
        },
    )
    .unwrap_err();
    assert!(format!("{error:#}").contains("Locked checksum changed"));
}
