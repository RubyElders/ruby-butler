use super::*;
use md5::Md5;

fn info(version: &str) -> String {
    format!("---\n{version} |checksum:{}\n", "a".repeat(64))
}

fn versions(entries: &[(&str, &str)]) -> Vec<u8> {
    let mut body = "---\n".to_owned();
    for (name, info) in entries {
        body.push_str(&format!("{name} 1.0 {:x}\n", Md5::digest(info.as_bytes())));
    }
    body.into_bytes()
}

#[test]
fn unchanged_metadata_needs_only_one_versions_request_across_parallel_clones() {
    let server = Server::new();
    let body = info("1.0");
    server.responses.lock().unwrap().extend([
        (
            "/versions".into(),
            versions(&[("app", &body), ("other", &body)]),
        ),
        ("/info/app".into(), body.as_bytes().to_vec()),
        ("/info/other".into(), body.as_bytes().to_vec()),
    ]);
    let cache = tempfile::tempdir().unwrap();
    let first = Client::new(&server.source, cache.path(), false).unwrap();
    for name in ["app", "other"] {
        assert_eq!(first.info(name).unwrap()[0].version, "1.0");
    }
    assert_eq!(
        *server.requests.lock().unwrap(),
        ["/versions", "/info/app", "/info/other"]
    );
    server.requests.lock().unwrap().clear();
    server.request_headers.lock().unwrap().clear();
    let client = Client::new(&server.source, cache.path(), false).unwrap();
    thread::scope(|scope| {
        for name in ["app", "other", "app", "other"] {
            let client = client.clone();
            scope.spawn(move || {
                assert_eq!(client.info(name).unwrap()[0].version, "1.0");
                assert_eq!(client.info(name).unwrap()[0].version, "1.0");
            });
        }
    });
    assert_eq!(*server.requests.lock().unwrap(), ["/versions"]);
    let requests = server.request_headers.lock().unwrap();
    assert!(
        requests[0]
            .to_lowercase()
            .contains("if-none-match: \"fixture\"")
    );
    assert!(requests[0].to_lowercase().contains("range: bytes="));
}

#[test]
fn changed_missing_and_damaged_metadata_are_fetched_without_refreshing_unchanged_entries() {
    let server = Server::new();
    let old = info("1.0");
    server.responses.lock().unwrap().extend([
        (
            "/versions".into(),
            versions(&[("app", &old), ("other", &old)]),
        ),
        ("/info/app".into(), old.as_bytes().to_vec()),
        ("/info/other".into(), old.as_bytes().to_vec()),
    ]);
    let cache = tempfile::tempdir().unwrap();
    let client = Client::new(&server.source, cache.path(), false).unwrap();
    client.info("app").unwrap();
    client.info("other").unwrap();
    let new = info("2.0");
    server.revalidate.store(false, Ordering::Relaxed);
    server.responses.lock().unwrap().extend([
        (
            "/versions".into(),
            versions(&[("app", &new), ("other", &old), ("missing", &old)]),
        ),
        ("/info/app".into(), new.as_bytes().to_vec()),
        ("/info/missing".into(), old.as_bytes().to_vec()),
    ]);
    server.requests.lock().unwrap().clear();
    let client = Client::new(&server.source, cache.path(), false).unwrap();
    assert_eq!(client.info("app").unwrap()[0].version, "2.0");
    assert_eq!(client.info("other").unwrap()[0].version, "1.0");
    assert_eq!(client.info("missing").unwrap()[0].version, "1.0");
    assert_eq!(
        *server.requests.lock().unwrap(),
        ["/versions", "/info/app", "/info/missing"]
    );
    let path = cache
        .path()
        .join("index")
        .join(checksum(server.source.as_bytes()))
        .join("app.json");
    let mut cached: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    cached["body"] = info("0.0").into();
    fs::write(path, serde_json::to_vec(&cached).unwrap()).unwrap();
    server.revalidate.store(true, Ordering::Relaxed);
    let client = Client::new(&server.source, cache.path(), false).unwrap();
    assert_eq!(client.info("app").unwrap()[0].version, "2.0");
    assert!(
        !server
            .request_headers
            .lock()
            .unwrap()
            .last()
            .unwrap()
            .to_lowercase()
            .contains("if-none-match")
    );
}

#[test]
fn last_versions_entry_controls_metadata_freshness_and_missing_checksum_revalidates() {
    let server = Server::new();
    let body = info("1.0");
    let mut index = versions(&[("app", &info("0.0")), ("app", &body)]);
    index.extend_from_slice(b"other 1.0\n");
    server.responses.lock().unwrap().extend([
        ("/versions".into(), index),
        ("/info/app".into(), body.as_bytes().to_vec()),
        ("/info/other".into(), body.as_bytes().to_vec()),
        ("/info/unlisted".into(), body.as_bytes().to_vec()),
    ]);
    let cache = tempfile::tempdir().unwrap();
    for _ in 0..2 {
        let client = Client::new(&server.source, cache.path(), false).unwrap();
        assert_eq!(client.info("app").unwrap()[0].version, "1.0");
        assert_eq!(client.info("other").unwrap()[0].version, "1.0");
        assert_eq!(client.info("unlisted").unwrap()[0].version, "1.0");
    }
    assert_eq!(
        *server.requests.lock().unwrap(),
        [
            "/versions",
            "/info/app",
            "/info/other",
            "/info/unlisted",
            "/versions",
            "/info/other"
        ]
    );
}
