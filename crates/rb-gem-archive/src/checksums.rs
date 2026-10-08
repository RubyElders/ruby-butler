use crate::{
    constants::{DATA_MEMBER, METADATA_MEMBER},
    metadata,
};
use anyhow::{Context, Result, ensure};
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha512};
use std::collections::BTreeMap;

#[derive(Default)]
pub(crate) struct Digests {
    sha1: Sha1,
    sha256: Sha256,
    sha512: Sha512,
}

impl Digests {
    pub(crate) fn update(&mut self, bytes: &[u8]) {
        self.sha1.update(bytes);
        self.sha256.update(bytes);
        self.sha512.update(bytes);
    }

    pub(crate) fn finish(self) -> BTreeMap<&'static str, String> {
        BTreeMap::from([
            ("SHA1", format!("{:x}", self.sha1.finalize())),
            ("SHA256", format!("{:x}", self.sha256.finalize())),
            ("SHA512", format!("{:x}", self.sha512.finalize())),
        ])
    }
}

pub(crate) fn verify(
    bytes: &[u8],
    hashes: &BTreeMap<String, BTreeMap<&str, String>>,
) -> Result<()> {
    let document: serde_yaml_ng::Value =
        serde_yaml_ng::from_slice(bytes).context("Reading gem checksums")?;
    let algorithms = metadata::untag(&document)
        .as_mapping()
        .context("Invalid gem checksum map")?;
    ensure!(!algorithms.is_empty(), "Empty gem checksum map");
    for (algorithm, values) in algorithms {
        let algorithm = metadata::untag(algorithm)
            .as_str()
            .context("Invalid checksum algorithm")?;
        ensure!(
            matches!(algorithm, "SHA1" | "SHA256" | "SHA512"),
            "Unsupported checksum algorithm: {algorithm}"
        );
        let values = metadata::untag(values)
            .as_mapping()
            .context("Invalid member checksum map")?;
        for name in [METADATA_MEMBER, DATA_MEMBER] {
            ensure!(
                values.contains_key(serde_yaml_ng::Value::String(name.into())),
                "Missing member checksum"
            );
        }
        for (name, expected) in values {
            let name = metadata::untag(name)
                .as_str()
                .context("Invalid checksum member")?;
            let expected = metadata::untag(expected)
                .as_str()
                .context("Invalid member checksum")?;
            let actual = hashes
                .get(name)
                .context("Checksum references missing member")?
                .get(algorithm)
                .unwrap();
            ensure!(
                expected.len() == actual.len() && expected.bytes().all(|c| c.is_ascii_hexdigit()),
                "Invalid {algorithm} member checksum"
            );
            ensure!(
                expected.eq_ignore_ascii_case(actual),
                "Gem member checksum mismatch: {name} ({algorithm})"
            );
        }
    }
    Ok(())
}
