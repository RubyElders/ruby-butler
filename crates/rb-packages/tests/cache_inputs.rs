mod common;
use rb_packages::{PrepareOptions, inspect_cache};
use std::fs;

fn options(root: &std::path::Path) -> PrepareOptions {
    let mut options = PrepareOptions::new(root, common::ruby(), root.join("cache"));
    options.gemfile = Some(root.join("Gemfile"));
    options
}

#[test]
fn cache_tracks_loaded_inputs_and_lock_without_application_or_credentials() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("Gemfile"),
        "source 'https://rubygems.org'\neval_gemfile 'Dependencies'\n",
    )
    .unwrap();
    fs::write(root.path().join("Dependencies"), "gem 'rake', '13.0.0'\n").unwrap();
    fs::write(root.path().join("Gemfile.lock"), "fixture lock\n").unwrap();
    let mut options = options(root.path());
    let first = inspect_cache(&options).unwrap();
    assert!(first.fingerprint.is_some());
    fs::write(root.path().join("app.rb"), "unrelated\n").unwrap();
    fs::create_dir_all(root.path().join(".trees/other")).unwrap();
    fs::write(root.path().join(".trees/other/large"), "unrelated\n").unwrap();
    fs::create_dir_all(root.path().join(".rb")).unwrap();
    fs::write(
        root.path().join(".rb/config"),
        "BUNDLE_GEMS__EXAMPLE__COM: secret\n",
    )
    .unwrap();
    assert_eq!(
        first.fingerprint,
        inspect_cache(&options).unwrap().fingerprint
    );
    assert_eq!(
        first.compatibility,
        inspect_cache(&options).unwrap().compatibility
    );
    fs::write(
        root.path().join(".rb/config"),
        "BUNDLE_GEMS__EXAMPLE__COM: changed-secret\n",
    )
    .unwrap();
    assert_eq!(
        first.compatibility,
        inspect_cache(&options).unwrap().compatibility
    );
    fs::write(root.path().join("Dependencies"), "gem 'rake', '13.1.0'\n").unwrap();
    assert_ne!(
        first.fingerprint,
        inspect_cache(&options).unwrap().fingerprint
    );
    let changed = inspect_cache(&options).unwrap();
    fs::write(root.path().join("Gemfile.lock"), "changed lock\n").unwrap();
    assert_ne!(
        changed.fingerprint,
        inspect_cache(&options).unwrap().fingerprint
    );
    options
        .install
        .build
        .environment
        .insert("CFLAGS".into(), "-O0".into());
    assert_ne!(
        first.compatibility,
        inspect_cache(&options).unwrap().compatibility
    );
}

#[test]
fn incomplete_environment_or_missing_lock_cannot_have_an_exact_key() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("Gemfile"),
        "source 'https://rubygems.org'\ngem 'rake'\n",
    )
    .unwrap();
    let options = options(root.path());
    assert!(inspect_cache(&options).unwrap().fingerprint.is_none());
    fs::write(root.path().join("Gemfile.lock"), "fixture\n").unwrap();
    fs::write(
        root.path().join("Gemfile"),
        "source 'https://rubygems.org'\nENV.to_h\ngem 'rake'\n",
    )
    .unwrap();
    assert!(inspect_cache(&options).unwrap().fingerprint.is_none());
    fs::write(
        root.path().join("Gemfile"),
        "source 'https://rubygems.org'\nENV['PRIVATE_TOKEN']\ngem 'rake'\n",
    )
    .unwrap();
    assert!(inspect_cache(&options).unwrap().fingerprint.is_none());
}

#[test]
fn local_sources_track_helpers_and_payload_without_git_metadata() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("local");
    fs::create_dir_all(source.join("lib")).unwrap();
    fs::write(source.join("lib/version.rb"), "VERSION = '1.0.0'\n").unwrap();
    fs::write(source.join("fixture.gemspec"), "require_relative 'lib/version'\nGem::Specification.new { |s| s.name='fixture'; s.version=VERSION; s.summary='Fixture'; s.authors=['Fixture']; s.files=['lib/version.rb'] }\n").unwrap();
    fs::write(
        root.path().join("Gemfile"),
        "source 'https://rubygems.org'\ngem 'fixture', path: 'local'\n",
    )
    .unwrap();
    fs::write(root.path().join("Gemfile.lock"), "fixture\n").unwrap();
    let options = options(root.path());
    let first = inspect_cache(&options).unwrap().fingerprint.unwrap();
    fs::create_dir(source.join(".git")).unwrap();
    fs::write(source.join(".git/config"), "secret\n").unwrap();
    assert_eq!(
        Some(first.clone()),
        inspect_cache(&options).unwrap().fingerprint
    );
    fs::write(source.join("lib/version.rb"), "VERSION = '1.1.0'\n").unwrap();
    assert_ne!(Some(first), inspect_cache(&options).unwrap().fingerprint);
}

#[test]
fn native_project_formats_report_the_generated_lock_and_complete_fingerprints() {
    let manifest: rb_gemfile::Manifest =
        toml::from_str("[bundle]\nsource='https://rubygems.org'\n[dependencies]\nrake='13.3.1'\n")
            .unwrap();
    for (name, text) in [
        ("rbproject.toml", manifest.to_toml().unwrap()),
        ("rbproject.kdl", manifest.to_kdl().unwrap()),
    ] {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join(name), text).unwrap();
        let options = PrepareOptions::new(root.path(), common::ruby(), root.path().join("cache"));
        let before = inspect_cache(&options).unwrap();
        assert!(!before.complete);
        assert!(before.fingerprint.is_none());
        fs::write(&before.lockfile, "fixture lock\n").unwrap();
        let after = inspect_cache(&options).unwrap();
        assert!(after.complete);
        assert_eq!(after.fingerprint.unwrap().len(), 64);
    }
}
