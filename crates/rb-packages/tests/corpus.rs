mod common;
use rb_packages::{MemoryIndex, PackageRecord, PlanOptions, plan};
use std::{fs, path::Path};

#[test]
fn frozen_gemfile_projections_resolve_reference_lock_closures() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../rb-resolver/tests/fixtures/corpus");
    let cases: Vec<serde_json::Value> =
        serde_json::from_str(&fs::read_to_string(root.join("cases.json")).unwrap()).unwrap();
    let mut index = MemoryIndex::default();
    for file in fs::read_dir(root.join("index")).unwrap() {
        let path = file.unwrap().path();
        let name = path.file_stem().unwrap().to_str().unwrap();
        for info in rb_compact_index::parse(name, &fs::read_to_string(&path).unwrap()).unwrap() {
            common::add(
                &mut index,
                PackageRecord::from_info(name, "https://rubygems.org/", info).unwrap(),
            );
        }
    }
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let expected: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(root.join(name).join("expected.json")).unwrap(),
        )
        .unwrap();
        let mut manifest: rb_gemfile::Manifest =
            toml::from_str(&fs::read_to_string(root.join(name).join("rbproject.toml")).unwrap())
                .unwrap();
        manifest.bundle.ruby.clear();
        manifest.dependencies.retain(|_, d| {
            d.enabled && d.platforms.is_empty() && d.groups.iter().any(|g| g == "default")
        });
        manifest.dependencies.insert(
            "bundler".into(),
            rb_gemfile::Dependency {
                version: vec![format!("= {}", expected["bundler"].as_str().unwrap())],
                ..Default::default()
            },
        );
        let mut runtime = common::runtime();
        runtime.version = expected["ruby"].as_str().unwrap().parse().unwrap();
        runtime.platform = expected["target"].as_str().unwrap().into();
        runtime.bundler = Some(expected["bundler"].as_str().unwrap().parse().unwrap());
        runtime.rubygems = expected["rubygems"].as_str().unwrap().parse().unwrap();
        let mut lock = rb_lockfile::Lockfile::parse(
            &fs::read_to_string(root.join(name).join("Gemfile.lock")).unwrap(),
        )
        .unwrap();
        lock.platforms = vec![runtime.platform.clone()];
        let result = plan(
            &manifest,
            &runtime,
            Some(&lock),
            &index,
            &PlanOptions::default(),
        )
        .unwrap_or_else(|e| panic!("{name}: {e:#}"));
        let actual: Vec<_> = result.packages.iter().map(|p| serde_json::json!({"name":p.record.package.id.name,"version":p.record.package.id.version.to_string(),"platform":p.record.package.id.platform.to_string()})).collect();
        assert_eq!(serde_json::json!(actual), expected["packages"], "{name}");
    }
}
