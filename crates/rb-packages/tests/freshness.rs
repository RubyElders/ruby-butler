mod common;
use common::*;
use rb_packages::{InstallOptions, MemoryIndex, PlanOptions, freshness::fingerprint, plan};

#[test]
fn resolved_identity_is_independent_of_declaration_format_order_and_operational_options() {
    let runtime = runtime();
    let mut index = MemoryIndex::default();
    add(&mut index, record("rake", "1.0.0", &[]));
    add(&mut index, record("rack", "1.0.0", &[]));
    let manifest = manifest("https://rubygems.org", "rake='>= 0'\nrack='>= 0'");
    let kdl = rb_gemfile::Manifest::from_kdl(&manifest.to_kdl().unwrap()).unwrap();
    let resolved = plan(&manifest, &runtime, None, &index, &PlanOptions::default()).unwrap();
    let options = InstallOptions::new(&runtime.executable);
    let expected = fingerprint(&resolved, &manifest, &runtime, &options).unwrap();
    let mut reordered = resolved.clone();
    reordered.packages.reverse();
    reordered.lock_changed = !resolved.lock_changed;
    let mut operational = options.clone();
    operational.workers += 1;
    operational.build.jobs += 1;
    operational.build.timeout += std::time::Duration::from_secs(1);
    assert_eq!(
        expected,
        fingerprint(&reordered, &kdl, &runtime, &operational).unwrap()
    );
    let mut compatible = manifest.clone();
    compatible.dependencies.get_mut("rake").unwrap().version = vec![">= 0.5".into()];
    assert_eq!(
        expected,
        fingerprint(&resolved, &compatible, &runtime, &options).unwrap()
    );
}

#[test]
fn resolved_identity_tracks_packages_provenance_activation_and_build_compatibility() {
    let runtime = runtime();
    let mut index = MemoryIndex::default();
    add(&mut index, record("rake", "1.0.0", &[]));
    let manifest = manifest("https://rubygems.org", "rake='>= 0'");
    let resolved = plan(&manifest, &runtime, None, &index, &PlanOptions::default()).unwrap();
    let options = InstallOptions::new(&runtime.executable);
    let expected = fingerprint(&resolved, &manifest, &runtime, &options).unwrap();
    let mut changed = resolved.clone();
    changed.packages[0].record.checksum = "b".repeat(64);
    assert_ne!(
        expected,
        fingerprint(&changed, &manifest, &runtime, &options).unwrap()
    );
    changed = resolved.clone();
    changed.packages[0].record.source = "https://other.example/".into();
    assert_ne!(
        expected,
        fingerprint(&changed, &manifest, &runtime, &options).unwrap()
    );
    changed = resolved.clone();
    changed.packages[0].record.package.id.version = "2.0.0".parse().unwrap();
    changed.packages[0].record.archive_version = "2.0.0".into();
    assert_ne!(
        expected,
        fingerprint(&changed, &manifest, &runtime, &options).unwrap()
    );
    changed = resolved.clone();
    changed.packages[0].install = false;
    assert_ne!(
        expected,
        fingerprint(&changed, &manifest, &runtime, &options).unwrap()
    );
    let mut activation = manifest.clone();
    activation.dependencies.get_mut("rake").unwrap().require =
        Some(rb_gemfile::Require::Enabled(false));
    assert_ne!(
        expected,
        fingerprint(&resolved, &activation, &runtime, &options).unwrap()
    );
    let mut build = options.clone();
    build.build.arguments.push("--with-feature".into());
    assert_ne!(
        expected,
        fingerprint(&resolved, &manifest, &runtime, &build).unwrap()
    );
    let mut ruby = runtime.clone();
    ruby.abi = "different-abi".into();
    assert_ne!(
        expected,
        fingerprint(&resolved, &manifest, &ruby, &options).unwrap()
    );
}

#[test]
fn resolved_identity_tracks_git_revisions_groups_and_platforms() {
    let runtime = runtime();
    let mut index = MemoryIndex::default();
    add(&mut index, record("rake", "1.0.0", &[]));
    let manifest = manifest("https://rubygems.org", "rake='>= 0'");
    let mut resolved = plan(&manifest, &runtime, None, &index, &PlanOptions::default()).unwrap();
    let options = InstallOptions::new(&runtime.executable);
    resolved.packages[0].record.source = rb_packages::PackageSource::Git {
        remote: "https://example.test/rake.git".into(),
        revision: "a".repeat(40),
        options: Default::default(),
    };
    let expected = fingerprint(&resolved, &manifest, &runtime, &options).unwrap();
    let mut revised = resolved.clone();
    if let rb_packages::PackageSource::Git { revision, .. } = &mut revised.packages[0].record.source
    {
        *revision = "b".repeat(40);
    }
    assert_ne!(
        expected,
        fingerprint(&revised, &manifest, &runtime, &options).unwrap()
    );
    let mut groups = manifest.clone();
    groups
        .dependencies
        .get_mut("rake")
        .unwrap()
        .groups
        .push("test".into());
    assert_ne!(
        expected,
        fingerprint(&resolved, &groups, &runtime, &options).unwrap()
    );
    let mut platforms = manifest.clone();
    platforms
        .dependencies
        .get_mut("rake")
        .unwrap()
        .platforms
        .push("ruby".into());
    assert_ne!(
        expected,
        fingerprint(&resolved, &platforms, &runtime, &options).unwrap()
    );
    let mut excluded = resolved.clone();
    excluded.without.insert("test".into());
    assert_ne!(
        expected,
        fingerprint(&excluded, &manifest, &runtime, &options).unwrap()
    );
}

#[test]
fn changing_input_format_reuses_the_same_resolved_bundle() {
    use rb_packages::{PrepareOptions, prepare};
    use rb_task::TaskEvents;
    use std::fs;
    let root = tempfile::tempdir().unwrap();
    fs::create_dir_all(root.path().join("local/lib")).unwrap();
    fs::write(root.path().join("local/local.gemspec"), "Gem::Specification.new { |s| s.name='local'; s.version='1.0.0'; s.summary='fixture'; s.authors=['Test']; s.files=['lib/local.rb'] }\n").unwrap();
    fs::write(root.path().join("local/lib/local.rb"), "LOCAL=42\n").unwrap();
    fs::write(
        root.path().join("Gemfile"),
        "gem 'local', path: './local'\n",
    )
    .unwrap();
    let options = PrepareOptions::new(root.path(), ruby(), root.path().join(".rb/cache"));
    let first = prepare(options.clone(), TaskEvents::default()).unwrap();
    assert!(first.changed);
    let mut invalid = options.clone();
    invalid.install.workers = 0;
    let error = prepare(invalid, TaskEvents::default())
        .err()
        .expect("An unchanged bundle must still reject zero workers");
    assert!(error.to_string().contains("Worker count must be positive"));
    #[cfg(unix)]
    let _socket =
        std::os::unix::net::UnixListener::bind(first.installed.gem_home.join("runtime.sock"))
            .unwrap();
    let second = prepare(options.clone(), TaskEvents::default()).unwrap();
    assert_eq!(second.installed.installed, 0);
    assert_eq!(second.installed.reused, 1);
    assert!(!second.changed);
    #[cfg(unix)]
    std::fs::remove_file(first.installed.gem_home.join("runtime.sock")).unwrap();
    assert_eq!(first.fingerprint, second.fingerprint);
    let manifest = first.input.manifest.clone();
    fs::remove_file(root.path().join("Gemfile")).unwrap();
    fs::write(
        root.path().join("rbproject.toml"),
        manifest.to_toml().unwrap(),
    )
    .unwrap();
    let toml = prepare(options.clone(), TaskEvents::default()).unwrap();
    assert_eq!(toml.fingerprint, first.fingerprint);
    assert_eq!(toml.installed.installed, 0);
    assert!(!toml.changed);
    fs::remove_file(root.path().join("rbproject.toml")).unwrap();
    fs::write(
        root.path().join("rbproject.kdl"),
        manifest.to_kdl().unwrap(),
    )
    .unwrap();
    let kdl = prepare(options, TaskEvents::default()).unwrap();
    assert_eq!(kdl.fingerprint, first.fingerprint);
    assert_eq!(kdl.installed.installed, 0);
    assert!(!kdl.changed);
}
