#![cfg(unix)]
use rb_ruby_installer::{Event, Installer, Phase, Platform};
use rb_tests::HttpServer as Server;
use sha2::{Digest, Sha256};
use std::{fs, path::Path, sync::Arc};
pub mod support;
use support::{archive, configure, install, installer};

#[test]
fn installs_atomically_reuses_install_and_preserves_permissions() {
    let root = tempfile::tempdir().unwrap();
    let server = Server::new();
    let bytes = archive(Some(("bin/alias", "ruby")), "3.4.10");
    configure(
        &server,
        bytes.clone(),
        Some(format!("sha256:{:x}", Sha256::digest(&bytes))),
    );
    let installer = installer(root.path(), &server);
    let destination = root.path().join("rubies with spaces");
    let mut phases = Vec::new();
    let mut selected = Vec::new();
    let result = installer
        .install(
            &"3.4".parse().unwrap(),
            Platform::LinuxX86_64,
            &destination,
            |event| match event {
                Event::Phase(phase) => phases.push(phase),
                Event::Selected { version } => selected.push(version.to_string()),
                Event::Download { .. } => {}
            },
        )
        .unwrap();
    assert_eq!(selected, ["3.4.10"]);
    assert!(result.installed);
    assert_eq!(result.version.to_string(), "3.4.10");
    assert!(result.path.join(".rb-install.json").is_file());
    assert_eq!(
        fs::read_link(result.path.join("bin/alias")).unwrap(),
        Path::new("ruby")
    );
    assert_eq!(
        phases,
        [
            Phase::Selecting,
            Phase::Downloading,
            Phase::Verifying,
            Phase::Extracting,
            Phase::Validating
        ]
    );
    assert!(!install(&installer, &destination).unwrap().installed);
    assert_eq!(server.requests().len(), 2);
    assert!(!fs::read_dir(&destination).unwrap().flatten().any(|entry| {
        entry
            .file_name()
            .to_string_lossy()
            .starts_with(".ruby-install-")
    }));
}

#[test]
fn corrupt_download_never_becomes_an_installation_or_cached_archive() {
    let root = tempfile::tempdir().unwrap();
    let server = Server::new();
    configure(
        &server,
        archive(None, "3.4.10"),
        Some(format!("sha256:{}", "0".repeat(64))),
    );
    let cache = root.path().join("cache");
    let installer = Installer::new(&cache)
        .unwrap()
        .with_catalog_url(format!("{}/catalog", server.url));
    assert!(
        install(&installer, &root.path().join("rubies"))
            .unwrap_err()
            .to_string()
            .contains("SHA-256 mismatch")
    );
    assert!(!root.path().join("rubies/ruby-3.4.10").exists());
    assert_eq!(fs::read_dir(cache.join("archives")).unwrap().count(), 0);
}

#[test]
fn checksum_file_fallback_and_corrupt_cache_recovery() {
    let root = tempfile::tempdir().unwrap();
    let server = Server::new();
    let bytes = archive(None, "3.4.10");
    let digest = format!("{:x}", Sha256::digest(&bytes));
    configure(&server, bytes, None);
    server.route(
        "/sums",
        200,
        format!("{digest}  ruby-3.4.10.x86_64_linux.tar.gz\n"),
    );
    let cache = root.path().join("cache");
    fs::create_dir_all(cache.join("archives")).unwrap();
    fs::write(cache.join(format!("archives/{digest}.tar.gz")), "broken").unwrap();
    let installer = Installer::new(&cache)
        .unwrap()
        .with_catalog_url(format!("{}/catalog", server.url));
    assert!(
        install(&installer, &root.path().join("first"))
            .unwrap()
            .installed
    );
    server.route("/ruby.tar.gz", 500, vec![]);
    assert!(
        install(&installer, &root.path().join("second"))
            .unwrap()
            .installed
    );
    assert_eq!(
        server
            .requests()
            .iter()
            .filter(|request| request.starts_with("GET /ruby.tar.gz "))
            .count(),
        1
    );
}

#[test]
fn rejects_escaping_links_and_wrong_runtime_version_and_cleans_staging() {
    for (link, version, expected) in [
        (
            Some(("escape", "../../../outside")),
            "3.4.10",
            "Ruby archive link",
        ),
        (None, "3.3.0", "does not match"),
    ] {
        let root = tempfile::tempdir().unwrap();
        let server = Server::new();
        let bytes = archive(link, version);
        configure(
            &server,
            bytes.clone(),
            Some(format!("sha256:{:x}", Sha256::digest(&bytes))),
        );
        let installer = installer(root.path(), &server);
        let destination = root.path().join("rubies");
        assert!(
            install(&installer, &destination)
                .unwrap_err()
                .to_string()
                .contains(expected)
        );
        assert!(!destination.join("ruby-3.4.10").exists());
        assert_eq!(fs::read_dir(destination).unwrap().count(), 1);
    }
}

#[test]
fn simultaneous_installs_publish_once() {
    let root = tempfile::tempdir().unwrap();
    let server = Server::new();
    let bytes = archive(None, "3.4.10");
    configure(
        &server,
        bytes.clone(),
        Some(format!("sha256:{:x}", Sha256::digest(&bytes))),
    );
    let installer = Arc::new(installer(root.path(), &server));
    let destination = root.path().join("rubies");
    let results = std::thread::scope(|scope| {
        let first = scope.spawn(|| install(&installer, &destination).unwrap());
        let second = scope.spawn(|| install(&installer, &destination).unwrap());
        [first.join().unwrap(), second.join().unwrap()]
    });
    assert_eq!(results.iter().filter(|result| result.installed).count(), 1);
    assert_eq!(
        server
            .requests()
            .iter()
            .filter(|request| request.starts_with("GET /ruby.tar.gz "))
            .count(),
        1
    );
}

#[test]
fn invalid_archives_missing_checksums_and_download_errors_preserve_destination() {
    for failure in ["archive", "checksum", "http", "size"] {
        let root = tempfile::tempdir().unwrap();
        let server = Server::new();
        let bytes = if failure == "archive" {
            b"not a gzip archive".to_vec()
        } else {
            archive(None, "3.4.10")
        };
        let digest =
            (failure != "checksum").then(|| format!("sha256:{:x}", Sha256::digest(&bytes)));
        configure(&server, bytes, digest);
        if failure == "http" {
            server.route("/ruby.tar.gz", 503, vec![]);
        }
        if failure == "checksum" {
            server.route("/sums", 200, "unrelated checksum\n");
        }
        if failure == "size" {
            server.route("/ruby.tar.gz", 200, "short");
        }
        let installer = installer(root.path(), &server);
        let directory = root.path().join("rubies");
        fs::create_dir_all(directory.join("ruby-3.3.0")).unwrap();
        fs::write(directory.join("ruby-3.3.0/keep"), "untouched").unwrap();
        assert!(install(&installer, &directory).is_err(), "{failure}");
        assert!(!directory.join("ruby-3.4.10").exists());
        assert_eq!(
            fs::read_to_string(directory.join("ruby-3.3.0/keep")).unwrap(),
            "untouched"
        );
        assert!(!fs::read_dir(directory).unwrap().flatten().any(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".ruby-install-")
        }));
    }
}

#[test]
fn complete_third_party_install_needs_neither_catalog_nor_receipt() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("rubies");
    fs::create_dir_all(directory.join("ruby-3.4.10/bin")).unwrap();
    fs::create_dir_all(directory.join("ruby-3.4.10/lib/ruby")).unwrap();
    fs::write(directory.join("ruby-3.4.10/bin/ruby"), "existing").unwrap();
    let installer = Installer::new(root.path().join("cache"))
        .unwrap()
        .with_catalog_url("https://example.invalid/no-network");
    let result = install(&installer, &directory).unwrap();
    assert!(!result.installed);
    assert!(!root.path().join("cache").exists());
    assert_eq!(
        fs::read_to_string(result.path.join("bin/ruby")).unwrap(),
        "existing"
    );
}

#[test]
fn interrupted_downloads_are_removed_and_can_be_retried() {
    let root = tempfile::tempdir().unwrap();
    let server = Server::new();
    let bytes = archive(None, "3.4.10");
    configure(
        &server,
        bytes.clone(),
        Some(format!("sha256:{:x}", Sha256::digest(&bytes))),
    );
    server.truncated_route(
        "/ruby.tar.gz",
        bytes.len(),
        bytes[..bytes.len() / 2].to_vec(),
    );
    let installer = installer(root.path(), &server);
    let destination = root.path().join("rubies");
    assert!(install(&installer, &destination).is_err());
    assert!(!destination.join("ruby-3.4.10").exists());
    assert_eq!(
        fs::read_dir(root.path().join("cache/archives"))
            .unwrap()
            .count(),
        0
    );
    server.route("/ruby.tar.gz", 200, bytes);
    assert!(install(&installer, &destination).unwrap().installed);
    assert_eq!(
        server
            .requests()
            .iter()
            .filter(|request| request.starts_with("GET /ruby.tar.gz "))
            .count(),
        2
    );
}

#[test]
fn incomplete_installations_are_preserved_with_an_actionable_error() {
    let root = tempfile::tempdir().unwrap();
    let server = Server::new();
    configure(&server, archive(None, "3.4.10"), None);
    let destination = root.path().join("rubies");
    fs::create_dir_all(destination.join("ruby-3.4.10")).unwrap();
    fs::write(destination.join("ruby-3.4.10/keep"), "user content").unwrap();
    let error = install(&installer(root.path(), &server), &destination)
        .unwrap_err()
        .to_string();
    assert!(error.contains("move it aside before retrying"), "{error}");
    assert_eq!(
        fs::read_to_string(destination.join("ruby-3.4.10/keep")).unwrap(),
        "user content"
    );
    assert_eq!(server.requests().len(), 1);
    assert!(!root.path().join("cache/archives").exists());
}

#[test]
fn invalid_sizes_and_checksums_fail_before_fetching_the_archive() {
    for (size, digest, expected) in [
        (
            0,
            format!("sha256:{}", "0".repeat(64)),
            "Invalid Ruby archive size",
        ),
        (
            512 * 1024 * 1024 + 1,
            format!("sha256:{}", "0".repeat(64)),
            "Invalid Ruby archive size",
        ),
        (
            100,
            "sha256:../../outside".into(),
            "Invalid SHA-256 checksum",
        ),
        (
            100,
            format!("sha512:{}", "0".repeat(128)),
            "Unsupported Ruby archive checksum algorithm",
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        let server = Server::new();
        server.route(
            "/catalog",
            200,
            serde_json::to_vec(&serde_json::json!({"tag_name":"fixture","assets":[{
                "name":"ruby-3.4.10.x86_64_linux.tar.gz","size":size,"digest":digest,
                "browser_download_url":format!("{}/ruby.tar.gz",server.url)
            }]}))
            .unwrap(),
        );
        let error = install(
            &installer(root.path(), &server),
            &root.path().join("rubies"),
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains(expected), "{error}");
        assert_eq!(server.requests().len(), 1);
        assert!(!root.path().join("cache/archives").exists());
    }
}
