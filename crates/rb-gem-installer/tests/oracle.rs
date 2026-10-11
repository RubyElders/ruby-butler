use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};

fn files(root: &Path, directory: &Path, found: &mut BTreeMap<String, String>) {
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files(root, &path, found);
        } else {
            let name = path
                .strip_prefix(root)
                .unwrap()
                .to_str()
                .unwrap()
                .replace('\\', "/");
            found.insert(
                name,
                format!("{:x}", Sha256::digest(fs::read(path).unwrap())),
            );
        }
    }
}

#[test]
fn installed_files_match_rubygems() {
    let archive = include_bytes!("fixtures/database_cleaner-2.0.2.gem");
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/database_cleaner-2.0.2.json")).unwrap();
    assert_eq!(format!("{:x}", Sha256::digest(archive)), oracle["sha256"]);
    let root = tempfile::tempdir().unwrap();
    let destination = root.path().join("gem");
    let metadata = rb_gem_installer::install(archive.as_slice(), &destination).unwrap();
    assert_eq!(metadata.id.full_name(), "database_cleaner-2.0.2");
    let mut actual = BTreeMap::new();
    files(&destination, &destination, &mut actual);
    let expected: BTreeMap<String, String> =
        serde_json::from_value(oracle["files"].clone()).unwrap();
    assert_eq!(actual, expected);
}
