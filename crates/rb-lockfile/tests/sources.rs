use rb_lockfile::{Lockfile, SourceKind};

#[test]
fn mixed_sources_preserve_variants_pins_checksums_and_engine_text() {
    let lock = Lockfile::parse(include_str!("fixtures/mixed.lock")).unwrap();
    assert_eq!(lock.sources.len(), 4);
    assert_eq!(lock.sources[0].kind, SourceKind::Git);
    assert_eq!(lock.sources[0].options["submodules"], "true");
    assert_eq!(lock.sources[1].kind, SourceKind::Path);
    assert_eq!(lock.sources[1].remotes, ["../local"]);
    assert!(lock.dependencies["local"].pinned);
    assert!(!lock.dependencies["ffi"].pinned);
    let variants: Vec<_> = lock.sources[2]
        .packages
        .iter()
        .filter(|p| p.name == "ffi")
        .collect();
    assert_eq!(variants.len(), 2);
    assert_eq!(
        variants[0].dependencies["generic_dep"][0].to_string(),
        ">= 1"
    );
    assert_eq!(
        variants[1].dependencies["native_dep"][0].to_string(),
        ">= 2"
    );
    let checksums = lock.checksums.as_ref().unwrap();
    assert_eq!(checksums["ffi (1.0.0)"], Some("b".repeat(64)));
    assert_eq!(checksums["ffi (1.0.0-arm64-darwin)"], Some("c".repeat(64)));
    assert_eq!(checksums["local (0.1.0)"], None);
    assert_eq!(
        lock.ruby_version.as_deref(),
        Some("ruby 3.1.2p20 (jruby 9.4.0.0)")
    );
    assert_eq!(lock.bundled_with.unwrap().to_string(), "2.6.9");
}

#[test]
fn historical_multi_remote_registry_sources_are_preserved() {
    let text = include_str!("fixtures/whatweb.lock").replace(
        "  remote: https://rubygems.org/",
        "  remote: https://rubygems.org/\n  remote: https://other.example/",
    );
    let lock = Lockfile::parse(&text).unwrap();
    assert_eq!(lock.sources[0].remotes.len(), 2);
    assert_eq!(
        Lockfile::parse(&lock.render().unwrap()).unwrap().sources[0].remotes,
        lock.sources[0].remotes
    );
}

#[test]
fn custom_platform_spelling_is_not_normalized_away() {
    let text = include_str!("fixtures/whatweb.lock").replace(
        "addressable (2.9.0)",
        "addressable (2.9.0-future-custom-os)",
    );
    let lock = Lockfile::parse(&text).unwrap();
    assert_eq!(lock.sources[0].packages[0].platform, "future-custom-os");
    assert!(
        lock.render()
            .unwrap()
            .contains("addressable (2.9.0-future-custom-os)")
    );
}

#[test]
fn git_and_path_only_locks_do_not_require_a_registry() {
    let mut lock = Lockfile::parse(include_str!("fixtures/mixed.lock")).unwrap();
    lock.sources
        .retain(|source| source.kind != SourceKind::Registry);
    lock.dependencies
        .retain(|name, _| name == "widget" || name == "local");
    lock.checksums = None;
    let rendered = lock.render().unwrap();
    assert!(!rendered.lines().any(|line| line == "GEM"));
    assert_eq!(Lockfile::parse(&rendered).unwrap().sources.len(), 2);
}
