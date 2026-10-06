use super::*;

#[test]
fn parses_info_without_resolver_or_platform_policy() {
    let body = format!(
        "created_at: now\n---\n1.0 shared:>= 1&< 3|checksum:{},ruby:>= 3.2,rubygems:>= 2,extra:kept\n1.0-x86_64-linux |checksum:{}\n",
        "a".repeat(64),
        "b".repeat(64)
    );
    let records = parse("app", &body).unwrap();
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].dependencies["shared"], ">= 1, < 3");
    assert_eq!(records[0].metadata["ruby"], ">= 3.2");
    assert_eq!(records[0].metadata["extra"], "kept");
    assert_eq!(records[1].version, "1.0-x86_64-linux");
}

#[test]
fn repeated_dependencies_preserve_all_requirements() {
    for (dependencies, expected) in [
        (
            "activerecord:~> 3.0pre,activerecord:~> 3.0pre",
            "~> 3.0pre, ~> 3.0pre",
        ),
        (
            "activerecord:>= 1&< 4,activerecord:!= 2,activerecord:< 3",
            ">= 1, < 4, != 2, < 3",
        ),
        ("activerecord:>= 2,activerecord:< 2", ">= 2, < 2"),
    ] {
        let body = format!(
            "---\n0.2.8 {dependencies}|checksum:{}\n1.0 |checksum:{}\n",
            "a".repeat(64),
            "b".repeat(64)
        );
        let records = parse("activerecord-import", &body).unwrap();
        assert_eq!(records.len(), 2);
        assert_eq!(records[0].dependencies["activerecord"], expected);
        assert!(records[1].dependencies.is_empty());
    }
}

#[test]
fn rejects_malformed_info_and_unsafe_request_components() {
    for body in [
        "1.0 |checksum:abc",
        "---\n1.0 |checksum:abc\n",
        "---\n1.0 |ruby:>= 0\n",
        "---\n1.0 a:1,a:2|checksum:abc\n",
    ] {
        assert!(parse("app", body).is_err());
    }
    let temp = tempfile::tempdir().unwrap();
    for source in [
        "file:///tmp/index",
        "https://user:pass@example.com",
        "https://example.com?query",
        "https://example.com#fragment",
    ] {
        assert!(Client::new(source, temp.path(), false).is_err());
    }
    let client = Client::new("https://example.com", temp.path(), true).unwrap();
    assert_eq!(client.source(), "https://example.com/");
    assert!(client.info("../escape").is_err());
    assert!(
        client
            .download("app", "../../escape", &"a".repeat(64))
            .is_err()
    );
}

#[test]
fn metadata_revalidates_and_works_offline() {
    let server = Server::new();
    let body = format!("---\n1.0 |checksum:{}\n", "a".repeat(64));
    server
        .responses
        .lock()
        .unwrap()
        .insert("/info/app".into(), body.into_bytes());
    let cache = tempfile::tempdir().unwrap();
    let client = Client::new(&server.source, cache.path(), false).unwrap();
    let first = client.info("app").unwrap();
    server.responses.lock().unwrap().remove("/info/app");
    assert_eq!(client.info("app").unwrap(), first);
    assert_eq!(server.requests.lock().unwrap().len(), 2);
    let offline = Client::new(&server.source, cache.path(), true).unwrap();
    drop(server);
    assert_eq!(offline.info("app").unwrap(), first);
    assert!(offline.info("missing").is_err());
    let other = Client::new("https://other.example", cache.path(), true).unwrap();
    assert!(other.info("app").is_err());
}

#[test]
fn verified_metadata_survives_invalid_replacements() {
    use base64::Engine;

    let server = Server::new();
    server.revalidate.store(false, Ordering::Relaxed);
    let body = format!("---\n1.0 |checksum:{}\n", "a".repeat(64));
    let digest = base64::engine::general_purpose::STANDARD.encode(Sha256::digest(body.as_bytes()));
    server
        .responses
        .lock()
        .unwrap()
        .insert("/info/app".into(), body.into_bytes());
    server.headers.lock().unwrap().insert(
        "/info/app".into(),
        format!("Repr-Digest: sha-256=:{digest}:\r\n"),
    );
    let cache = tempfile::tempdir().unwrap();
    let client = Client::new(&server.source, cache.path(), false).unwrap();
    let expected = client.info("app").unwrap();
    let offline = Client::new(&server.source, cache.path(), true).unwrap();

    server.headers.lock().unwrap().insert(
        "/info/app".into(),
        format!(
            "Repr-Digest: sha-256=:{}:\r\n",
            base64::engine::general_purpose::STANDARD.encode([0; 32])
        ),
    );
    assert!(
        client
            .info("app")
            .unwrap_err()
            .to_string()
            .contains("digest mismatch")
    );
    assert_eq!(offline.info("app").unwrap(), expected);

    server.headers.lock().unwrap().remove("/info/app");
    server
        .responses
        .lock()
        .unwrap()
        .insert("/info/app".into(), b"invalid metadata".to_vec());
    assert!(client.info("app").is_err());
    assert_eq!(offline.info("app").unwrap(), expected);
}

#[test]
fn not_modified_without_cached_metadata_is_an_error() {
    let server = Server::new();
    server
        .statuses
        .lock()
        .unwrap()
        .insert("/info/app".into(), "304 Not Modified".into());
    let cache = tempfile::tempdir().unwrap();
    let client = Client::new(&server.source, cache.path(), false).unwrap();
    assert!(
        client
            .info("app")
            .unwrap_err()
            .to_string()
            .contains("304 without cached metadata")
    );
    let offline = Client::new(&server.source, cache.path(), true).unwrap();
    assert!(offline.info("app").is_err());
}

#[test]
fn bounded_reads_accept_the_limit_and_reject_an_extra_byte() {
    assert_eq!(
        crate::cache::limited_read(&b"1234"[..], 4).unwrap(),
        b"1234"
    );
    assert!(crate::cache::limited_read(&b"12345"[..], 4).is_err());
}

#[test]
fn duplicate_fields_and_invalid_dependency_names_are_rejected() {
    let checksum = "a".repeat(64);
    for body in [
        format!("---\n1.0 |checksum:{checksum},checksum:{checksum}\n"),
        format!("---\n1.0 ../other:>= 1|checksum:{checksum}\n"),
    ] {
        assert!(parse("app", &body).is_err());
    }
}

#[test]
fn gem_coop_namespace_info_without_a_separator_is_supported() {
    let body = include_str!("../../tests/fixtures/gem-coop-oaken.info");
    let records = parse("oaken", body).unwrap();
    assert_eq!(records.len(), body.lines().count());
    assert_eq!(records[0].version, "0.1.0");
    assert_eq!(records[0].metadata["licenses"], "MIT");
    assert_eq!(
        records[0].metadata["published_at"],
        "2023-07-20T16:29:33.102Z"
    );
}

#[test]
fn namespace_requests_and_offline_cache_keep_the_source_prefix() {
    let server = Server::new();
    let archive = b"namespaced archive";
    let digest = checksum(archive);
    server.responses.lock().unwrap().insert(
        "/@team/info/app".into(),
        format!("1.0 |checksum:{digest}\n").into_bytes(),
    );
    server
        .responses
        .lock()
        .unwrap()
        .insert("/@team/gems/app-1.0.gem".into(), archive.to_vec());
    let cache = tempfile::tempdir().unwrap();
    let source = format!("{}@team", server.source);
    let client = Client::new(&source, cache.path(), false).unwrap();
    let records = client.info("app").unwrap();
    let path = client.download("app", "1.0", &digest).unwrap();
    assert_eq!(fs::read(&path).unwrap(), archive);
    assert_eq!(
        server.requests.lock().unwrap().as_slice(),
        ["/@team/info/app", "/@team/gems/app-1.0.gem"]
    );
    let offline = Client::new(&source, cache.path(), true).unwrap();
    assert_eq!(offline.info("app").unwrap(), records);
    assert!(
        Client::new(&server.source, cache.path(), true)
            .unwrap()
            .info("app")
            .is_err()
    );
    assert_eq!(server.requests.lock().unwrap().len(), 2);
}

#[test]
fn empty_or_unterminated_headers_are_rejected() {
    for body in ["", "\n", "created_at: now\n"] {
        assert!(parse("app", body).is_err());
    }
}
