mod common;
use rb_gem_archive::read;
use sha2::{Digest, Sha512};

#[test]
fn sha512_only_archives_verify_every_member_without_sha256() {
    let mut members = common::members(include_bytes!("fixtures/metadata.yml"));
    members[2].1 = common::gzip(
        format!(
            "SHA512:\n  metadata.gz: {:x}\n  data.tar.gz: {:x}\n",
            Sha512::digest(&members[0].1),
            Sha512::digest(&members[1].1)
        )
        .as_bytes(),
    );
    assert_eq!(
        read(common::container(&members).as_slice())
            .unwrap()
            .id
            .name,
        "dinner"
    );
    members[1].1.push(0);
    assert!(read(common::container(&members).as_slice()).is_err());
}

#[test]
fn signed_archives_are_read_without_asserting_signature_trust() {
    let mut members = common::members(include_bytes!("fixtures/metadata.yml"));
    for name in [
        "metadata.gz.sig",
        "data.tar.gz.sig",
        "checksums.yaml.gz.sig",
    ] {
        members.push((name, b"signature bytes".to_vec()));
    }
    assert_eq!(
        read(common::container(&members).as_slice())
            .unwrap()
            .id
            .name,
        "dinner"
    );
    members.push(("metadata.gz.sig", b"duplicate".to_vec()));
    assert!(read(common::container(&members).as_slice()).is_err());
}

#[test]
fn sha1_only_archives_are_checked_and_bad_sha512_cannot_hide_behind_valid_sha256() {
    let mut members = common::members(include_bytes!("fixtures/metadata.yml"));
    let sha1 = format!(
        "SHA1:\n  metadata.gz: {:x}\n  data.tar.gz: {:x}\n",
        sha1::Sha1::digest(&members[0].1),
        sha1::Sha1::digest(&members[1].1)
    );
    members[2].1 = common::gzip(sha1.as_bytes());
    assert_eq!(
        read(common::container(&members).as_slice())
            .unwrap()
            .id
            .name,
        "dinner"
    );
    members[1].1.push(0);
    assert!(read(common::container(&members).as_slice()).is_err());

    let mut members = common::members(include_bytes!("fixtures/metadata.yml"));
    let sha256 = format!(
        "SHA256:\n  metadata.gz: {:x}\n  data.tar.gz: {:x}\n",
        sha2::Sha256::digest(&members[0].1),
        sha2::Sha256::digest(&members[1].1)
    );
    let mixed = format!(
        "{sha256}SHA512:\n  metadata.gz: {}\n  data.tar.gz: {}\n",
        "0".repeat(128),
        "0".repeat(128)
    );
    members[2].1 = common::gzip(mixed.as_bytes());
    assert!(
        read(common::container(&members).as_slice())
            .unwrap_err()
            .to_string()
            .contains("checksum mismatch")
    );
}

#[test]
fn signature_members_must_stay_bounded_and_belong_to_known_members() {
    let mut members = common::members(include_bytes!("fixtures/metadata.yml"));
    members.push(("metadata.gz.sig", vec![0; 2 * 1024 * 1024]));
    let error = read(common::container(&members).as_slice()).unwrap_err();
    assert!(error.to_string().contains("size limit"));
    members.pop();
    members.push(("other.sig", b"signature bytes".to_vec()));
    assert!(read(common::container(&members).as_slice()).is_err());
}
