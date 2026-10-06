use rb_gem_types::{Platform, Requirement, Version};
use serde::Deserialize;
use serde_json::{Value, json};
use std::process::Command;

#[derive(Deserialize)]
struct Cases {
    versions: Vec<String>,
    invalid_versions: Vec<String>,
    requirements: Vec<String>,
    invalid_requirements: Vec<String>,
    platforms: Vec<String>,
}

fn rust_results() -> Value {
    let cases: Cases = serde_json::from_str(include_str!("fixtures/cases.json")).unwrap();
    let versions: Vec<Version> = cases
        .versions
        .iter()
        .map(|value| value.parse().unwrap())
        .collect();
    let requirements: Vec<Requirement> = cases
        .requirements
        .iter()
        .map(|value| value.parse().unwrap())
        .collect();
    let platforms: Vec<Platform> = cases
        .platforms
        .iter()
        .map(|value| Platform::parse(value))
        .collect();
    json!({
        "versions": versions.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "comparison": versions.iter().flat_map(|a| versions.iter().map(move |b| a.cmp(b) as i8)).collect::<Vec<_>>(),
        "release": versions.iter().map(|version| version.release().to_string()).collect::<Vec<_>>(),
        "bump": versions.iter().map(|version| version.bump().to_string()).collect::<Vec<_>>(),
        "prerelease": versions.iter().map(Version::prerelease).collect::<Vec<_>>(),
        "invalid_versions": cases.invalid_versions.iter().map(|value| value.parse::<Version>().is_err()).collect::<Vec<_>>(),
        "requirements": requirements.iter().flat_map(|requirement| versions.iter().map(move |version| requirement.matches(version))).collect::<Vec<_>>(),
        "requirement_prerelease": requirements.iter().map(Requirement::prerelease).collect::<Vec<_>>(),
        "invalid_requirements": cases.invalid_requirements.iter().map(|value| value.parse::<Requirement>().is_err()).collect::<Vec<_>>(),
        "platforms": platforms.iter().map(|platform| json!([platform.cpu, platform.os, platform.version])).collect::<Vec<_>>(),
        "platform_matches": platforms.iter().flat_map(|gem| platforms.iter().map(move |target| gem.matches(target))).collect::<Vec<_>>()
    })
}

fn assert_contract(expected: Value, platforms: bool) {
    let actual = rust_results();
    for (field, value) in expected.as_object().unwrap() {
        if !platforms && matches!(field.as_str(), "platforms" | "platform_matches") {
            continue;
        }
        let actual = actual[field].as_array().unwrap();
        let expected = value.as_array().unwrap();
        assert_eq!(
            actual.len(),
            expected.len(),
            "RubyGems contract length: {field}"
        );
        for (index, (actual, expected)) in actual.iter().zip(expected).enumerate() {
            assert_eq!(actual, expected, "RubyGems contract: {field}[{index}]");
        }
    }
    assert_eq!(
        actual.as_object().unwrap().len(),
        expected.as_object().unwrap().len()
    );
}

#[test]
fn frozen_rubygems_contract() {
    assert_contract(
        serde_json::from_str(include_str!("fixtures/rubygems.json")).unwrap(),
        true,
    );
}

#[test]
fn installed_rubygems_version_and_requirement_contract() {
    let ruby = std::env::var_os("RB_TEST_RUBY").unwrap_or_else(|| "ruby".into());
    let output = Command::new(ruby)
        .env_remove("RUBYOPT")
        .env_remove("RUBYLIB")
        .env_remove("RUBYGEMS_GEMDEPS")
        .args([
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/oracle.rb"),
            concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/cases.json"),
        ])
        .output()
        .expect("Ruby with RubyGems is required for the live oracle; set RB_TEST_RUBY");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_contract(serde_json::from_slice(&output.stdout).unwrap(), false);
}
