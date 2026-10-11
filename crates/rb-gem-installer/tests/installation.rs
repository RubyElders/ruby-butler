use flate2::{Compression, write::GzEncoder};
use std::{fs, io::Write};

fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(bytes).unwrap();
    encoder.finish().unwrap()
}

fn payload(entries: &[(&str, tar::EntryType, &[u8], u32)]) -> Vec<u8> {
    let mut archive = tar::Builder::new(Vec::new());
    for (path, kind, bytes, mode) in entries {
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(*mode);
        header.set_entry_type(*kind);
        if kind.is_symlink() || kind.is_hard_link() {
            header.set_link_name("../../outside").unwrap();
        }
        let path_bytes = path.as_bytes();
        header.as_mut_bytes()[..path_bytes.len()].copy_from_slice(path_bytes);
        header.set_cksum();
        archive.append(&header, *bytes).unwrap();
    }
    archive.into_inner().unwrap()
}

fn gem(payload: &[u8]) -> Vec<u8> {
    let mut archive = tar::Builder::new(Vec::new());
    for (name, bytes) in [
        (
            "metadata.gz",
            gzip(b"name: dinner\nversion: 1.2.3\nplatform: ruby\n"),
        ),
        ("data.tar.gz", gzip(payload)),
    ] {
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        archive
            .append_data(&mut header, name, bytes.as_slice())
            .unwrap();
    }
    archive.into_inner().unwrap()
}

#[test]
fn installs_files_directories_and_executables() {
    let root = tempfile::tempdir().unwrap();
    let destination = root.path().join("dinner");
    let archive = gem(&payload(&[
        ("lib/", tar::EntryType::Directory, b"", 0o755),
        (
            "lib/dinner.rb",
            tar::EntryType::Regular,
            b"puts :dinner\n",
            0o644,
        ),
        (
            "bin/dinner",
            tar::EntryType::Regular,
            b"#!/usr/bin/env ruby\n",
            0o6755,
        ),
    ]));
    let metadata = rb_gem_installer::install(archive.as_slice(), &destination).unwrap();
    assert_eq!(metadata.id.full_name(), "dinner-1.2.3");
    assert_eq!(
        fs::read(destination.join("lib/dinner.rb")).unwrap(),
        b"puts :dinner\n"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(destination.join("bin/dinner"))
                .unwrap()
                .permissions()
                .mode()
                & 0o7777,
            0o755
        );
    }
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
}

#[test]
fn rejects_unsafe_paths_and_links_without_partial_installation() {
    for (path, kind) in [
        ("../escape", tar::EntryType::Regular),
        ("/escape", tar::EntryType::Regular),
        ("C:\\escape", tar::EntryType::Regular),
        ("..\\escape", tar::EntryType::Regular),
        ("lib/../../escape", tar::EntryType::Regular),
        ("link", tar::EntryType::Symlink),
        ("link", tar::EntryType::Link),
        ("pipe", tar::EntryType::Fifo),
    ] {
        let root = tempfile::tempdir().unwrap();
        let archive = gem(&payload(&[
            ("first", tar::EntryType::Regular, b"ok", 0o644),
            (path, kind, b"", 0o644),
        ]));
        assert!(
            rb_gem_installer::install(archive.as_slice(), root.path().join("gem")).is_err(),
            "{path}"
        );
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0, "{path}");
    }
}

#[test]
fn rejects_duplicates_and_file_directory_collisions() {
    for paths in [
        ["file", "file"],
        ["file", "file/nested"],
        ["./file", "file"],
    ] {
        let root = tempfile::tempdir().unwrap();
        let archive = gem(&payload(&[
            (paths[0], tar::EntryType::Regular, b"x".as_slice(), 0o644),
            (paths[1], tar::EntryType::Regular, b"y".as_slice(), 0o644),
        ]));
        assert!(rb_gem_installer::install(archive.as_slice(), root.path().join("gem")).is_err());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }
}

#[test]
fn leaves_existing_installations_untouched() {
    let root = tempfile::tempdir().unwrap();
    let destination = root.path().join("gem");
    fs::create_dir(&destination).unwrap();
    fs::write(destination.join("existing"), b"original").unwrap();
    let archive = gem(&payload(&[("new", tar::EntryType::Regular, b"new", 0o644)]));
    assert!(rb_gem_installer::install(archive.as_slice(), &destination).is_err());
    assert_eq!(fs::read(destination.join("existing")).unwrap(), b"original");
    assert!(!destination.join("new").exists());
}

#[test]
fn corrupt_gzip_does_not_publish_a_directory() {
    let root = tempfile::tempdir().unwrap();
    let mut archive = gem(&payload(&[("file", tar::EntryType::Regular, b"x", 0o644)]));
    let marker = archive
        .windows(3)
        .rposition(|window| window == [0x1f, 0x8b, 8])
        .unwrap();
    archive[marker + 10] ^= 0xff;
    assert!(rb_gem_installer::install(archive.as_slice(), root.path().join("gem")).is_err());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn oversized_payload_header_is_rejected_before_writing() {
    let mut header = tar::Header::new_gnu();
    header.set_path("large").unwrap();
    header.set_size(1024 * 1024 * 1024 + 1);
    header.set_mode(0o644);
    header.set_cksum();
    let archive = gem(header.as_bytes());
    let root = tempfile::tempdir().unwrap();
    let error = rb_gem_installer::install(archive.as_slice(), root.path().join("gem")).unwrap_err();
    assert!(error.to_string().contains("size limit"), "{error:#}");
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn concurrent_installers_publish_only_one_complete_payload() {
    let root = tempfile::tempdir().unwrap();
    let destination = root.path().join("gem");
    let archive = gem(&payload(&[(
        "file",
        tar::EntryType::Regular,
        b"content",
        0o644,
    )]));
    let barrier = std::sync::Barrier::new(8);
    let successes = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..8)
            .map(|_| {
                scope.spawn(|| {
                    barrier.wait();
                    rb_gem_installer::install(archive.as_slice(), &destination).is_ok()
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| usize::from(handle.join().unwrap()))
            .sum::<usize>()
    });
    assert_eq!(successes, 1);
    assert_eq!(fs::read(destination.join("file")).unwrap(), b"content");
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
}

#[test]
fn missing_parent_and_invalid_archives_leave_no_installation() {
    let root = tempfile::tempdir().unwrap();
    let archive = gem(&payload(&[]));
    assert!(
        rb_gem_installer::install(archive.as_slice(), root.path().join("missing/gem")).is_err()
    );
    assert!(rb_gem_installer::install(b"not a gem".as_slice(), root.path().join("gem")).is_err());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn dangling_destination_symlinks_are_not_followed() {
    let root = tempfile::tempdir().unwrap();
    let destination = root.path().join("gem");
    std::os::unix::fs::symlink("missing", &destination).unwrap();
    let archive = gem(&payload(&[(
        "file",
        tar::EntryType::Regular,
        b"content",
        0o644,
    )]));
    assert!(rb_gem_installer::install(archive.as_slice(), &destination).is_err());
    assert_eq!(
        fs::read_link(destination).unwrap(),
        std::path::Path::new("missing")
    );
    assert!(!root.path().join("missing").exists());
}

#[cfg(unix)]
#[test]
fn payload_permissions_respect_the_process_umask() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let reference = root.path().join("reference");
    fs::write(&reference, b"reference").unwrap();
    let expected = fs::metadata(reference).unwrap().permissions().mode() & 0o664;
    let archive = gem(&payload(&[(
        "file",
        tar::EntryType::Regular,
        b"content",
        0o664,
    )]));
    let destination = root.path().join("gem");
    rb_gem_installer::install(archive.as_slice(), &destination).unwrap();
    assert_eq!(
        fs::metadata(destination.join("file"))
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        expected
    );
}

#[test]
fn identical_files_with_dot_prefixes_are_deduplicated() {
    let root = tempfile::tempdir().unwrap();
    let archive = gem(&payload(&[
        ("./file", tar::EntryType::Regular, b"content", 0o644),
        ("file", tar::EntryType::Regular, b"content", 0o644),
    ]));
    let destination = root.path().join("gem");
    rb_gem_installer::install(archive.as_slice(), &destination).unwrap();
    assert_eq!(fs::read(destination.join("file")).unwrap(), b"content");
}

#[cfg(unix)]
#[test]
fn relative_symlinks_can_reference_regular_payload_files() {
    let mut builder = tar::Builder::new(Vec::new());
    let mut file = tar::Header::new_gnu();
    file.set_size(7);
    file.set_mode(0o644);
    file.set_cksum();
    builder
        .append_data(&mut file, "Cargo.lock", b"content".as_slice())
        .unwrap();
    let mut link = tar::Header::new_gnu();
    link.set_size(0);
    link.set_mode(0o777);
    link.set_entry_type(tar::EntryType::Symlink);
    link.set_link_name("../../Cargo.lock").unwrap();
    link.set_cksum();
    builder
        .append_data(&mut link, "ext/native/Cargo.lock", b"".as_slice())
        .unwrap();
    let archive = gem(&builder.into_inner().unwrap());
    let root = tempfile::tempdir().unwrap();
    let destination = root.path().join("gem");
    rb_gem_installer::install(archive.as_slice(), &destination).unwrap();
    assert_eq!(
        fs::read(destination.join("ext/native/Cargo.lock")).unwrap(),
        b"content"
    );
    assert_eq!(
        fs::read_link(destination.join("ext/native/Cargo.lock")).unwrap(),
        std::path::Path::new("../../Cargo.lock")
    );
}

#[cfg(unix)]
fn symlink_archive(links: &[(&str, &str)]) -> Vec<u8> {
    let mut builder = tar::Builder::new(Vec::new());
    let mut header = tar::Header::new_gnu();
    header.set_size(7);
    header.set_mode(0o644);
    header.set_cksum();
    builder
        .append_data(&mut header, "real/file", b"content".as_slice())
        .unwrap();
    for (path, target) in links {
        let mut header = tar::Header::new_gnu();
        header.set_size(0);
        header.set_mode(0o777);
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_link_name(target).unwrap();
        header.set_cksum();
        builder
            .append_data(&mut header, path, b"".as_slice())
            .unwrap();
    }
    gem(&builder.into_inner().unwrap())
}

#[cfg(unix)]
#[test]
fn directory_symlinks_and_forward_chains_remain_inside_the_payload() {
    let archive = symlink_archive(&[
        ("directory", "real"),
        ("file", "next"),
        ("next", "directory/file"),
    ]);
    let root = tempfile::tempdir().unwrap();
    let destination = root.path().join("gem");
    rb_gem_installer::install(archive.as_slice(), &destination).unwrap();
    assert_eq!(fs::read(destination.join("file")).unwrap(), b"content");
    assert_eq!(
        fs::read(destination.join("directory/file")).unwrap(),
        b"content"
    );
    assert_eq!(
        fs::read_link(destination.join("file")).unwrap(),
        std::path::Path::new("next")
    );
}

#[cfg(unix)]
#[test]
fn cycles_dangling_links_and_link_parents_roll_back() {
    for links in [
        vec![("a", "b"), ("b", "a")],
        vec![("a", "missing")],
        vec![("a", "real"), ("a/b", "file")],
        vec![("a", "/outside")],
        vec![("a", "../outside")],
    ] {
        let root = tempfile::tempdir().unwrap();
        let archive = symlink_archive(&links);
        assert!(rb_gem_installer::install(archive.as_slice(), root.path().join("gem")).is_err());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }
}
