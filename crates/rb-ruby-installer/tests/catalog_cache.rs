use rb_ruby_installer::Installer;
use rb_tests::HttpServer as Server;
use std::fs;

#[test]
fn catalog_revalidation_uses_etag_and_304() {
    let root = tempfile::tempdir().unwrap();
    let server = Server::new();
    server.route(
        "/catalog",
        200,
        br#"{"tag_name":"fixture","assets":[]}"#.as_slice(),
    );
    let installer = Installer::new(root.path())
        .unwrap()
        .with_catalog_url(format!("{}/catalog", server.url));
    installer.catalog().unwrap();
    expire(root.path());
    server.route("/catalog", 304, vec![]);
    assert_eq!(installer.catalog().unwrap().tag_name, "fixture");
    assert!(
        server
            .requests()
            .last()
            .unwrap()
            .to_lowercase()
            .contains("if-none-match: \"fixture\"")
    );
    assert_eq!(installer.catalog().unwrap().tag_name, "fixture");
    assert_eq!(server.requests().len(), 2);
}

fn expire(path: &std::path::Path) {
    let file = path.join("catalog.json");
    let mut cached: serde_json::Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    cached["fetched"] = serde_json::json!({"secs_since_epoch":0,"nanos_since_epoch":0});
    fs::write(file, serde_json::to_vec(&cached).unwrap()).unwrap();
}

#[test]
fn refreshed_catalog_replaces_stale_or_corrupt_metadata() {
    let root = tempfile::tempdir().unwrap();
    let server = Server::new();
    server.route(
        "/catalog",
        200,
        br#"{"tag_name":"first","assets":[]}"#.as_slice(),
    );
    let installer = Installer::new(root.path())
        .unwrap()
        .with_catalog_url(format!("{}/catalog", server.url));
    fs::write(root.path().join("catalog.json"), "corrupt cache").unwrap();
    assert_eq!(installer.catalog().unwrap().tag_name, "first");
    server.route(
        "/catalog",
        200,
        br#"{"tag_name":"second","assets":[]}"#.as_slice(),
    );
    expire(root.path());
    assert_eq!(installer.catalog().unwrap().tag_name, "second");
    server.route("/catalog", 500, vec![]);
    assert_eq!(installer.catalog().unwrap().tag_name, "second");
    assert_eq!(server.requests().len(), 2);
}

#[test]
fn changing_catalog_sources_does_not_reuse_release_or_etag() {
    let root = tempfile::tempdir().unwrap();
    for tag in ["first", "second"] {
        let server = Server::new();
        server.route(
            "/catalog",
            200,
            format!(r#"{{"tag_name":"{tag}","assets":[]}}"#),
        );
        let installer = Installer::new(root.path())
            .unwrap()
            .with_catalog_url(format!("{}/catalog", server.url));
        assert_eq!(installer.catalog().unwrap().tag_name, tag);
        assert!(
            !server.requests()[0]
                .to_ascii_lowercase()
                .contains("if-none-match:")
        );
    }
}

#[test]
fn invalid_or_unbounded_responses_do_not_publish_cache_entries() {
    for (status, body, expected) in [
        (500, vec![], "500"),
        (304, vec![], "304 without a cached release"),
        (200, b"not json".to_vec(), "Invalid Ruby release catalog"),
        (
            200,
            vec![b' '; 8 * 1024 * 1024 + 1],
            "exceeds the size limit",
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        let server = Server::new();
        server.route("/catalog", status, body);
        let installer = Installer::new(root.path())
            .unwrap()
            .with_catalog_url(format!("{}/catalog", server.url));
        let error = installer.catalog().unwrap_err().to_string();
        assert!(error.contains(expected), "{error}");
        assert!(!root.path().join("catalog.json").exists());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }
}
