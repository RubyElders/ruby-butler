use crate::{
    constants::{DATA_MEMBER, METADATA_MEMBER},
    metadata,
};
use anyhow::{Context, Result, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha512};
use std::collections::{BTreeMap, BTreeSet};

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
    let mut seen_algorithms = BTreeSet::new();
    for (algorithm, values) in algorithms {
        let algorithm = text(algorithm)?;
        ensure!(
            seen_algorithms.insert(algorithm.clone()),
            "Duplicate checksum algorithm"
        );
        ensure!(
            matches!(algorithm.as_str(), "SHA1" | "SHA256" | "SHA512"),
            "Unsupported checksum algorithm: {algorithm}"
        );
        let values = metadata::untag(values)
            .as_mapping()
            .context("Invalid member checksum map")?;
        let mut names = BTreeSet::new();
        for (name, expected) in values {
            let name = text(name)?;
            ensure!(names.insert(name.clone()), "Duplicate checksum member");
            let expected = text(expected)?;
            let actual = hashes
                .get(&name)
                .context("Checksum references missing member")?
                .get(algorithm.as_str())
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
        ensure!(
            names.contains(METADATA_MEMBER) && names.contains(DATA_MEMBER),
            "Missing member checksum"
        );
    }
    Ok(())
}

fn text(value: &serde_yaml_ng::Value) -> Result<String> {
    let content = metadata::untag(value)
        .as_str()
        .context("Invalid checksum text")?;
    if let serde_yaml_ng::Value::Tagged(tagged) = value
        && (tagged.tag == "binary" || tagged.tag == "tag:yaml.org,2002:binary")
    {
        let encoded: String = content
            .chars()
            .filter(|c| !c.is_ascii_whitespace())
            .collect();
        let bytes = STANDARD
            .decode(encoded)
            .context("Invalid binary checksum encoding")?;
        return String::from_utf8(bytes).context("Non-UTF8 checksum text");
    }
    Ok(content.to_owned())
}
