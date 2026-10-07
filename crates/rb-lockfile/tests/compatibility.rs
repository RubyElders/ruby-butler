use rb_lockfile::Lockfile;

#[test]
fn legacy_repeated_dependencies_preserve_every_declaration() {
    let lock = Lockfile::parse(include_str!("fixtures/legacy-dependencies.lock")).unwrap();
    let rendered = lock.render().unwrap();
    assert!(rendered.contains("      polyglot\n      polyglot (>= 0.3.1)\n"));
    assert_eq!(
        Lockfile::parse(&rendered).unwrap().render().unwrap(),
        rendered
    );
}

#[test]
fn implicit_bundler_checksum_is_preserved_without_creating_a_spec() {
    let lock = Lockfile::parse(include_str!("fixtures/implicit-bundler-checksum.lock")).unwrap();
    assert_eq!(lock.sources[0].packages.len(), 1);
    assert_eq!(
        lock.checksums.as_ref().unwrap()["bundler (4.0.18)"],
        Some("a".repeat(64))
    );
    let rendered = lock.render().unwrap();
    assert!(rendered.contains("  bundler (4.0.18) sha256="));
    assert!(!rendered.contains("    bundler (4.0.18)"));
    assert_eq!(Lockfile::parse(&rendered).unwrap(), lock);
}

#[test]
fn registry_sections_without_remotes_are_preserved() {
    let lock = Lockfile::parse(include_str!("fixtures/no-remote.lock")).unwrap();
    assert_eq!(lock.sources.len(), 2);
    assert!(lock.sources.iter().all(|source| source.remotes.is_empty()));
    assert_eq!(lock.sources[1].packages[0].name, "rack");
    assert_eq!(
        Lockfile::parse(&lock.render().unwrap())
            .unwrap()
            .sources
            .len(),
        2
    );
}

#[test]
fn implicit_bundler_checksums_must_match_the_environment_version() {
    let text = include_str!("fixtures/implicit-bundler-checksum.lock");
    let wrong_version = text.replace("bundler (4.0.18)", "bundler (4.0.17)");
    assert!(Lockfile::parse(&wrong_version).is_err());
    let missing_version = text.split("BUNDLED WITH").next().unwrap();
    assert!(Lockfile::parse(missing_version).is_err());
}

#[test]
fn git_and_path_sources_still_require_a_remote() {
    let lock = Lockfile::parse(include_str!("fixtures/mixed.lock")).unwrap();
    for index in [0, 1] {
        let mut invalid = lock.clone();
        invalid.sources[index].remotes.clear();
        assert!(invalid.render().is_err());
    }
}
