mod common;
use common::{names, package, request};
use rb_gem_types::Platform;
use rb_resolver::{InMemoryIndex, ResolveError, resolve};
use std::collections::BTreeMap;

#[test]
fn picks_newest_candidates_and_returns_sorted_reachable_packages() {
    let index = InMemoryIndex::new([
        package("rack", "1.10", &[]),
        package("app", "1", &[("rack", ">= 1")]),
        package("rack", "1.9", &[]),
        package("unused", "99", &[]),
    ]);
    let result = resolve(&index, &request(&[("app", ">= 0")])).unwrap();
    assert_eq!(
        result
            .iter()
            .map(|p| p.id.name.as_str())
            .collect::<Vec<_>>(),
        ["app", "rack"]
    );
    assert_eq!(result[1].id.version.to_string(), "1.10");
}

#[test]
fn backtracks_when_latest_root_conflicts_with_a_transitive_requirement() {
    let index = InMemoryIndex::new([
        package("app", "2", &[("shared", "~> 2")]),
        package("app", "1", &[("shared", "~> 1")]),
        package("other", "1", &[("shared", "< 2")]),
        package("shared", "2", &[]),
        package("shared", "1", &[]),
    ]);
    assert_eq!(
        names(resolve(&index, &request(&[("app", ">= 0"), ("other", ">= 0")])).unwrap()),
        BTreeMap::from([
            ("app".into(), "1".into()),
            ("other".into(), "1".into()),
            ("shared".into(), "1".into())
        ])
    );
}

#[test]
fn explains_conflicting_roots_and_missing_packages() {
    let index = InMemoryIndex::new([
        package("app", "1", &[("shared", "< 2")]),
        package("other", "1", &[("shared", ">= 2")]),
        package("shared", "1", &[]),
        package("shared", "2", &[]),
    ]);
    let error = resolve(&index, &request(&[("app", "= 1"), ("other", "= 1")])).unwrap_err();
    assert!(matches!(error, ResolveError::NoSolution(_)));
    let message = error.to_string();
    for detail in ["app", "other", "shared", "3.4", "x86_64-linux-gnu"] {
        assert!(message.contains(detail), "{message}");
    }
    let error = resolve(&index, &request(&[("missing", "= 9")])).unwrap_err();
    assert!(error.to_string().contains("missing (= 9)"));
}

#[test]
fn empty_roots_select_no_packages() {
    assert!(
        resolve(&InMemoryIndex::default(), &request(&[]))
            .unwrap()
            .is_empty()
    );
}

#[test]
fn ruby_and_platform_constraints_exclude_incompatible_versions() {
    let mut latest = package("app", "3", &[]);
    latest.ruby = ">= 4".parse().unwrap();
    let mut foreign = package("app", "2", &[]);
    foreign.id.platform = Platform::parse("java");
    let index = InMemoryIndex::new([latest, foreign, package("app", "1", &[])]);
    assert_eq!(
        resolve(&index, &request(&[("app", ">= 0")])).unwrap()[0]
            .id
            .version
            .to_string(),
        "1"
    );
    assert!(matches!(
        resolve(&index, &request(&[("app", ">= 2")])),
        Err(ResolveError::NoSolution(_))
    ));
}

#[test]
fn native_and_pure_variants_have_distinct_dependencies() {
    let pure = package("app", "1", &[("build-helper", ">= 0")]);
    let mut native = package("app", "1", &[("native-helper", ">= 0")]);
    native.id.platform = Platform::parse("x86_64-linux-gnu");
    let index = InMemoryIndex::new([
        pure,
        native,
        package("build-helper", "1", &[]),
        package("native-helper", "1", &[]),
    ]);
    let result = resolve(&index, &request(&[("app", ">= 0")])).unwrap();
    assert_eq!(result[0].id.platform.to_string(), "x86_64-linux-gnu");
    assert_eq!(
        result
            .iter()
            .map(|p| p.id.name.as_str())
            .collect::<Vec<_>>(),
        ["app", "native-helper"]
    );
}

#[test]
fn falls_back_to_pure_ruby_when_native_dependencies_cannot_resolve() {
    let mut native = package("app", "1", &[("missing", ">= 0")]);
    native.id.platform = Platform::parse("x86_64-linux-gnu");
    let index = InMemoryIndex::new([native, package("app", "1", &[])]);
    let result = resolve(&index, &request(&[("app", ">= 0")])).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].id.platform.to_string(), "ruby");
}

#[test]
fn compatible_locks_are_preferences_and_conflicting_locks_can_move() {
    let old = package("app", "1", &[("shared", "< 2")]);
    let index = InMemoryIndex::new([
        old.clone(),
        package("app", "2", &[("shared", ">= 2")]),
        package("shared", "1", &[]),
        package("shared", "2", &[]),
    ]);
    let mut input = request(&[("app", ">= 0")]);
    input.locked.push(old.id);
    assert_eq!(
        resolve(&index, &input).unwrap()[0].id.version.to_string(),
        "1"
    );
    input.roots.insert("shared".into(), ">= 2".parse().unwrap());
    assert_eq!(
        resolve(&index, &input).unwrap()[0].id.version.to_string(),
        "2"
    );
}

#[test]
fn a_lock_from_another_platform_prefers_its_version_on_the_current_target() {
    let mut input = request(&[("app", ">= 0")]);
    let mut locked = package("app", "1", &[]).id;
    locked.platform = Platform::parse("java");
    input.locked.push(locked);
    let index = InMemoryIndex::new([package("app", "2", &[]), package("app", "1", &[])]);
    assert_eq!(
        resolve(&index, &input).unwrap()[0].id.version.to_string(),
        "1"
    );
}

#[test]
fn exact_locked_platform_is_preferred_but_ruby_incompatible_locks_are_ignored() {
    let pure = package("app", "1", &[]);
    let mut native = pure.clone();
    native.id.platform = Platform::parse("x86_64-linux-gnu");
    let mut input = request(&[("app", ">= 0")]);
    input.locked.push(pure.id.clone());
    assert_eq!(
        resolve(&InMemoryIndex::new([native, pure]), &input).unwrap()[0]
            .id
            .platform
            .to_string(),
        "ruby"
    );
    let mut old = package("app", "1", &[]);
    old.ruby = "< 3".parse().unwrap();
    assert_eq!(
        resolve(&InMemoryIndex::new([old, package("app", "2", &[])]), &input).unwrap()[0]
            .id
            .version
            .to_string(),
        "2"
    );
}

#[test]
fn fixed_candidate_sets_cannot_fetch_missing_transitive_packages() {
    let index = InMemoryIndex::new([package("app", "1", &[("missing", ">= 0")])]);
    assert!(matches!(
        resolve(&index, &request(&[("app", ">= 0")])),
        Err(ResolveError::NoSolution(_))
    ));
}

#[test]
fn cyclic_and_self_dependencies_resolve_without_recursion() {
    let index = InMemoryIndex::new([
        package("a", "1", &[("b", "= 1"), ("a", "= 1")]),
        package("b", "1", &[("a", "= 1")]),
    ]);
    assert_eq!(
        names(resolve(&index, &request(&[("a", ">= 0")])).unwrap()),
        BTreeMap::from([("a".into(), "1".into()), ("b".into(), "1".into())])
    );
}

#[test]
fn a_gem_named_like_the_synthetic_root_remains_a_real_package() {
    let index = InMemoryIndex::new([package("<project>", "7", &[])]);
    let result = resolve(&index, &request(&[("<project>", "= 7")])).unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].id.full_name(), "<project>-7");
}

#[test]
fn pessimistic_requirements_exclude_the_next_release_prerelease() {
    let mut input = request(&[("app", "~> 1.2")]);
    input.allow_prereleases = true;
    let index = InMemoryIndex::new([package("app", "2.0.pre", &[]), package("app", "1.9", &[])]);
    assert_eq!(
        resolve(&index, &input).unwrap()[0].id.version.to_string(),
        "1.9"
    );
}
