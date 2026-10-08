use crate::{
    GemMetadata,
    checksums::{self, Digests},
    constants::{
        CHECKSUM_MEMBER, DATA_MEMBER, MAX_ARCHIVE_BYTES, MAX_CHECKSUM_BYTES, MAX_METADATA_BYTES,
        MAX_SIGNATURE_BYTES, METADATA_MEMBER, READ_BUFFER_BYTES,
    },
    metadata,
};
use anyhow::{Context, Result, ensure};
use flate2::read::GzDecoder;
use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
};

fn limited_read(reader: impl Read, limit: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(limit + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() as u64 <= limit, "Gem member exceeds size limit");
    Ok(bytes)
}

/// Reads metadata and verifies supported member checksums without unpacking payload files.
pub fn read(reader: impl Read) -> Result<GemMetadata> {
    let mut archive = tar::Archive::new(reader.take(MAX_ARCHIVE_BYTES + 1));
    let mut found = BTreeSet::new();
    let mut hashes = BTreeMap::new();
    let mut gem_metadata = None;
    let mut checksum_bytes = None;
    let mut total = 0_u64;
    for entry in archive.entries()? {
        let mut entry = entry?;
        let name = entry
            .path()?
            .to_str()
            .context("Non-UTF8 gem member")?
            .to_string();
        let signed_member = name.strip_suffix(".sig");
        let member = signed_member.unwrap_or(&name);
        ensure!(
            matches!(member, METADATA_MEMBER | DATA_MEMBER | CHECKSUM_MEMBER),
            "Unsupported gem member: {name}"
        );
        ensure!(
            entry.header().entry_type().is_file(),
            "Gem member must be a regular file"
        );
        ensure!(found.insert(name.clone()), "Duplicate gem member: {name}");
        total = total
            .checked_add(entry.size())
            .context("Gem archive size overflow")?;
        ensure!(total <= MAX_ARCHIVE_BYTES, "Gem archive exceeds size limit");
        if signed_member.is_some() {
            limited_read(entry, MAX_SIGNATURE_BYTES)?;
            continue;
        }
        if name == DATA_MEMBER {
            let mut hash = Digests::default();
            let mut buffer = [0; READ_BUFFER_BYTES];
            loop {
                let read = entry.read(&mut buffer)?;
                if read == 0 {
                    break;
                }
                hash.update(&buffer[..read]);
            }
            hashes.insert(name, hash.finish());
        } else {
            let limit = if name == METADATA_MEMBER {
                MAX_METADATA_BYTES
            } else {
                MAX_CHECKSUM_BYTES
            };
            let compressed = limited_read(entry, limit)?;
            let mut hash = Digests::default();
            hash.update(&compressed);
            hashes.insert(name.clone(), hash.finish());
            let decoded = limited_read(GzDecoder::new(compressed.as_slice()), limit)?;
            if name == METADATA_MEMBER {
                gem_metadata = Some(decoded);
            } else {
                checksum_bytes = Some(decoded);
            }
        }
    }
    ensure!(found.contains(DATA_MEMBER), "Gem is missing data.tar.gz");
    let gem_metadata = gem_metadata.context("Gem is missing metadata.gz")?;
    if let Some(bytes) = checksum_bytes {
        checksums::verify(&bytes, &hashes)?;
    }

    metadata::parse(&gem_metadata)
}
