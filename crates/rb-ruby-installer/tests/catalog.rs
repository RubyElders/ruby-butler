use rb_ruby_installer::{Platform, Release, Request};
use semver::Version;

fn release() -> Release {
    serde_json::from_str(include_str!("fixtures/catalog.json")).unwrap()
}

#[test]
fn selects_stable_versions_and_correct_platform() {
    for (request, expected) in [
        ("latest", "4.0.7"),
        ("3.4", "3.4.10"),
        ("3", "3.4.10"),
        ("3.4.9", "3.4.9"),
        (">= 3.4, < 4", "3.4.10"),
        ("~> 3.4.0", "3.4.10"),
    ] {
        assert_eq!(
            release()
                .select(&request.parse().unwrap(), Platform::LinuxX86_64)
                .unwrap()
                .version
                .to_string(),
            expected
        );
    }
    assert_eq!(release().distributions(Platform::LinuxX86_64).len(), 3);
    assert!(
        release()
            .select(&"3.3.0".parse().unwrap(), Platform::LinuxX86_64)
            .is_err()
    );
    assert!(release().distributions(Platform::MacosX86_64).is_empty());
    assert_eq!(release().distributions(Platform::MacosAarch64).len(), 1);
    assert_eq!(release().distributions(Platform::LinuxMuslAarch64).len(), 1);
}

#[test]
fn matches_installed_versions_with_the_same_rules() {
    let request: Request = "3.4".parse().unwrap();
    assert!(request.matches(&Version::new(3, 4, 10)));
    assert!(!request.matches(&Version::new(3, 3, 10)));
    assert!(!request.matches(&"3.4.11-preview1".parse().unwrap()));
    assert_eq!(
        "3.4.10".parse::<Request>().unwrap().exact_version(),
        Some(Version::new(3, 4, 10))
    );
    assert!(request.exact_version().is_none());
    for invalid in ["", " ", "no-such-ruby", "jruby-10", "../../ruby"] {
        assert!(invalid.parse::<Request>().is_err(), "{invalid}");
    }
}

#[test]
fn accepts_explicit_cruby_version_requests() {
    let request: Request = " ruby-4.0.5\n".parse().unwrap();
    assert_eq!(request.exact_version(), Some(Version::new(4, 0, 5)));
    assert!(request.matches(&Version::new(4, 0, 5)));
    assert!(!request.matches(&Version::new(4, 0, 6)));
    assert_eq!(request.to_string(), "ruby-4.0.5");
    let minor: Request = "ruby-3.4".parse().unwrap();
    assert!(minor.matches(&Version::new(3, 4, 10)));
    assert!(!minor.matches(&Version::new(4, 0, 5)));
    assert!(
        release()
            .select(&"ruby-3.4.10".parse().unwrap(), Platform::LinuxX86_64)
            .is_ok()
    );
    for unsupported in ["ruby-", "jruby-4.0.5", "truffleruby-4.0.5"] {
        assert!(unsupported.parse::<Request>().is_err(), "{unsupported}");
    }
}

#[test]
fn supported_hosts_are_explicit() {
    for (os, arch, musl, expected) in [
        ("linux", "x86_64", false, Platform::LinuxX86_64),
        ("linux", "aarch64", false, Platform::LinuxAarch64),
        ("linux", "x86_64", true, Platform::LinuxMuslX86_64),
        ("linux", "aarch64", true, Platform::LinuxMuslAarch64),
        ("macos", "x86_64", false, Platform::MacosX86_64),
        ("macos", "aarch64", false, Platform::MacosAarch64),
    ] {
        assert_eq!(Platform::for_host(os, arch, musl).unwrap(), expected);
    }
    for (os, arch) in [
        ("windows", "x86_64"),
        ("linux", "riscv64"),
        ("freebsd", "x86_64"),
    ] {
        assert!(
            Platform::for_host(os, arch, false)
                .unwrap_err()
                .to_string()
                .contains("not supported")
        );
    }
}

#[test]
fn requirements_exclude_incompatible_versions_and_release_candidates() {
    for (requirement, expected) in [
        (">= 3.4, < 4, != 3.4.10", "3.4.9"),
        ("~> 3.4.9", "3.4.10"),
        ("= 4.0.7", "4.0.7"),
    ] {
        let selected = release()
            .select(&requirement.parse().unwrap(), Platform::LinuxX86_64)
            .unwrap();
        assert_eq!(selected.version.to_string(), expected);
    }
    for requirement in ["> 4.0.7", "< 3.4.9", "4.1.0-preview1"] {
        assert!(
            release()
                .select(&requirement.parse().unwrap(), Platform::LinuxX86_64)
                .is_err()
        );
    }
}
