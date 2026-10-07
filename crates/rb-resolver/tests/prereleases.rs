pub mod common;
use common::{package, request};
use rb_gem_types::Platform;
use rb_resolver::{InMemoryIndex, ResolveError, resolve};

#[test]
fn stable_versions_are_preferred_unless_prereleases_are_enabled_or_requested() {
    let index = InMemoryIndex::new([package("app", "1", &[]), package("app", "2.pre", &[])]);
    let mut input = request(&[("app", ">= 0")]);
    assert_eq!(
        resolve(&index, &input).unwrap()[0].id.version.to_string(),
        "1"
    );
    input.allow_prereleases = true;
    assert_eq!(
        resolve(&index, &input).unwrap()[0].id.version.to_string(),
        "2.pre"
    );
    input.allow_prereleases = false;
    input
        .roots
        .insert("app".into(), ">= 2.pre".parse().unwrap());
    assert_eq!(
        resolve(&index, &input).unwrap()[0].id.version.to_string(),
        "2.pre"
    );
}

#[test]
fn prerelease_only_candidates_are_usable_and_incompatible_stable_versions_do_not_block_them() {
    let mut foreign = package("app", "1", &[]);
    foreign.id.platform = Platform::parse("java");
    let mut future = package("app", "3", &[]);
    future.ruby = ">= 4".parse().unwrap();
    let index = InMemoryIndex::new([foreign, future, package("app", "2.pre", &[])]);
    assert_eq!(
        resolve(&index, &request(&[("app", ">= 0")])).unwrap()[0]
            .id
            .version
            .to_string(),
        "2.pre"
    );
}

#[test]
fn a_compatible_locked_prerelease_is_retained_when_stable_versions_exist() {
    let pre = package("app", "2.pre", &[]);
    let mut input = request(&[("app", ">= 0")]);
    input.locked.push(pre.id.clone());
    assert_eq!(
        resolve(&InMemoryIndex::new([pre, package("app", "1", &[])]), &input).unwrap()[0]
            .id
            .version
            .to_string(),
        "2.pre"
    );
}

#[test]
fn transitive_prerelease_requirements_and_intersections_are_respected() {
    let index = InMemoryIndex::new([
        package("app", "1", &[("shared", ">= 2.pre")]),
        package("shared", "2.pre", &[]),
        package("shared", "1", &[]),
    ]);
    assert_eq!(
        resolve(&index, &request(&[("app", ">= 0")])).unwrap()[1]
            .id
            .version
            .to_string(),
        "2.pre"
    );
    assert!(matches!(
        resolve(&index, &request(&[("app", ">= 0"), ("shared", "< 2.pre")])),
        Err(ResolveError::NoSolution(_))
    ));
}

#[test]
fn a_locked_prerelease_can_move_to_a_compatible_platform() {
    let mut locked = package("app", "2.pre", &[]).id;
    locked.platform = Platform::parse("java");
    let mut input = request(&[("app", ">= 0")]);
    input.locked.push(locked);
    let index = InMemoryIndex::new([package("app", "1", &[]), package("app", "2.pre", &[])]);
    let result = resolve(&index, &input).unwrap();
    assert_eq!(result[0].id.version.to_string(), "2.pre");
    assert_eq!(result[0].id.platform.to_string(), "ruby");
}
