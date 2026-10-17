use super::*;
use base64::Engine;

#[test]
fn credentials_authenticate_metadata_and_archives_without_entering_source_or_cache() {
    let server = Server::new();
    let cache = tempfile::tempdir().unwrap();
    let archive = b"fixture archive";
    let sha = checksum(archive);
    server.responses.lock().unwrap().extend([
        (
            "/info/example".into(),
            format!("---\n1.0 |checksum:{sha}\n").into_bytes(),
        ),
        ("/versions".into(), b"---\nexample 1.0 abc\n".to_vec()),
        ("/gems/example-1.0.gem".into(), archive.to_vec()),
    ]);
    let client = Client::new(&server.source, cache.path(), false)
        .unwrap()
        .with_credentials("user:secret-token")
        .unwrap();
    assert_eq!(client.source(), server.source);
    assert_eq!(client.info("example").unwrap().len(), 1);
    assert!(client.names().unwrap().unwrap().contains("example"));
    client.download("example", "1.0", &sha).unwrap();
    let expected = format!(
        "authorization: basic {}",
        base64::engine::general_purpose::STANDARD
            .encode("user:secret-token")
            .to_lowercase()
    );
    for request in server.request_headers.lock().unwrap().iter() {
        assert!(request.to_lowercase().contains(&expected));
    }
    let info = cache
        .path()
        .join("index")
        .join(checksum(server.source.as_bytes()))
        .join("example.json");
    let cached = fs::read_to_string(info).unwrap();
    assert!(!cached.contains("secret-token"));
    assert!(!cached.contains("authorization"));
}

#[test]
fn credentials_are_not_forwarded_to_another_origin_on_redirect() {
    let server = Server::new();
    let destination = Server::new();
    server
        .statuses
        .lock()
        .unwrap()
        .insert("/info/example".into(), "302 Found".into());
    server.headers.lock().unwrap().insert(
        "/info/example".into(),
        format!("Location: {}info/example\r\n", destination.source),
    );
    destination.responses.lock().unwrap().insert(
        "/info/example".into(),
        format!("---\n1.0 |checksum:{}\n", "a".repeat(64)).into_bytes(),
    );
    let cache = tempfile::tempdir().unwrap();
    let client = Client::new(&server.source, cache.path(), false)
        .unwrap()
        .with_credentials("user:secret-token")
        .unwrap();
    client.info("example").unwrap();
    assert!(
        !destination.request_headers.lock().unwrap()[0]
            .to_lowercase()
            .contains("authorization:")
    );
}
