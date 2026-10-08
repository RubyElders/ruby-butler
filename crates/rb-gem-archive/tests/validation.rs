mod common;
use rb_gem_archive::read;

#[test]
fn old_archives_without_checksums_remain_readable() {
    let mut members = common::members(include_bytes!("fixtures/metadata.yml"));
    members.pop();
    assert_eq!(
        read(common::container(&members).as_slice())
            .unwrap()
            .id
            .name,
        "dinner"
    );
}

#[test]
fn changed_metadata_or_payload_fails_member_checksum_verification() {
    for index in [0, 1] {
        let mut members = common::members(include_bytes!("fixtures/metadata.yml"));
        members[index].1.push(0);
        let error = read(common::container(&members).as_slice()).unwrap_err();
        assert!(error.to_string().contains("checksum mismatch"), "{error}");
    }
}

#[test]
fn missing_duplicate_and_unknown_members_are_rejected() {
    for missing in [0, 1] {
        let mut members = common::members(include_bytes!("fixtures/metadata.yml"));
        members.remove(missing);
        assert!(read(common::container(&members).as_slice()).is_err());
    }
    let mut members = common::members(include_bytes!("fixtures/metadata.yml"));
    members.push(members[0].clone());
    assert!(
        read(common::container(&members).as_slice())
            .unwrap_err()
            .to_string()
            .contains("Duplicate")
    );
    members.pop();
    members.push(("unexpected", Vec::new()));
    assert!(read(common::container(&members).as_slice()).is_err());
}

#[test]
fn invalid_gzip_yaml_requirements_and_platforms_are_rejected() {
    for text in [
        "not a mapping",
        "name: '../bad'\nversion: '1'",
        "name: valid\nversion: nope",
        "name: valid\nversion: '1'\nplatform: []",
        "name: valid\nversion: '1'\nrequired_ruby_version: '? 2'",
        "name: valid\nversion: '1'\ndependencies: wrong",
        "name: valid\nversion: '1'\nmetadata: {wrong: []}",
    ] {
        assert!(
            read(common::container(&common::members(text.as_bytes())).as_slice()).is_err(),
            "{text}"
        );
    }
    let bytes = common::container(&[
        ("metadata.gz", vec![1, 2, 3]),
        ("data.tar.gz", common::gzip(b"")),
    ]);
    assert!(read(bytes.as_slice()).is_err());
}

#[test]
fn decompressed_metadata_is_bounded() {
    let bytes = vec![b'a'; 16 * 1024 * 1024 + 1];
    let members = common::members(&bytes);
    assert!(
        read(common::container(&members).as_slice())
            .unwrap_err()
            .to_string()
            .contains("size limit")
    );
}

#[test]
fn oversized_declared_members_are_rejected_before_reading_payload() {
    let mut header = tar::Header::new_gnu();
    header.set_path("data.tar.gz").unwrap();
    header.set_size(1 << 40);
    header.set_mode(0o644);
    header.set_cksum();
    let error = read(header.as_bytes().as_slice()).unwrap_err();
    assert!(error.to_string().contains("size limit"));
}

#[test]
fn outer_members_cannot_be_links() {
    let mut header = tar::Header::new_gnu();
    header.set_path("metadata.gz").unwrap();
    header.set_entry_type(tar::EntryType::Symlink);
    header.set_link_name("other.gz").unwrap();
    header.set_size(0);
    header.set_mode(0o644);
    header.set_cksum();
    assert!(
        read(header.as_bytes().as_slice())
            .unwrap_err()
            .to_string()
            .contains("regular file")
    );
}

#[test]
fn invalid_checksum_maps_are_rejected() {
    for checksum in [
        "SHA256: wrong",
        "SHA512: {}",
        "SHA256: {}",
        "SHA256: {metadata.gz: xyz, data.tar.gz: xyz}",
    ] {
        let mut members = common::members(include_bytes!("fixtures/metadata.yml"));
        members[2].1 = common::gzip(checksum.as_bytes());
        assert!(
            read(common::container(&members).as_slice()).is_err(),
            "{checksum}"
        );
    }
}
