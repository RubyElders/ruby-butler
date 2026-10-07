use rb_lockfile::Lockfile;

#[test]
fn rendering_is_independent_of_source_package_and_platform_order() {
    let lock = Lockfile::parse(include_str!("fixtures/mixed.lock")).unwrap();
    let expected = lock.render().unwrap();
    let mut reverse = lock.clone();
    reverse.sources.reverse();
    reverse.platforms.reverse();
    for source in &mut reverse.sources {
        source.packages.reverse();
    }
    assert_eq!(reverse.render().unwrap(), expected);
}

#[test]
fn checksum_presence_and_absence_are_distinct() {
    let lock = Lockfile::parse(include_str!("fixtures/whatweb.lock")).unwrap();
    assert!(lock.checksums.is_none());
    assert!(!lock.render().unwrap().contains("CHECKSUMS"));
    let mut lock = lock;
    lock.checksums = Some(Default::default());
    assert!(lock.render().unwrap().contains("CHECKSUMS"));
}

#[test]
fn repeated_source_headers_have_deterministic_package_group_order() {
    let mut lock = Lockfile::parse(include_str!("fixtures/whatweb.lock")).unwrap();
    let mut second = lock.sources[0].clone();
    second.packages = lock.sources[0].packages.split_off(2);
    lock.sources.push(second);
    let expected = lock.render().unwrap();
    lock.sources.reverse();
    assert_eq!(lock.render().unwrap(), expected);
}
