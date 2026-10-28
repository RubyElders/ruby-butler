mod common;
use common::server::Server;
use rb_packages::{PrepareOptions, prepare, prepared};
use rb_task::TaskEvents;
use std::{collections::BTreeMap, fs};

#[test]
fn ready_state_checks_dependency_inputs_and_installation_state_without_registry_requests() {
    let root = tempfile::tempdir().unwrap();
    let archives = tempfile::tempdir().unwrap();
    let mut index = rb_packages::MemoryIndex::default();
    common::add_archive(
        &mut index,
        archives.path(),
        "teacup",
        &[],
        &[],
        &[("lib/teacup.rb", b"TEACUP=42\n")],
    );
    let server = Server::new(BTreeMap::from([
        (
            "/info/teacup".into(),
            format!(
                "---\n1.0.0 |checksum:{}\n",
                index.packages["teacup"][0].checksum
            )
            .into_bytes(),
        ),
        (
            "/gems/teacup-1.0.0.gem".into(),
            fs::read(index.archives.values().next().unwrap()).unwrap(),
        ),
    ]));
    let gemfile = root.path().join("Gemfile");
    fs::write(&gemfile, format!("source '{}'\ngem 'teacup'\n", server.url)).unwrap();
    let mut options =
        PrepareOptions::new(root.path(), common::ruby(), root.path().join(".rb/cache"));
    options.record_state = true;
    let bundle = prepare(options.clone(), TaskEvents::default()).unwrap();
    assert!(root.path().join(".rb/prepared.json").is_file());
    let state: serde_json::Value =
        serde_json::from_slice(&fs::read(root.path().join(".rb/prepared.json")).unwrap()).unwrap();
    assert_eq!(
        state["state"]["inputs"],
        serde_json::json!([gemfile.canonicalize().unwrap()])
    );
    let ready = prepared(&options).unwrap().unwrap();
    assert_eq!(ready.installed, 0);
    assert_eq!(ready.reused, 1);
    assert_eq!(ready.environment(), bundle.installed.environment());
    let payload = bundle.installed.gem_home.join("untracked-payload");
    fs::write(&payload, "not a dependency input").unwrap();
    assert!(
        prepared(&options).unwrap().is_some(),
        "Startup must not scan installed payloads"
    );
    fs::remove_file(&payload).unwrap();
    let requests = server.requests.lock().unwrap().len();
    for path in [
        gemfile.clone(),
        root.path().join("Gemfile.lock"),
        root.path().join(".rb/rbproject.toml"),
        root.path().join(".rb/prepared.json"),
        bundle.installed.bundle_path.join("fingerprint"),
    ] {
        let bytes = fs::read(&path).unwrap();
        let permissions = fs::metadata(&path).unwrap().permissions();
        fs::remove_file(&path).unwrap();
        assert!(
            prepared(&options).unwrap().is_none(),
            "Missing {}",
            path.display()
        );
        fs::write(&path, b"corrupt").unwrap();
        assert!(
            prepared(&options).unwrap().is_none(),
            "Changed {}",
            path.display()
        );
        fs::write(&path, bytes).unwrap();
        fs::set_permissions(&path, permissions).unwrap();
        assert!(
            prepared(&options).unwrap().is_some(),
            "Restored {}",
            path.display()
        );
    }
    let home = &bundle.installed.gem_home;
    let removed = home.with_extension("removed");
    fs::rename(home, &removed).unwrap();
    assert!(prepared(&options).unwrap().is_none());
    fs::rename(&removed, home).unwrap();
    assert!(prepared(&options).unwrap().is_some());
    assert!(!root.path().join(".rb/installed-files.json").exists());
    let mut changed = options.clone();
    changed.plan.without.insert("test".into());
    assert!(prepared(&changed).unwrap().is_none());
    changed = options.clone();
    changed
        .install
        .build
        .arguments
        .push("--changed-build".into());
    assert!(prepared(&changed).unwrap().is_none());
    changed = options.clone();
    changed.ruby = root.path().join("another-ruby");
    assert!(prepared(&changed).unwrap().is_none());
    let new_input = root.path().join("Gemfile.extra");
    fs::write(&new_input, "gem 'saucer'\n").unwrap();
    assert!(prepared(&options).unwrap().is_some());
    fs::remove_file(new_input).unwrap();
    assert!(prepared(&options).unwrap().is_some());
    assert_eq!(server.requests.lock().unwrap().len(), requests);
    fs::write(&gemfile, "raise 'preparation failed'\n").unwrap();
    assert!(prepare(options.clone(), TaskEvents::default()).is_err());
    assert!(!root.path().join(".rb/prepared.json").exists());
}
