#![cfg(unix)]
pub mod support;
use rb_tests::{HttpServer, RubyArchive};
use sha2::{Digest, Sha256};
use std::{fs, io::Write, os::unix::fs::MetadataExt};
use support::{archive, configure, install, installer};

fn raw_archive(path: &str, size: u64, kind: tar::EntryType) -> Vec<u8> {
    let mut header = tar::Header::new_gnu();
    header.set_size(size);
    header.set_mode(0o644);
    header.set_entry_type(kind);
    header.as_mut_bytes()[..path.len()].copy_from_slice(path.as_bytes());
    header.set_cksum();
    let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gzip.write_all(header.as_bytes()).unwrap();
    gzip.write_all(&[0; 1024]).unwrap();
    gzip.finish().unwrap()
}

#[test]
fn rejects_traversal_special_entries_and_oversized_payloads() {
    for (bytes, expected) in [
        (
            raw_archive("/outside", 0, tar::EntryType::Regular),
            "Unsafe Ruby archive path",
        ),
        (
            raw_archive("ruby/version/../../outside", 0, tar::EntryType::Regular),
            "Unsafe Ruby archive path",
        ),
        (
            raw_archive("ruby/version/pipe", 0, tar::EntryType::Fifo),
            "Unsupported Ruby archive entry",
        ),
        (
            raw_archive(
                "ruby/version/huge",
                2 * 1024 * 1024 * 1024 + 1,
                tar::EntryType::Regular,
            ),
            "unpacked size limit",
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        let server = HttpServer::new();
        configure(
            &server,
            bytes.clone(),
            Some(format!("sha256:{:x}", Sha256::digest(&bytes))),
        );
        let destination = root.path().join("rubies");
        let error = install(&installer(root.path(), &server), &destination)
            .unwrap_err()
            .to_string();
        assert!(error.contains(expected), "{error}");
        assert!(!destination.join("ruby-3.4.10").exists());
        assert!(!root.path().join("outside").exists());
        assert_eq!(fs::read_dir(destination).unwrap().count(), 1);
    }
}

#[test]
fn hardlinks_stay_inside_the_installation_and_preserve_file_identity() {
    for escape in [false, true] {
        let root = tempfile::tempdir().unwrap();
        let server = HttpServer::new();
        let mut builder = RubyArchive::new("3.4.10");
        builder
            .file("bin/ruby", "#!/bin/sh\nprintf '3.4.10'\n", 0o755)
            .unwrap();
        builder.file("lib/ruby/fixture.rb", [], 0o644).unwrap();
        let target = if escape {
            "../../outside"
        } else {
            "rv-ruby@3.4.10/3.4.10/bin/ruby"
        };
        builder.hardlink("bin/alias", target).unwrap();
        let bytes = builder.finish().unwrap();
        configure(
            &server,
            bytes.clone(),
            Some(format!("sha256:{:x}", Sha256::digest(&bytes))),
        );
        let destination = root.path().join("rubies");
        let result = install(&installer(root.path(), &server), &destination);
        if escape {
            assert!(result.unwrap_err().to_string().contains("link"));
            assert!(!destination.join("ruby-3.4.10").exists());
        } else {
            let result = result.unwrap();
            assert_eq!(
                fs::metadata(result.path.join("bin/ruby")).unwrap().ino(),
                fs::metadata(result.path.join("bin/alias")).unwrap().ino()
            );
        }
    }
}

#[test]
fn verified_bytes_are_not_enough_when_the_archive_layout_is_invalid() {
    let good = archive(None, "3.4.10");
    let mut mixed = tar::Builder::new(Vec::new());
    for path in ["first/version/one", "second/version/two"] {
        let mut header = tar::Header::new_gnu();
        header.set_size(0);
        header.set_mode(0o644);
        mixed.append_data(&mut header, path, &[][..]).unwrap();
    }
    let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    gzip.write_all(&mixed.into_inner().unwrap()).unwrap();
    for (bytes, expected) in [
        (gzip.finish().unwrap(), Some("inconsistent roots")),
        (good, None),
    ] {
        let root = tempfile::tempdir().unwrap();
        let server = HttpServer::new();
        configure(
            &server,
            bytes.clone(),
            Some(format!("sha256:{:x}", Sha256::digest(&bytes))),
        );
        let result = install(
            &installer(root.path(), &server),
            &root.path().join("rubies"),
        );
        if let Some(expected) = expected {
            assert!(result.unwrap_err().to_string().contains(expected));
        } else {
            assert!(result.unwrap().path.join("bin/ruby").is_file());
        }
    }
}
