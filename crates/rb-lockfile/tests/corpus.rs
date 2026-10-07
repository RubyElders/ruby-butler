use rb_lockfile::Lockfile;
use std::{collections::BTreeSet, fs, path::Path};

#[test]
fn real_bundler_locks_round_trip_without_losing_metadata() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut count = 0;
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|ext| ext != "lock") {
            continue;
        }
        let lock = Lockfile::parse(&fs::read_to_string(&path).unwrap()).unwrap();
        let rendered = lock.render().unwrap();
        let again = Lockfile::parse(&rendered).unwrap();
        assert_eq!(again.render().unwrap(), rendered, "{}", path.display());
        assert_eq!(again.dependencies, lock.dependencies);
        assert_eq!(again.checksums, lock.checksums);
        assert_eq!(again.ruby_version, lock.ruby_version);
        assert_eq!(again.bundled_with, lock.bundled_with);
        assert_eq!(again.sources.len(), lock.sources.len());
        for source in &lock.sources {
            let restored = again
                .sources
                .iter()
                .find(|s| {
                    s.kind == source.kind
                        && s.remotes == source.remotes
                        && s.options == source.options
                        && s.packages.len() == source.packages.len()
                        && source
                            .packages
                            .iter()
                            .all(|package| s.packages.contains(package))
                })
                .unwrap();
            assert_eq!(restored.packages.len(), source.packages.len());
            for package in &source.packages {
                assert!(restored.packages.contains(package));
            }
        }
        assert_eq!(
            again.platforms.iter().collect::<BTreeSet<_>>(),
            lock.platforms.iter().collect()
        );
        count += 1;
    }
    assert_eq!(count, 15);
}
