use super::*;

#[test]
fn names_are_scoped_cached_offline_and_do_not_collide_with_gem_metadata() {
    let server = Server::new();
    server.responses.lock().unwrap().insert(
        "/versions".into(),
        b"created_at: fixture\n---\naddon 1.0 a\nchild 1.0 \naddon 2.0 c\n".to_vec(),
    );
    server.responses.lock().unwrap().insert(
        "/info/versions".into(),
        format!("---\n1.0 |checksum:{}\n", "a".repeat(64)).into_bytes(),
    );
    let cache = tempfile::tempdir().unwrap();
    let client = Client::new(&server.source, cache.path(), false).unwrap();
    let names = client.names().unwrap().unwrap();
    assert_eq!(names, ["addon".into(), "child".into()].into());
    assert_eq!(client.info("versions").unwrap()[0].version, "1.0");
    let offline = Client::new(&server.source, cache.path(), true).unwrap();
    drop(server);
    assert_eq!(offline.names().unwrap().unwrap(), names);
    assert_eq!(offline.info("versions").unwrap()[0].version, "1.0");
    assert!(
        Client::new("https://other.example/", cache.path(), true)
            .unwrap()
            .names()
            .is_err()
    );
}

#[test]
fn unsupported_names_endpoint_is_remembered_without_hiding_other_failures() {
    let server = Server::new();
    let cache = tempfile::tempdir().unwrap();
    let client = Client::new(&server.source, cache.path(), false).unwrap();
    assert!(client.names().unwrap().is_none());
    assert_eq!(*server.requests.lock().unwrap(), ["/versions"]);
    drop(server);
    assert!(
        Client::new(client.source(), cache.path(), true)
            .unwrap()
            .names()
            .unwrap()
            .is_none()
    );
    for status in [
        "401 Unauthorized",
        "403 Forbidden",
        "500 Internal Server Error",
    ] {
        let server = Server::new();
        server
            .statuses
            .lock()
            .unwrap()
            .insert("/versions".into(), status.into());
        let cache = tempfile::tempdir().unwrap();
        assert!(
            Client::new(&server.source, cache.path(), false)
                .unwrap()
                .names()
                .is_err()
        );
        assert!(
            Client::new(&server.source, cache.path(), true)
                .unwrap()
                .names()
                .is_err()
        );
    }
}

#[test]
fn malformed_names_do_not_replace_the_cached_registry_listing() {
    let server = Server::new();
    server.revalidate.store(false, Ordering::Relaxed);
    let cache = tempfile::tempdir().unwrap();
    let client = Client::new(&server.source, cache.path(), false).unwrap();
    server
        .responses
        .lock()
        .unwrap()
        .insert("/versions".into(), b"---\naddon 1.0 a\n".to_vec());
    let expected = client.names().unwrap();
    for body in [
        "not a versions index",
        "---\naddon",
        "---\n../escape 1.0 a\n",
    ] {
        server
            .responses
            .lock()
            .unwrap()
            .insert("/versions".into(), body.as_bytes().to_vec());
        assert!(client.names().is_err());
        assert_eq!(
            Client::new(&server.source, cache.path(), true)
                .unwrap()
                .names()
                .unwrap(),
            expected
        );
    }
    server
        .responses
        .lock()
        .unwrap()
        .insert("/versions".into(), b"---\nchanged 1.0 b\n".to_vec());
    server
        .headers
        .lock()
        .unwrap()
        .insert("/versions".into(), "Repr-Digest: sha-256=:AAAA:\r\n".into());
    assert!(
        client
            .names()
            .unwrap_err()
            .to_string()
            .contains("Index digest mismatch")
    );
    assert_eq!(
        Client::new(&server.source, cache.path(), true)
            .unwrap()
            .names()
            .unwrap(),
        expected
    );
}
