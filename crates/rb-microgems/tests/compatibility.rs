use rb_microgems::{Environment, InstalledGem};
use std::{collections::BTreeMap, ffi::OsString, process::Command};

fn ruby() -> OsString {
    std::env::var_os("RB_GEM_BUILD_RUBY").unwrap_or_else(|| "ruby".into())
}

#[test]
fn build_api_contracts_without_rubygems() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("lib")).unwrap();
    std::fs::write(root.path().join("lib/fixture_api.rb"), "FIXTURE_API = 42\n").unwrap();
    let gem = InstalledGem {
        name: "fixture".into(),
        version: "2.1.3".into(),
        platform: "ruby".into(),
        root: root.path().into(),
        require_paths: vec!["lib".into()],
        metadata: BTreeMap::from([("purpose".into(), "test".into())]),
        executables: vec![],
        bindir: "bin".into(),
    };
    let environment = Environment::new(&[gem], &["--with-extra=a b".into()]).unwrap();
    let output = Command::new(ruby())
        .args(["--disable=gems", "-I"])
        .arg(environment.load_path())
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../gems/rb-microgems/test/fixtures/contracts.rb"
        ))
        .env("RB_MICROGEMS_MANIFEST", environment.manifest_path())
        .env("FIXTURE_ROOT", root.path().join("."))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("contracts passed"));
}

#[test]
fn environment_is_removed_on_drop() {
    let environment = Environment::new(&[], &[]).unwrap();
    let path = environment.load_path().to_owned();
    assert!(environment.manifest_path().is_file());
    drop(environment);
    assert!(!path.exists());
}

#[test]
fn version_requirement_and_platform_match_rubygems() {
    let fixture = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../gems/rb-microgems/test/fixtures/version_oracle.rb"
    );
    let canonical = Command::new(ruby())
        .arg("-rrubygems")
        .arg(fixture)
        .output()
        .unwrap();
    assert!(
        canonical.status.success(),
        "{}",
        String::from_utf8_lossy(&canonical.stderr)
    );
    let environment = Environment::new(&[], &[]).unwrap();
    let compatible = Command::new(ruby())
        .args(["--disable=gems", "-I"])
        .arg(environment.load_path())
        .arg("-rmicrogems/compatibility")
        .arg(fixture)
        .output()
        .unwrap();
    assert!(
        compatible.status.success(),
        "{}",
        String::from_utf8_lossy(&compatible.stderr)
    );
    let expected: serde_json::Value = serde_json::from_slice(&canonical.stdout).unwrap();
    let actual: serde_json::Value = serde_json::from_slice(&compatible.stdout).unwrap();
    for key in [
        "comparisons",
        "requirements",
        "versions",
        "platforms",
        "generic_platforms",
    ] {
        assert_eq!(actual[key], expected[key], "RubyGems mismatch for {key}");
    }
}

#[cfg(unix)]
#[test]
fn rejects_non_utf8_build_arguments() {
    use std::os::unix::ffi::OsStringExt;
    let error = Environment::new(&[], &[OsString::from_vec(vec![0xff])])
        .err()
        .unwrap();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn core_namespace_works_without_the_gem_alias() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("lib")).unwrap();
    std::fs::write(root.path().join("lib/fixture_api.rb"), "FIXTURE_API = 42\n").unwrap();
    let gem = InstalledGem {
        name: "fixture".into(),
        version: "2.1.3".into(),
        platform: "ruby".into(),
        root: root.path().into(),
        require_paths: vec!["lib".into()],
        metadata: BTreeMap::new(),
        executables: vec![],
        bindir: "bin".into(),
    };
    let environment = Environment::new(&[gem], &[]).unwrap();
    let output = Command::new(ruby())
        .args(["--disable=gems", "-I"])
        .arg(environment.load_path())
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../gems/rb-microgems/test/fixtures/namespaces.rb"
        ))
        .env("RB_MICROGEMS_MANIFEST", environment.manifest_path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "namespaces passed"
    );
}

#[test]
fn compatibility_imports_work_as_the_first_require() {
    let environment = Environment::new(&[], &[]).unwrap();
    for feature in [
        "rubygems",
        "rubygems/version",
        "rubygems/requirement",
        "rubygems/platform",
        "rubygems/specification",
        "rubygems/command",
        "rubygems/package",
        "rubygems/ext",
        "rubygems/ext/builder",
    ] {
        let output = Command::new(ruby())
            .args(["--disable=gems", "-I"])
            .arg(environment.load_path())
            .arg(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../gems/rb-microgems/test/fixtures/shims.rb"
            ))
            .arg(feature)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{feature}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            "shims passed"
        );
    }
}

#[test]
fn compatibility_alias_does_not_replace_loaded_rubygems() {
    let environment = Environment::new(&[], &[]).unwrap();
    let output = Command::new(ruby())
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../gems/rb-microgems/test/fixtures/existing_rubygems.rb"
        ))
        .env("MICROGEMS_PATH", environment.load_path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "existing RubyGems preserved"
    );
}

#[test]
fn materialized_library_matches_the_gem_sources() {
    fn files(
        root: &std::path::Path,
        directory: &std::path::Path,
        found: &mut BTreeMap<std::path::PathBuf, Vec<u8>>,
    ) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                files(root, &path, found);
            } else {
                found.insert(
                    path.strip_prefix(root).unwrap().to_owned(),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }
    let source =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../gems/rb-microgems/lib");
    let mut expected = BTreeMap::new();
    files(&source, &source, &mut expected);
    let environment = Environment::new(&[], &[]).unwrap();
    let mut actual = BTreeMap::new();
    files(
        environment.load_path(),
        environment.load_path(),
        &mut actual,
    );
    actual.remove(std::path::Path::new("manifest.json"));
    assert_eq!(actual, expected);
}
