use rb_gemfile::{Evaluator, Manifest, Require, SnapshotCache};
use std::path::Path;

fn evaluate(project: &str, count: usize) -> Manifest {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/real-world")
        .join(project);
    let ruby = std::env::var_os("RB_GEMFILE_RUBY").unwrap_or_else(|| "ruby".into());
    let result = Evaluator::new(ruby)
        .evaluate(root.join("Gemfile"))
        .unwrap_or_else(|error| panic!("{project}: {error}"));
    assert_eq!(result.manifest.dependencies.len(), count, "{project}");
    assert_eq!(
        result.manifest.bundle.source.as_deref(),
        Some("https://rubygems.org"),
        "{project}"
    );
    assert!(
        result.manifest.dependencies.contains_key("rails"),
        "{project}"
    );
    let expected: serde_json::Value = serde_json::from_slice(
        &std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/corpus")
                .join(format!("{project}.json")),
        )
        .unwrap(),
    )
    .unwrap();
    let actual = serde_json::to_value(&result.manifest).unwrap();
    for (name, fields) in expected["dependencies"].as_object().unwrap() {
        for (field, value) in fields.as_object().unwrap() {
            assert_eq!(
                &actual["dependencies"][name][field], value,
                "{project}: {name}.{field}"
            );
        }
    }
    for name in expected["absent"].as_array().unwrap() {
        assert!(
            !result
                .manifest
                .dependencies
                .contains_key(name.as_str().unwrap()),
            "{project}: {name}"
        );
    }
    if let Some(groups) = expected.get("optional_groups") {
        assert_eq!(&actual["bundle"]["optional_groups"], groups, "{project}");
    }
    let toml = result.manifest.to_toml().unwrap();
    let kdl = result.manifest.to_kdl().unwrap();
    let parsed_kdl =
        rb_gemfile::Manifest::from_kdl(&kdl).unwrap_or_else(|error| panic!("{project}: {error}"));
    let parsed_toml: rb_gemfile::Manifest = toml::from_str(&toml).unwrap();
    assert_eq!(parsed_kdl, parsed_toml, "{project}");

    assert_eq!(
        parsed_kdl.dependencies.len(),
        count,
        "{project}: KDL dependency count"
    );
    let exported = toml::from_str::<Manifest>(&toml).unwrap();
    assert!(
        exported
            .dependencies
            .values()
            .all(|dependency| dependency.enabled
                && dependency.variants.iter().all(|variant| variant.enabled)),
        "{project}"
    );
    assert_eq!(exported.to_toml().unwrap(), toml, "{project}");
    assert_eq!(
        exported.dependencies.len(),
        count,
        "{project}: exported count"
    );
    for name in expected["absent"].as_array().unwrap() {
        assert!(
            !exported.dependencies.contains_key(name.as_str().unwrap()),
            "{project}: exported {name}"
        );
    }
    for (name, dependency) in &result.manifest.dependencies {
        assert!(dependency.enabled, "{project}: {name}");
        assert!(
            dependency.variants.iter().all(|variant| variant.enabled),
            "{project}: {name}"
        );
        assert_eq!(
            &exported.dependencies[name], dependency,
            "{project}: round-trip {name}"
        );
    }
    let merged: toml::Value = toml::from_str(
        &result
            .manifest
            .merge_project("[project]\nname = 'corpus'\n")
            .unwrap(),
    )
    .unwrap();
    let plain: toml::Value = toml::from_str(&toml).unwrap();
    assert_eq!(merged.get("dependencies"), plain.get("dependencies"));
    assert_eq!(merged.get("sources"), plain.get("sources"));
    assert_eq!(merged["project"]["name"].as_str(), Some("corpus"));
    let directory = tempfile::tempdir().unwrap();
    let cache = SnapshotCache::new(directory.path().join("snapshot.json"));
    let evaluator =
        Evaluator::new(std::env::var_os("RB_GEMFILE_RUBY").unwrap_or_else(|| "ruby".into()));
    let initial = cache.evaluate(&evaluator, root.join("Gemfile")).unwrap();
    assert!(!initial.reused);
    assert_eq!(initial.evaluation.manifest, result.manifest);
    let reused = cache.evaluate(&evaluator, root.join("Gemfile")).unwrap();
    assert!(reused.reused);
    assert_eq!(reused.evaluation.manifest, result.manifest);
    result.manifest
}

#[test]
fn chatwoot() {
    let manifest = evaluate("chatwoot", 152);
    assert_eq!(manifest.dependencies["rack-cors"].version, ["2.0.0"]);
    assert_eq!(
        manifest.dependencies["rack-cors"].require,
        Some(Require::Name("rack/cors".into()))
    );
}

#[test]
fn diaspora() {
    let manifest = evaluate("diaspora", 113);
    assert_eq!(manifest.dependencies["puma"].version, ["7.2.0"]);
    assert_eq!(
        manifest.dependencies["puma"].require,
        Some(Require::Enabled(false))
    );
}

#[test]
fn errbit() {
    let manifest = evaluate("errbit", 83);
    assert_eq!(manifest.dependencies["rails"].version, ["8.1.3.1"]);
    assert_eq!(
        manifest.dependencies["simplecov"].require,
        Some(Require::Enabled(false))
    );
}

#[test]
fn forem() {
    let manifest = evaluate("forem", 167);
    assert_eq!(manifest.dependencies["stackprof"].platforms, ["ruby"]);
    assert_eq!(
        manifest.dependencies["stripe-ruby-mock"].require,
        Some(Require::Name("stripe_mock".into()))
    );
}

#[test]
fn foreman() {
    let manifest = evaluate("foreman", 105);
    assert_eq!(manifest.dependencies["rdoc"].version, [">= 0"]);
    assert_eq!(manifest.dependencies["puma"].groups, ["test", "service"]);
    assert_eq!(
        manifest.dependencies["fog-vsphere"].version,
        [">= 3.7.1", "< 4.0"]
    );
    assert!(manifest.dependencies.contains_key("theforeman-rubocop"));
}

#[test]
fn lobsters() {
    let manifest = evaluate("lobsters", 58);
    assert_eq!(
        manifest.dependencies["sqlite3"].force_ruby_platform,
        Some(true)
    );
    assert_eq!(
        manifest.dependencies["svg-graph"].require,
        Some(Require::Name("SVG/Graph/TimeSeries".into()))
    );
}

#[test]
fn manageiq() {
    let manifest = evaluate("manageiq", 145);
    assert_eq!(manifest.bundle.plugins[0].name, "bundler-inject");
    assert_eq!(manifest.bundle.plugins[0].version, ["~> 2.0"]);
    assert!(
        manifest
            .bundle
            .optional_groups
            .contains(&"appliance".into())
    );
    assert_eq!(
        manifest.dependencies["pg"].source.as_deref(),
        Some("https://rubygems.manageiq.org")
    );
    assert_eq!(
        manifest.dependencies["manageiq-schema"].git.as_deref(),
        Some("https://github.com/ManageIQ/manageiq-schema")
    );
}

#[test]
fn mastodon() {
    let manifest = evaluate("mastodon", 154);
    assert_eq!(manifest.bundle.ruby, [">= 3.3.0", "< 4.1.0"]);
    assert_eq!(
        manifest.dependencies["idn-ruby"].require,
        Some(Require::Name("idn".into()))
    );
}

#[test]
fn postal() {
    let manifest = evaluate("postal", 50);
    assert_eq!(
        manifest.dependencies["highline"].require,
        Some(Require::Enabled(false))
    );
}

#[test]
fn zammad() {
    let manifest = evaluate("zammad", 131);
    assert_eq!(manifest.dependencies["debug"].platforms, ["mri"]);
    assert_eq!(
        manifest.dependencies["autodiscover"].git.as_deref(),
        Some("https://github.com/zammad-deps/autodiscover")
    );
}

#[test]
fn corpus_gemfiles_match_their_recorded_upstream_hashes() {
    use sha2::{Digest, Sha256};
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/real-world");
    let mut projects = 0;
    for entry in std::fs::read_dir(root).unwrap() {
        let directory = entry.unwrap().path();
        if !directory.is_dir() {
            continue;
        }
        projects += 1;
        let provenance: serde_json::Value =
            serde_json::from_slice(&std::fs::read(directory.join("source.json")).unwrap()).unwrap();
        assert_eq!(provenance["sha"].as_str().unwrap().len(), 40);
        assert!(provenance["files_sha256"].get("Gemfile").is_some());
        for (name, expected) in provenance["files_sha256"].as_object().unwrap() {
            let path = directory.join(name);
            if path.is_file() {
                let actual = format!("{:x}", Sha256::digest(std::fs::read(&path).unwrap()));
                assert_eq!(actual, expected.as_str().unwrap(), "{}", path.display());
            }
        }
    }
    assert_eq!(projects, 10);
}
