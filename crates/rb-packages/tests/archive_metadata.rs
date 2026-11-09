mod common;

use common::server::Server;
use rb_packages::{
    InstallOptions, LockMode, MemoryIndex, PlanOptions, PrepareOptions, install, plan, prepare,
};
use rb_task::TaskEvents;
use std::{collections::BTreeMap, fs, sync::Arc};

fn archive(root: &std::path::Path, ruby: &str) -> (MemoryIndex, Vec<u8>) {
    let mut index = MemoryIndex::default();
    common::add_archive_with_ruby(
        &mut index,
        root,
        "teacup",
        &[],
        &[],
        &[("lib/teacup.rb", b"TEACUP = 42\n")],
        ruby,
    );
    let bytes = fs::read(index.archives.values().next().unwrap()).unwrap();
    (index, bytes)
}

fn registry(index: &MemoryIndex, bytes: Vec<u8>, ruby_metadata: &str) -> Server {
    Server::new(BTreeMap::from([
        (
            "/info/teacup".into(),
            format!(
                "---\n1.0.0 |checksum:{}{ruby_metadata}\n",
                index.packages["teacup"][0].checksum
            )
            .into_bytes(),
        ),
        ("/gems/teacup-1.0.0.gem".into(), bytes),
    ]))
}

fn options(root: &std::path::Path, server: &Server) -> PrepareOptions {
    fs::write(
        root.join("Gemfile"),
        format!("source '{}'\ngem 'teacup'\n", server.url),
    )
    .unwrap();
    PrepareOptions::new(root, common::ruby(), root.join(".rb/cache"))
}

#[test]
fn missing_registry_ruby_metadata_uses_archive_requirement_and_reuses_locked_offline() {
    let root = tempfile::tempdir().unwrap();
    let archives = tempfile::tempdir().unwrap();
    let (index, bytes) = archive(archives.path(), ">= 3.2.2");
    let server = registry(&index, bytes, "");
    let mut options = options(root.path(), &server);
    let installed = prepare(options.clone(), TaskEvents::default()).unwrap();
    assert_eq!(installed.installed.installed, 1);
    let output = common::run(
        &common::runtime(),
        &installed.installed,
        root.path(),
        "require 'teacup'; puts TEACUP",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(common::stdout(&output), "42\n");
    let count = server.requests.lock().unwrap().len();
    options.offline = true;
    options.plan.mode = LockMode::Locked;
    let reused = prepare(options, TaskEvents::default()).unwrap();
    assert_eq!(reused.installed.reused, 1);
    assert_eq!(server.requests.lock().unwrap().len(), count);
}

#[test]
fn missing_registry_ruby_metadata_still_rejects_an_incompatible_archive() {
    let root = tempfile::tempdir().unwrap();
    let archives = tempfile::tempdir().unwrap();
    let (index, bytes) = archive(archives.path(), ">= 99");
    let server = registry(&index, bytes, "");
    let error = prepare(options(root.path(), &server), TaskEvents::default())
        .err()
        .unwrap();
    let text = format!("{error:#}");
    assert!(text.contains("teacup-1.0.0 requires Ruby >= 99"), "{text}");
    assert!(!text.contains("Constraint {"), "{text}");
    assert!(
        !root
            .path()
            .join(".rb/bundle/ruby")
            .join(common::runtime().abi)
            .exists()
    );
}

#[test]
fn explicit_registry_ruby_metadata_must_match_archive() {
    let root = tempfile::tempdir().unwrap();
    let archives = tempfile::tempdir().unwrap();
    let (index, bytes) = archive(archives.path(), ">= 3.2.2");
    let server = registry(&index, bytes, ",ruby:>= 0");
    let error = prepare(options(root.path(), &server), TaskEvents::default())
        .err()
        .unwrap();
    let text = format!("{error:#}");
    assert!(
        text.contains("archive requires >= 3.2.2, resolution declares >= 0"),
        "{text}"
    );
    assert!(!text.contains("dependencies"), "{text}");
}

#[test]
fn dependency_mismatches_stay_strict_and_report_only_the_difference() {
    for (kind, expected) in [
        ("missing", "missing spoon (resolution declares >= 0)"),
        ("unexpected", "unexpected spoon >= 1"),
        (
            "requirement",
            "spoon requires >= 1, resolution declares >= 0",
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        let mut index = MemoryIndex::default();
        let dependencies = if kind == "missing" {
            vec![]
        } else {
            vec![("spoon", ">= 1")]
        };
        common::add_archive(
            &mut index,
            root.path(),
            "teacup",
            &dependencies,
            &[],
            &[("lib/teacup.rb", b"TEACUP = 42\n")],
        );
        common::add_archive(
            &mut index,
            root.path(),
            "spoon",
            &[],
            &[],
            &[("lib/spoon.rb", b"SPOON = 1\n")],
        );
        let record = &mut index.packages.get_mut("teacup").unwrap()[0];
        record.declared_ruby = None;
        record.package.dependencies = if kind == "unexpected" {
            BTreeMap::new()
        } else {
            BTreeMap::from([("spoon".into(), ">= 0".parse().unwrap())])
        };
        let runtime = common::runtime();
        let plan = plan(
            &common::manifest("https://rubygems.org", "teacup = '>= 0'"),
            &runtime,
            None,
            &index,
            &PlanOptions::default(),
        )
        .unwrap();
        let error = install(
            root.path(),
            &plan,
            &runtime,
            Arc::new(index),
            InstallOptions::new(&runtime.executable),
            TaskEvents::default(),
        )
        .err()
        .unwrap();
        let text = format!("{error:#}");
        assert!(text.contains(expected), "{kind}: {text}");
        assert!(!text.contains("Requirement {"), "{text}");
        assert!(!text.contains("Ruby"), "{text}");
    }
}
