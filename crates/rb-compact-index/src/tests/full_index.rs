use super::*;

#[test]
fn full_index_fallback_keeps_the_declared_source_and_caches_archives() {
    use alox_48::Value;
    use flate2::{Compression, write::GzEncoder};
    fn gzip(bytes: &[u8]) -> Vec<u8> {
        let mut writer = GzEncoder::new(Vec::new(), Compression::default());
        writer.write_all(bytes).unwrap();
        writer.finish().unwrap()
    }
    let server = Server::new();
    let entry = |name: &str, version: &str, platform: &str| {
        Value::Array(vec![
            Value::String(name.into()),
            Value::UserMarshal {
                class: "Gem::Version".into(),
                value: Box::new(Value::Array(vec![Value::String(version.into())])),
            },
            Value::String(platform.into()),
        ])
    };
    let index = Value::Array(vec![
        entry("custom", "1.0", "ruby"),
        entry("custom", "1.0", "x86_64-linux"),
    ]);
    server.responses.lock().unwrap().insert(
        "/specs.4.8.gz".into(),
        gzip(&alox_48::to_bytes(&index).unwrap()),
    );
    let metadata = b"--- !ruby/object:Gem::Specification\nname: custom\nversion: !ruby/object:Gem::Version\n  version: '1.0'\nrequired_ruby_version: !ruby/object:Gem::Requirement\n  requirements:\n  - - '>='\n    - !ruby/object:Gem::Version\n      version: '3.2'\ndependencies:\n- !ruby/object:Gem::Dependency\n  name: dep\n  type: :runtime\n  requirement: !ruby/object:Gem::Requirement\n    requirements:\n    - - '~>'\n      - !ruby/object:Gem::Version\n        version: '2.0'\n- !ruby/object:Gem::Dependency\n  name: test_only\n  type: :development\n";
    let bytes = gzip(metadata);
    let mut archive = tar::Builder::new(Vec::new());
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    archive
        .append_data(&mut header, "metadata.gz", &bytes[..])
        .unwrap();
    let archive = archive.into_inner().unwrap();
    server
        .responses
        .lock()
        .unwrap()
        .insert("/gems/custom-1.0.gem".into(), archive.clone());
    let bytes = gzip(&[metadata.as_slice(), b"platform: x86_64-linux\n"].concat());
    let mut platform_archive = tar::Builder::new(Vec::new());
    let mut header = tar::Header::new_gnu();
    header.set_size(bytes.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();
    platform_archive
        .append_data(&mut header, "metadata.gz", &bytes[..])
        .unwrap();
    server.responses.lock().unwrap().insert(
        "/gems/custom-1.0-x86_64-linux.gem".into(),
        platform_archive.into_inner().unwrap(),
    );
    let cache = tempfile::tempdir().unwrap();
    let client = Client::new(&server.source, cache.path(), false).unwrap();
    let gems = client.info("custom").unwrap();
    assert_eq!(gems.len(), 2);
    assert_eq!(gems[1].version, "1.0-x86_64-linux");
    assert_eq!(
        gems[0].dependencies,
        BTreeMap::from([("dep".into(), "~> 2.0".into())])
    );
    assert_eq!(gems[0].metadata["ruby"], ">= 3.2");
    assert_eq!(gems[0].metadata["checksum"], checksum(&archive));
    let requests = server.requests.lock().unwrap().len();
    let offline = Client::new(&server.source, cache.path(), true).unwrap();
    assert_eq!(offline.info("custom").unwrap(), gems);
    let path = offline
        .download("custom", "1.0", &gems[0].metadata["checksum"])
        .unwrap();
    assert_eq!(fs::read(path).unwrap(), archive);
    assert_eq!(server.requests.lock().unwrap().len(), requests);
}

#[test]
fn missing_compact_gem_does_not_fetch_the_full_registry() {
    let server = Server::new();
    server
        .responses
        .lock()
        .unwrap()
        .insert("/versions".into(), b"---\n".to_vec());
    let cache = tempfile::tempdir().unwrap();
    assert!(
        Client::new(&server.source, cache.path(), false)
            .unwrap()
            .info("missing")
            .is_err()
    );
    assert!(
        !server
            .requests
            .lock()
            .unwrap()
            .iter()
            .any(|path| path.contains("specs.4.8"))
    );
}
