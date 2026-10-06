use rb_gem_types::{Operator, PackageId, Platform, Requirement, Version};
use std::collections::{BTreeSet, HashSet};

fn version(value: &str) -> Version {
    value.parse().unwrap()
}

#[test]
fn equal_versions_have_equal_hashes_and_ordering() {
    for equivalents in [
        vec!["1", "1.0", "1.0.0", "01.00"],
        vec!["1.a", "1.0.a", "1.0.a.0"],
        vec!["1-rc1", "1.pre.rc.1"],
        vec!["", "0", "0.0"],
    ] {
        let values: Vec<_> = equivalents.iter().map(|value| version(value)).collect();
        assert_eq!(values.iter().cloned().collect::<HashSet<_>>().len(), 1);
        assert_eq!(values.into_iter().collect::<BTreeSet<_>>().len(), 1);
    }
}

#[test]
fn large_numeric_segments_order_and_bump_without_overflow() {
    let digits = "9".repeat(1000);
    let next = format!("1{}", "0".repeat(1000));
    assert_eq!(version(&digits).bump().to_string(), next);
    assert!(version(&digits) < version(&next));
    assert!(version("18446744073709551615") < version("18446744073709551616"));
}

#[test]
fn pessimistic_upper_bound_uses_the_release_version() {
    let requirement: Requirement = "~> 1.2".parse().unwrap();
    assert!(requirement.matches(&version("1.9.pre")));
    assert!(!requirement.matches(&version("2.0.pre")));
    let narrow: Requirement = "~> 1.2.0".parse().unwrap();
    assert!(!narrow.matches(&version("1.3.pre")));
    assert!(!requirement.prerelease());
    assert!("~> 1.2.rc1".parse::<Requirement>().unwrap().prerelease());
}

#[test]
fn requirements_preserve_intersections_and_exclusions() {
    let constraint: Requirement = ">= 1&< 2, != 1.5".parse().unwrap();
    assert!(constraint.matches(&version("1.4")));
    assert!(!constraint.matches(&version("1.5")));
    assert!(!constraint.matches(&version("2")));
    assert_eq!(constraint.to_string(), ">= 1, < 2, != 1.5");
    assert!(
        !">= 2, < 1"
            .parse::<Requirement>()
            .unwrap()
            .matches(&version("1.5"))
    );
    assert!(Requirement::default().matches(&version("0")));
}

#[test]
fn platform_matches_are_directional_and_scores_prefer_native() {
    let target = Platform::parse("x86_64-linux-musl");
    let native = Platform::parse("x86_64-linux-musl");
    let generic = Platform::parse("x86_64-linux");
    let pure = Platform::parse("ruby");
    assert!(generic.matches(&target));
    assert!(!target.matches(&generic));
    assert!(native.score(&target) < generic.score(&target));
    assert!(generic.score(&target) < pure.score(&target));
    assert_eq!(Platform::parse("x86_64-linux-gnu").score(&target), None);
    assert_eq!(native.score(&target), Some(0));
}

#[test]
fn serde_round_trips_and_rejects_invalid_versions_and_requirements() {
    let version = version("1.2-rc1");
    let encoded = serde_json::to_string(&version).unwrap();
    assert_eq!(encoded, "\"1.2.pre.rc1\"");
    assert_eq!(serde_json::from_str::<Version>(&encoded).unwrap(), version);
    let requirement: Requirement = "~> 1.2, != 1.3".parse().unwrap();
    assert_eq!(
        serde_json::from_str::<Requirement>(&serde_json::to_string(&requirement).unwrap()).unwrap(),
        requirement
    );
    assert!(serde_json::from_str::<Version>("\"garbage\"").is_err());
    assert!(serde_json::from_str::<Requirement>("\"^1\"").is_err());
}

#[test]
fn package_identity_names_platform_archives_and_round_trips() {
    let pure = PackageId {
        name: "rack".into(),
        version: version("3.1.0"),
        platform: Platform::parse("ruby"),
    };
    let native = PackageId {
        name: "nokogiri".into(),
        version: version("1.19.4"),
        platform: Platform::parse("x86_64-linux-gnu"),
    };
    assert_eq!(pure.archive_version(), "3.1.0");
    assert_eq!(pure.full_name(), "rack-3.1.0");
    assert_eq!(native.to_string(), "nokogiri-1.19.4-x86_64-linux-gnu");
    assert_eq!(
        serde_json::from_str::<PackageId>(&serde_json::to_string(&native).unwrap()).unwrap(),
        native
    );
    let musl = PackageId {
        platform: Platform::parse("x86_64-linux-musl"),
        ..native.clone()
    };
    assert_eq!(HashSet::from([native, musl]).len(), 2);
}

#[test]
fn typed_constraints_are_available_to_resolver_adapters() {
    let requirement: Requirement = "~> 1.2, != 1.3".parse().unwrap();
    assert_eq!(
        requirement
            .constraints()
            .map(|(operator, version)| (operator, version.to_string()))
            .collect::<Vec<_>>(),
        [
            (Operator::Pessimistic, "1.2".into()),
            (Operator::NotEqual, "1.3".into())
        ]
    );
    assert_eq!(Operator::GreaterEqual.to_string(), ">=");
}

#[test]
fn platform_display_round_trips_canonical_components() {
    for value in [
        "ruby",
        "jruby",
        "java1.8",
        "mswin32",
        "i686-linux",
        "x86_64-linux-musl",
        "arm64-darwin-23.4",
    ] {
        let platform = Platform::parse(value);
        assert_eq!(Platform::parse(&platform.to_string()), platform);
    }
    assert_eq!(Platform::parse("ruby---").os, "unknown");
    assert_eq!(Platform::parse("x86_64-ruby").os, "unknown");
    assert_eq!(Platform::parse("x86_64-haiku").os, "unknown");
}
