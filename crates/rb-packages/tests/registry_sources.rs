mod common;
use common::server::Server;
use rb_packages::{LockMode, PackageIndex, PlanOptions, RegistryIndex, plan};
use std::collections::BTreeMap;

fn info(name: &str) -> Vec<u8> {
    std::fs::read(format!(
        "{}/tests/fixtures/source-priority/{name}.info",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap()
}

fn versions(names: &[&str]) -> Vec<u8> {
    format!(
        "---\n{}",
        names
            .iter()
            .map(|name| format!("{name} 1.0 {}\n", "a".repeat(32)))
            .collect::<String>()
    )
    .into_bytes()
}

fn scoped() -> Server {
    Server::new(BTreeMap::from([
        ("/versions".into(), versions(&["parent", "child", "leaf"])),
        ("/info/parent".into(), info("parent")),
        ("/info/child".into(), info("child")),
        ("/info/leaf".into(), info("leaf")),
    ]))
}

#[test]
fn optional_scoped_children_take_priority_before_global_candidates_and_reuse_offline() {
    let global = Server::new(BTreeMap::from([
        ("/info/application".into(), info("application")),
        ("/info/public".into(), info("public")),
        ("/info/child".into(), info("newer-child")),
    ]));
    let scoped = scoped();
    let mut manifest = common::manifest(
        &global.url,
        "application='>= 0'\nparent={groups=['optional']}",
    );
    manifest.bundle.optional_groups.push("optional".into());
    manifest.dependencies.get_mut("parent").unwrap().source = Some(scoped.url.clone());
    let cache = tempfile::tempdir().unwrap();
    let index = RegistryIndex::new(&manifest, cache.path(), false).unwrap();
    let child = index.candidates("child").unwrap();
    assert_eq!(child.len(), 1);
    assert_eq!(child[0].archive_version, "1.0");
    assert_eq!(child[0].source, scoped.url.clone().into());
    assert_eq!(
        index.candidates("leaf").unwrap()[0].source,
        scoped.url.clone().into()
    );
    let runtime = common::runtime();
    let result = plan(&manifest, &runtime, None, &index, &PlanOptions::default()).unwrap();
    assert_eq!(result.packages.len(), 5);
    let installed: Vec<_> = result
        .packages
        .iter()
        .filter(|p| p.install)
        .map(|p| p.record.package.id.name.as_str())
        .collect();
    assert_eq!(installed, ["application", "public"]);
    assert_eq!(
        result
            .packages
            .iter()
            .find(|p| p.record.package.id.name == "public")
            .unwrap()
            .record
            .source,
        global.url.clone().into()
    );
    assert!(
        !global
            .requests
            .lock()
            .unwrap()
            .contains(&"/info/child".into())
    );
    assert!(
        !scoped
            .requests
            .lock()
            .unwrap()
            .contains(&"/info/public".into())
    );
    for path in ["/info/parent", "/info/child", "/info/leaf"] {
        assert_eq!(
            scoped
                .requests
                .lock()
                .unwrap()
                .iter()
                .filter(|p| p.as_str() == path)
                .count(),
            1
        );
    }
    drop(index);
    drop(global);
    drop(scoped);
    let offline = RegistryIndex::new(&manifest, cache.path(), true).unwrap();
    let frozen = plan(
        &manifest,
        &runtime,
        Some(&result.lockfile),
        &offline,
        &PlanOptions {
            mode: LockMode::Locked,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(frozen.lockfile, result.lockfile);
    assert!(!frozen.lock_changed);
    assert_eq!(frozen.packages.iter().filter(|p| p.install).count(), 2);
}

#[test]
fn competing_scoped_children_require_an_explicit_source() {
    let one = Server::new(BTreeMap::from([
        ("/versions".into(), versions(&["alpha", "child", "leaf"])),
        ("/info/alpha".into(), info("parent")),
        ("/info/child".into(), info("child")),
        ("/info/leaf".into(), info("leaf")),
    ]));
    let two = Server::new(BTreeMap::from([
        ("/versions".into(), versions(&["omega", "child"])),
        ("/info/omega".into(), info("parent")),
        ("/info/child".into(), info("newer-child")),
    ]));
    let global = Server::new(BTreeMap::from([("/info/public".into(), info("public"))]));
    let mut manifest = common::manifest(&global.url, "alpha='>= 0'\nomega='>= 0'");
    manifest.dependencies.get_mut("alpha").unwrap().source = Some(one.url.clone());
    manifest.dependencies.get_mut("omega").unwrap().source = Some(two.url.clone());
    let cache = tempfile::tempdir().unwrap();
    let index = RegistryIndex::new(&manifest, cache.path(), false).unwrap();
    let error = index.candidates("alpha").unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Ambiguous transitive source for child")
    );
    manifest.dependencies.insert(
        "child".into(),
        rb_gemfile::Dependency {
            source: Some(one.url.clone()),
            ..Default::default()
        },
    );
    let explicit = RegistryIndex::new(&manifest, cache.path(), false).unwrap();
    let result = plan(
        &manifest,
        &common::runtime(),
        None,
        &explicit,
        &PlanOptions::default(),
    )
    .unwrap();
    assert_eq!(
        result
            .packages
            .iter()
            .find(|p| p.record.package.id.name == "child")
            .unwrap()
            .record
            .source,
        one.url.clone().into()
    );
    assert_eq!(
        result
            .packages
            .iter()
            .find(|p| p.record.package.id.name == "child")
            .unwrap()
            .record
            .archive_version,
        "1.0"
    );
}

#[test]
fn explicit_missing_sources_do_not_fall_back_and_global_roots_override_inheritance() {
    let private = scoped();
    let global = Server::new(BTreeMap::from([
        ("/info/child".into(), info("newer-child")),
        ("/info/public".into(), info("public")),
        ("/info/absent".into(), info("leaf")),
    ]));
    let mut manifest = common::manifest(&global.url, "parent='>= 0'\nchild='>= 0'\nabsent='>= 0'");
    manifest.dependencies.get_mut("parent").unwrap().source = Some(private.url.clone());
    manifest.dependencies.get_mut("absent").unwrap().source = Some(private.url.clone());
    let cache = tempfile::tempdir().unwrap();
    let index = RegistryIndex::new(&manifest, cache.path(), false).unwrap();
    assert!(index.candidates("absent").unwrap().is_empty());
    assert!(
        !global
            .requests
            .lock()
            .unwrap()
            .contains(&"/info/absent".into())
    );
    let child = index.candidates("child").unwrap();
    assert_eq!(child[0].archive_version, "99.0");
    assert_eq!(child[0].source, global.url.clone().into());
    assert!(
        !private
            .requests
            .lock()
            .unwrap()
            .contains(&"/info/child".into())
    );
}

#[test]
fn optional_scoped_archives_are_downloaded_only_after_opt_in_and_run_with_frozen_bundler() {
    let project = tempfile::tempdir().unwrap();
    let archives = tempfile::tempdir().unwrap();
    let mut index = rb_packages::MemoryIndex::default();
    for (name, dependencies, file, content) in [
        (
            "application",
            vec![("helper", ">= 1.0.0")],
            "lib/application.rb",
            "APPLICATION = 1\n",
        ),
        ("helper", vec![], "lib/helper.rb", "HELPER = 2\n"),
        (
            "addon",
            vec![("license", ">= 1.0.0"), ("helper", ">= 1.0.0")],
            "lib/addon.rb",
            "ADDON = 3\n",
        ),
        ("license", vec![], "lib/license.rb", "LICENSE = 4\n"),
    ] {
        common::add_archive(
            &mut index,
            archives.path(),
            name,
            &dependencies,
            &[],
            &[(file, content.as_bytes())],
        );
    }
    let responses = |names: &[&str]| {
        let mut responses = BTreeMap::from([("/versions".into(), versions(names))]);
        for name in names {
            let record = &index.packages[*name][0];
            let dependencies = record
                .package
                .dependencies
                .iter()
                .map(|(n, r)| format!("{n}:{r}"))
                .collect::<Vec<_>>()
                .join(",");
            responses.insert(
                format!("/info/{name}"),
                format!("---\n1.0.0 {dependencies}|checksum:{}\n", record.checksum).into_bytes(),
            );
            responses.insert(
                format!("/gems/{name}-1.0.0.gem"),
                std::fs::read(&index.archives[&record.checksum]).unwrap(),
            );
        }
        responses
    };
    let global = Server::new(responses(&["application", "helper"]));
    let scoped = Server::new(responses(&["addon", "license"]));
    std::fs::write(project.path().join("Gemfile"), format!("source '{}'\ngem 'application'\ngroup :addon, optional: true do\n  source '{}' do\n    gem 'addon'\n  end\nend\n", global.url, scoped.url)).unwrap();
    let options = || {
        rb_packages::PrepareOptions::new(
            project.path(),
            common::ruby(),
            project.path().join(".rb/cache"),
        )
    };
    let first = rb_packages::prepare(options(), rb_task::TaskEvents::default()).unwrap();
    assert_eq!(first.installed.installed, 2);
    assert!(
        scoped
            .requests
            .lock()
            .unwrap()
            .iter()
            .all(|path| !path.starts_with("/gems/"))
    );
    assert!(!first.installed.gem_home.join("gems/addon-1.0.0").exists());
    let output = common::run(
        &common::runtime(),
        &first.installed,
        project.path(),
        "require 'bundler'; Bundler.settings.set_local(:frozen, true); require 'bundler/setup'; require 'application'; raise 'optional activated' if Gem.loaded_specs.key?('addon'); puts APPLICATION",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(common::stdout(&output), "1\n");
    let mut enabled = options();
    enabled.plan.with.insert("addon".into());
    let second = rb_packages::prepare(enabled, rb_task::TaskEvents::default()).unwrap();
    assert!(!second.plan.lock_changed);
    assert_eq!(second.installed.installed, 2);
    assert_eq!(second.installed.reused, 2);
    assert!(
        scoped
            .requests
            .lock()
            .unwrap()
            .contains(&"/gems/addon-1.0.0.gem".into())
    );
    assert!(
        scoped
            .requests
            .lock()
            .unwrap()
            .contains(&"/gems/license-1.0.0.gem".into())
    );
    let output = common::run(
        &common::runtime(),
        &second.installed,
        project.path(),
        "require 'bundler'; Bundler.settings.set_local(:frozen, true); require 'bundler/setup'; require 'application'; require 'addon'; require 'license'; puts APPLICATION + ADDON + LICENSE",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(common::stdout(&output), "8\n");
}
