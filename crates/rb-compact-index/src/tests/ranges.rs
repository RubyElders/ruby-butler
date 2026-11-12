use super::*;
use base64::Engine;

fn response(status: &str, body: &[u8], extra: &str) -> Vec<u8> {
    let mut bytes = format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nETag: \"fixture\"\r\n{extra}Connection: close\r\n\r\n", body.len()).into_bytes();
    bytes.extend_from_slice(body);
    bytes
}

fn serve(replies: Vec<Vec<u8>>) -> (String, thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let source = format!("http://{}/", listener.local_addr().unwrap());
    let thread = thread::spawn(move || {
        let mut requests = Vec::new();
        for reply in replies {
            let started = std::time::Instant::now();
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(
                            started.elapsed() < Duration::from_secs(5),
                            "Missing expected request"
                        );
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(error) => panic!("{error}"),
                }
            };
            let mut request = Vec::new();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            while !request.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                assert_eq!(stream.read(&mut byte).unwrap(), 1);
                request.push(byte[0]);
            }
            requests.push(String::from_utf8(request).unwrap());
            stream.write_all(&reply).unwrap();
        }
        requests
    });
    (source, thread)
}

#[test]
fn versions_refresh_appends_a_verified_range() {
    let old = b"---\napp 1.0 a\n";
    let new = b"---\napp 1.0 a\nother 1.0 b\n";
    let start = old.len() - 1;
    let digest = base64::engine::general_purpose::STANDARD.encode(Sha256::digest(new));
    let (source, server) = serve(vec![
        response("200 OK", old, ""),
        response(
            "206 Partial Content",
            &new[start..],
            &format!(
                "Content-Range: bytes {start}-{}/{}\r\nRepr-Digest: sha-256=:{digest}:\r\n",
                new.len() - 1,
                new.len()
            ),
        ),
    ]);
    let cache = tempfile::tempdir().unwrap();
    assert_eq!(
        Client::new(&source, cache.path(), false)
            .unwrap()
            .names()
            .unwrap()
            .unwrap(),
        ["app".into()].into()
    );
    assert_eq!(
        Client::new(&source, cache.path(), false)
            .unwrap()
            .names()
            .unwrap()
            .unwrap(),
        ["app".into(), "other".into()].into()
    );
    let requests = server.join().unwrap();
    assert!(
        requests[1]
            .to_lowercase()
            .contains(&format!("range: bytes={start}-"))
    );
    assert!(
        requests[1]
            .to_lowercase()
            .contains("if-none-match: \"fixture\"")
    );
    assert_eq!(
        Client::new(&source, cache.path(), true)
            .unwrap()
            .names()
            .unwrap()
            .unwrap(),
        ["app".into(), "other".into()].into()
    );
}

#[test]
fn rejected_or_invalid_ranges_retry_a_full_request() {
    let old = b"---\napp 1.0 a\n";
    let new = b"---\nother 1.0 b\n";
    for reply in [
        response("416 Range Not Satisfiable", b"", ""),
        response(
            "206 Partial Content",
            b"wrong",
            "Content-Range: bytes 0-4/5\r\n",
        ),
        response(
            "206 Partial Content",
            b"x",
            &format!(
                "Content-Range: bytes {}-{}/{}\r\n",
                old.len() - 1,
                old.len() - 1,
                old.len()
            ),
        ),
        response(
            "206 Partial Content",
            b"\n",
            &format!(
                "Content-Range: bytes {}-{}/{}\r\nRepr-Digest: sha-256=:AAAA:\r\n",
                old.len() - 1,
                old.len() - 1,
                old.len()
            ),
        ),
    ] {
        let (source, server) = serve(vec![
            response("200 OK", old, ""),
            reply,
            response("200 OK", new, ""),
        ]);
        let cache = tempfile::tempdir().unwrap();
        Client::new(&source, cache.path(), false)
            .unwrap()
            .names()
            .unwrap();
        assert_eq!(
            Client::new(&source, cache.path(), false)
                .unwrap()
                .names()
                .unwrap()
                .unwrap(),
            ["other".into()].into()
        );
        let requests = server.join().unwrap();
        assert!(requests[1].to_lowercase().contains("range: bytes="));
        assert!(!requests[2].to_lowercase().contains("range:"));
        assert!(!requests[2].to_lowercase().contains("if-none-match:"));
    }
}

#[test]
fn not_modified_without_cached_versions_is_an_error() {
    let server = Server::new();
    server
        .statuses
        .lock()
        .unwrap()
        .insert("/versions".into(), "304 Not Modified".into());
    let cache = tempfile::tempdir().unwrap();
    let error = Client::new(&server.source, cache.path(), false)
        .unwrap()
        .names()
        .unwrap_err();
    assert!(error.to_string().contains("304 without cached versions"));
    assert!(
        Client::new(&server.source, cache.path(), true)
            .unwrap()
            .names()
            .is_err()
    );
}
