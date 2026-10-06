use super::*;
use base64::Engine;

const SOURCES: &[(&str, &str, &str)] = &[
    (
        "",
        "nokogiri",
        include_str!("../../tests/fixtures/rubygems-nokogiri.info"),
    ),
    (
        "",
        "activerecord-import",
        include_str!("../../tests/fixtures/rubygems-activerecord-import.info"),
    ),
    (
        "",
        "rake",
        include_str!("../../tests/fixtures/rubygems-rake.info"),
    ),
    (
        "mirror/",
        "rake",
        include_str!("../../tests/fixtures/gem-coop-rake.info"),
    ),
    (
        "@dry/",
        "dry-core",
        include_str!("../../tests/fixtures/gem-coop-dry-core.info"),
    ),
    (
        "@kaspth/",
        "oaken",
        include_str!("../../tests/fixtures/gem-coop-oaken.info"),
    ),
];

#[test]
fn captured_records_preserve_platforms_prereleases_and_requirements() {
    let nokogiri = parse("nokogiri", SOURCES[0].2).unwrap();
    assert_eq!(
        nokogiri
            .iter()
            .map(|gem| gem.version.as_str())
            .collect::<Vec<_>>(),
        [
            "1.5.1.rc1-java",
            "1.19.4-aarch64-linux-musl",
            "1.19.4-x64-mingw-ucrt",
            "1.19.4-x86_64-linux-gnu",
            "1.19.4",
        ]
    );
    assert!(nokogiri[0].dependencies.is_empty());
    assert_eq!(nokogiri[1].metadata["ruby"], "< 4.1.dev&>= 3.2");
    assert_eq!(nokogiri[1].dependencies["racc"], "~> 1.4");
    assert_eq!(nokogiri[4].dependencies["mini_portile2"], "~> 2.8.2");
    assert!(!nokogiri[1].dependencies.contains_key("mini_portile2"));

    let imports = parse("activerecord-import", SOURCES[1].2).unwrap();
    assert_eq!(imports.len(), 3);
    assert_eq!(imports[0].version, "0.2.8.rc1");
    assert_eq!(
        imports[0].dependencies["activerecord"],
        "~> 3.0pre, ~> 3.0pre"
    );
    assert_eq!(
        imports[1].dependencies["activerecord"],
        "~> 3.0pre, ~> 3.0pre"
    );
    assert_eq!(
        parse("rake", SOURCES[2].2).unwrap(),
        parse("rake", SOURCES[3].2).unwrap()
    );

    let dry = parse("dry-core", SOURCES[4].2).unwrap();
    assert_eq!(dry.len(), 2);
    assert_eq!(dry[1].version, "1.2.0");
    assert_eq!(dry[1].dependencies["zeitwerk"], "~> 2.6");
    assert_eq!(dry[1].metadata["licenses"], "MIT");
    assert_eq!(dry[1].metadata["published_at"], "2025-12-28T02:24:36.645Z");
}

#[test]
fn captured_sources_revalidate_and_reuse_verified_metadata_offline() {
    for &(prefix, name, body) in SOURCES {
        let server = Server::new();
        let path = format!("/{prefix}info/{name}");
        server
            .responses
            .lock()
            .unwrap()
            .insert(path.clone(), body.as_bytes().to_vec());
        let digest =
            base64::engine::general_purpose::STANDARD.encode(Sha256::digest(body.as_bytes()));
        server
            .headers
            .lock()
            .unwrap()
            .insert(path.clone(), format!("Repr-Digest: sha-256=:{digest}:\r\n"));
        let cache = tempfile::tempdir().unwrap();
        let source = format!("{}{prefix}", server.source);
        let client = Client::new(&source, cache.path(), false).unwrap();
        let expected = parse(name, body).unwrap();
        assert_eq!(client.info(name).unwrap(), expected);
        server.responses.lock().unwrap().remove(&path);
        assert_eq!(client.info(name).unwrap(), expected);
        assert_eq!(
            server.requests.lock().unwrap().as_slice(),
            [path.clone(), path]
        );
        drop(server);
        assert_eq!(
            Client::new(&source, cache.path(), true)
                .unwrap()
                .info(name)
                .unwrap(),
            expected
        );
    }
}

#[test]
fn redirects_fetch_and_cache_the_final_metadata() {
    let server = Server::new();
    let body = SOURCES[2].2;
    server
        .statuses
        .lock()
        .unwrap()
        .insert("/info/rake".into(), "302 Found".into());
    server.headers.lock().unwrap().insert(
        "/info/rake".into(),
        "Location: /upstream/info/rake\r\n".into(),
    );
    server
        .responses
        .lock()
        .unwrap()
        .insert("/upstream/info/rake".into(), body.as_bytes().to_vec());
    let cache = tempfile::tempdir().unwrap();
    let expected = parse("rake", body).unwrap();
    assert_eq!(
        Client::new(&server.source, cache.path(), false)
            .unwrap()
            .info("rake")
            .unwrap(),
        expected
    );
    assert_eq!(
        server.requests.lock().unwrap().as_slice(),
        ["/info/rake", "/upstream/info/rake"]
    );
    assert_eq!(
        Client::new(&server.source, cache.path(), true)
            .unwrap()
            .info("rake")
            .unwrap(),
        expected
    );
}

#[test]
fn authentication_errors_do_not_trigger_fallback_or_replace_cached_metadata() {
    for status in ["401 Unauthorized", "403 Forbidden"] {
        let server = Server::new();
        server.revalidate.store(false, Ordering::Relaxed);
        server
            .responses
            .lock()
            .unwrap()
            .insert("/info/rake".into(), SOURCES[2].2.as_bytes().to_vec());
        let cache = tempfile::tempdir().unwrap();
        let client = Client::new(&server.source, cache.path(), false).unwrap();
        let expected = client.info("rake").unwrap();
        server
            .statuses
            .lock()
            .unwrap()
            .insert("/info/rake".into(), status.into());
        let error = client.info("rake").unwrap_err();
        assert!(error.to_string().contains(&status[..3]), "{error}");
        assert_eq!(
            server.requests.lock().unwrap().as_slice(),
            ["/info/rake", "/info/rake"]
        );
        assert_eq!(
            Client::new(&server.source, cache.path(), true)
                .unwrap()
                .info("rake")
                .unwrap(),
            expected
        );
    }
}

#[test]
fn malformed_digest_does_not_cache_metadata() {
    let server = Server::new();
    server
        .responses
        .lock()
        .unwrap()
        .insert("/info/rake".into(), SOURCES[2].2.as_bytes().to_vec());
    server.headers.lock().unwrap().insert(
        "/info/rake".into(),
        "Repr-Digest: sha-256=:not-base64!:\r\n".into(),
    );
    let cache = tempfile::tempdir().unwrap();
    assert!(
        Client::new(&server.source, cache.path(), false)
            .unwrap()
            .info("rake")
            .is_err()
    );
    assert!(
        Client::new(&server.source, cache.path(), true)
            .unwrap()
            .info("rake")
            .is_err()
    );
}
