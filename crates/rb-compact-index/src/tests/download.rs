use super::*;

#[test]
fn downloads_are_shared_verified_and_available_offline() {
    let server = Server::new();
    let archive = b"fixture archive";
    let expected = checksum(archive);
    server
        .responses
        .lock()
        .unwrap()
        .insert("/gems/app-1.0.gem".into(), archive.to_vec());
    let cache = tempfile::tempdir().unwrap();
    let client = Client::new(&server.source, cache.path(), false).unwrap();
    thread::scope(|scope| {
        let first = scope.spawn(|| client.download("app", "1.0", &expected).unwrap());
        let second = scope.spawn(|| client.download("app", "1.0", &expected).unwrap());
        assert_eq!(first.join().unwrap(), second.join().unwrap());
    });
    assert_eq!(server.requests.lock().unwrap().len(), 1);
    let offline = Client::new(&server.source, cache.path(), true).unwrap();
    let path = offline.download("app", "1.0", &expected).unwrap();
    fs::write(&path, "broken").unwrap();
    assert!(offline.download("app", "1.0", &expected).is_err());
    client.download("app", "1.0", &expected).unwrap();
    assert_eq!(fs::read(&path).unwrap(), archive);
    assert!(client.download("app", "1.0", &"a".repeat(64)).is_err());
    assert!(
        !client
            .archive_directory()
            .join(format!("{}.gem", "a".repeat(64)))
            .exists()
    );
}

#[test]
fn large_archives_are_streamed_verified_and_reused_offline() {
    let cache = tempfile::tempdir().unwrap();
    let block = vec![b'x'; 64 * 1024];
    let blocks = 257 * 16;
    let mut digest = Sha256::new();
    for _ in 0..blocks {
        digest.update(&block);
    }
    let expected = format!("{:x}", digest.finalize());
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let source = format!("http://{}/", listener.local_addr().unwrap());
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = Vec::new();
        let mut byte = [0];
        while !request.ends_with(b"\r\n\r\n") {
            stream.read_exact(&mut byte).unwrap();
            request.push(byte[0]);
        }
        write!(
            stream,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            block.len() * blocks
        )
        .unwrap();
        for _ in 0..blocks {
            if stream.write_all(&block).is_err() {
                break;
            }
        }
    });
    let client = Client::new(&source, cache.path(), false).unwrap();
    let result = client.download("large", "1.0", &expected);
    server.join().unwrap();
    let path = result.unwrap();
    assert_eq!(fs::metadata(&path).unwrap().len(), 257 * 1024 * 1024);
    let offline = Client::new(&source, cache.path(), true).unwrap();
    assert_eq!(offline.download("large", "1.0", &expected).unwrap(), path);
}

#[test]
fn offline_archive_miss_does_not_contact_the_registry() {
    let server = Server::new();
    let cache = tempfile::tempdir().unwrap();
    let client = Client::new(&server.source, cache.path(), true).unwrap();
    assert!(
        client
            .download("app", "1.0", &"a".repeat(64))
            .unwrap_err()
            .to_string()
            .contains("offline")
    );
    assert!(server.requests.lock().unwrap().is_empty());
}

#[test]
fn equivalent_source_urls_share_archives_but_other_sources_do_not() {
    let cache = tempfile::tempdir().unwrap();
    let canonical = Client::new("https://example.com/registry/", cache.path(), true).unwrap();
    let expected = checksum(b"archive");
    fs::create_dir_all(canonical.archive_directory()).unwrap();
    let path = canonical
        .archive_directory()
        .join(format!("{expected}.gem"));
    fs::write(&path, b"archive").unwrap();
    for source in [
        "HTTPS://EXAMPLE.COM:443/registry",
        "https://example.com/old/../registry/",
    ] {
        let client = Client::new(source, cache.path(), true).unwrap();
        assert_eq!(
            client
                .download("app", "1.0", &expected.to_uppercase())
                .unwrap(),
            path
        );
    }
    for source in [
        "http://example.com/registry",
        "https://example.com:8443/registry",
        "https://example.com/other",
        "https://example.com/Registry",
    ] {
        let client = Client::new(source, cache.path(), true).unwrap();
        assert_ne!(client.archive_directory(), canonical.archive_directory());
        assert!(client.download("app", "1.0", &expected).is_err());
    }
}

#[test]
fn independent_clients_reuse_the_same_verified_archive() {
    let server = Server::new();
    let archive = b"shared archive";
    let expected = checksum(archive);
    server
        .responses
        .lock()
        .unwrap()
        .insert("/gems/app-1.0.gem".into(), archive.to_vec());
    let cache = tempfile::tempdir().unwrap();
    let first = Client::new(server.source.trim_end_matches('/'), cache.path(), false).unwrap();
    let second = Client::new(&server.source, cache.path(), false).unwrap();
    let path = first.download("app", "1.0", &expected).unwrap();
    assert_eq!(second.download("app", "1.0", &expected).unwrap(), path);
    assert_eq!(server.requests.lock().unwrap().len(), 1);
}
