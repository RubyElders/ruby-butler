mod common;
use rb_gem_archive::read;
use serde_json::{Value, json};
use std::{fs, path::Path};

#[test]
fn captured_metadata_matches_rubygems_for_pure_native_and_java_archives() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    for name in ["rake-13.2.1", "bigdecimal-3.1.8", "json-2.7.2-java"] {
        let metadata = fs::read(root.join(format!("{name}.yml"))).unwrap();
        let gem = read(common::container(&common::members(&metadata)).as_slice()).unwrap();
        let expected: Value =
            serde_json::from_slice(&fs::read(root.join(format!("{name}.expected.json"))).unwrap())
                .unwrap();
        let actual = json!({
            "name": gem.id.name, "version": gem.id.version.to_string(), "platform": gem.id.platform.to_string(),
            "dependencies": gem.dependencies.into_iter().map(|(name, requirement)| (name, requirement.to_string())).collect::<std::collections::BTreeMap<_,_>>(),
            "ruby": gem.ruby.to_string(), "rubygems": gem.rubygems.to_string(),
            "require_paths": gem.require_paths, "executables": gem.executables,
            "bindir": gem.bindir, "extensions": gem.extensions, "files": gem.files,
            "metadata": gem.metadata
        });
        assert_eq!(actual, expected, "{name}");
    }
}
