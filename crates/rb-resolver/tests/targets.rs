mod common;
use common::{names, package, request};
use rb_gem_types::Platform;
use rb_resolver::{InMemoryIndex, Package, ResolveError, resolve_targets};

fn native(name: &str, version: &str, platform: &str, dependencies: &[(&str, &str)]) -> Package {
    let mut candidate = package(name, version, dependencies);
    candidate.id.platform = Platform::parse(platform);
    candidate
}

#[test]
fn targets_backtrack_to_a_shared_release_and_keep_native_dependency_closures() {
    let records = [
        native("native", "3", "x86_64-linux", &[("shared", "= 3")]),
        native("native", "2", "x86_64-linux", &[("shared", "= 2")]),
        native("native", "2", "arm64-darwin", &[("shared", "= 1")]),
        native("native", "1", "x86_64-linux", &[("shared", "= 1")]),
        native(
            "native",
            "1",
            "arm64-darwin",
            &[("shared", "= 1"), ("darwin-helper", ">= 0")],
        ),
        package("shared", "1", &[]),
        package("shared", "2", &[]),
        package("shared", "3", &[]),
        package("darwin-helper", "1", &[]),
    ];
    let linux = request(&[("native", ">= 0")]);
    let mut darwin = linux.clone();
    darwin.target = Platform::parse("arm64-darwin");
    for candidates in [records.to_vec(), records.into_iter().rev().collect()] {
        let result = resolve_targets(
            &InMemoryIndex::new(candidates),
            &[linux.clone(), darwin.clone()],
        )
        .unwrap();
        assert_eq!(result.len(), 2);
        for packages in &result {
            assert!(
                names(packages.clone())
                    .values()
                    .all(|version| version == "1")
            );
        }
        assert_eq!(result[0].len(), 2);
        assert_eq!(result[1].len(), 3);
        assert_eq!(result[0][0].id.platform.to_string(), "x86_64-linux");
        assert_eq!(result[1][0].id.name, "darwin-helper");
        assert_eq!(result[1][1].id.platform.to_string(), "arm64-darwin");
    }
}

#[test]
fn incompatible_target_roots_fail_instead_of_locking_unrelated_versions() {
    let index = InMemoryIndex::new([package("shared", "1", &[]), package("shared", "2", &[])]);
    let linux = request(&[("shared", "= 1")]);
    let mut darwin = request(&[("shared", "= 2")]);
    darwin.target = Platform::parse("arm64-darwin");
    let error = resolve_targets(&index, &[linux, darwin]).unwrap_err();
    assert!(matches!(error, ResolveError::NoSolution(_)));
    let message = error.to_string();
    assert!(message.contains("shared"));
    assert!(message.contains("arm64-darwin"));
    assert!(message.contains("x86_64-linux"));
}

#[test]
fn force_ruby_is_local_to_each_target_and_empty_requests_do_not_resolve_packages() {
    let index = InMemoryIndex::new([
        package("native", "1", &[]),
        native("native", "1", "x86_64-linux", &[("linux-helper", ">= 0")]),
        native("native", "1", "arm64-darwin", &[("darwin-helper", ">= 0")]),
        package("linux-helper", "1", &[]),
        package("darwin-helper", "1", &[]),
    ]);
    let mut linux = request(&[("native", ">= 0")]);
    linux.force_ruby.insert("native".into());
    let mut darwin = request(&[("native", ">= 0")]);
    darwin.target = Platform::parse("arm64-darwin");
    let result = resolve_targets(&index, &[linux, darwin]).unwrap();
    assert_eq!(result[0].len(), 1);
    assert_eq!(result[0][0].id.platform.os, "ruby");
    assert_eq!(result[1].len(), 2);
    assert_eq!(result[1][0].id.name, "darwin-helper");
    assert!(resolve_targets(&index, &[]).unwrap().is_empty());
}

#[test]
fn equivalent_versions_share_a_requirement_without_merging_target_archives() {
    let index = InMemoryIndex::new([
        native("native", "2.0", "x86_64-linux", &[]),
        native("native", "2.0.0", "arm64-darwin", &[]),
    ]);
    let linux = request(&[("native", "= 2")]);
    let mut darwin = linux.clone();
    darwin.target = Platform::parse("arm64-darwin");
    let result = resolve_targets(&index, &[linux, darwin]).unwrap();
    assert_eq!(result[0][0].id.version, result[1][0].id.version);
    assert_eq!(result[0][0].id.full_name(), "native-2.0-x86_64-linux");
    assert_eq!(result[1][0].id.full_name(), "native-2.0.0-arm64-darwin");
}
