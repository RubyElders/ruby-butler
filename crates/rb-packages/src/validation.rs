use crate::PackageRecord;
use anyhow::{Result, ensure};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::Path};

pub(crate) fn verify(path: &Path, checksum: &str) -> Result<()> {
    let mut input = fs::File::open(path)?;
    let mut hash = Sha256::new();
    let mut buffer = [0; crate::constants::READ_BUFFER_BYTES];
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    ensure!(
        format!("{:x}", hash.finalize()) == checksum,
        "Archive checksum mismatch"
    );
    Ok(())
}

pub(crate) fn ensure_record(
    metadata: &rb_gem_archive::GemMetadata,
    record: &PackageRecord,
) -> Result<()> {
    ensure!(
        metadata.id == record.package.id,
        "Archive identity differs from resolution"
    );
    ensure!(
        metadata.dependencies.len() == record.package.dependencies.len()
            && metadata
                .dependencies
                .iter()
                .all(|(name, requirement)| record
                    .package
                    .dependencies
                    .get(name)
                    .is_some_and(|expected| same_requirement(requirement, expected)))
            && same_requirement(&metadata.ruby, &record.package.ruby),
        "Archive metadata differs from resolution for {}: dependencies {:?} / {:?}, Ruby {} / {}",
        record.package.id,
        metadata.dependencies,
        record.package.dependencies,
        metadata.ruby,
        record.package.ruby
    );
    Ok(())
}

pub(crate) fn same_requirement(
    left: &rb_gem_types::Requirement,
    right: &rb_gem_types::Requirement,
) -> bool {
    left.constraints()
        .all(|constraint| right.constraints().any(|other| constraint == other))
        && right
            .constraints()
            .all(|constraint| left.constraints().any(|other| constraint == other))
}
