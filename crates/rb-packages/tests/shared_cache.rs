mod common;

use common::{add_archive, server::Server};
use rb_packages::{MemoryIndex, PrepareOptions, prepare};
use rb_task::TaskEvents;
use std::{collections::BTreeMap, fs};

#[test]
fn independent_projects_install_offline_from_a_shared_verified_cache() {
    let archives = tempfile::tempdir().unwrap();
    let mut index = MemoryIndex::default();
    add_archive(
        &mut index,
        archives.path(),
        "teacup",
        &[],
        &[],
        &[("lib/teacup.rb", b"TEACUP=42\n")],
    );
    let checksum = &index.packages["teacup"][0].checksum;
    let server = Server::new(BTreeMap::from([
        (
            "/info/teacup".into(),
            format!("---\n1.0.0 |checksum:{checksum}\n").into_bytes(),
        ),
        (
            "/gems/teacup-1.0.0.gem".into(),
            fs::read(index.archives.values().next().unwrap()).unwrap(),
        ),
    ]));
    let cache = tempfile::tempdir().unwrap();
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    let gemfile = format!("source '{}'\ngem 'teacup'\n", server.url);
    fs::write(first.path().join("Gemfile"), &gemfile).unwrap();
    let original = prepare(
        PrepareOptions::new(first.path(), common::ruby(), cache.path()),
        TaskEvents::default(),
    )
    .unwrap();
    let requests = server.requests.lock().unwrap().clone();
    assert_eq!(
        requests
            .iter()
            .filter(|path| path.as_str() == "/gems/teacup-1.0.0.gem")
            .count(),
        1
    );
    fs::write(second.path().join("Gemfile"), &gemfile).unwrap();
    fs::copy(
        first.path().join("Gemfile.lock"),
        second.path().join("Gemfile.lock"),
    )
    .unwrap();
    let mut options = PrepareOptions::new(second.path(), common::ruby(), cache.path());
    options.offline = true;
    let reused = prepare(options, TaskEvents::default()).unwrap();
    assert_eq!(*server.requests.lock().unwrap(), requests);
    assert_ne!(original.installed.gem_home, reused.installed.gem_home);
    for bundle in [&original.installed, &reused.installed] {
        assert_eq!(
            fs::read(bundle.gem_home.join("gems/teacup-1.0.0/lib/teacup.rb")).unwrap(),
            b"TEACUP=42\n"
        );
    }
}
