use crate::PackageRecord;
use anyhow::{Result, bail, ensure};
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
    ruby: &rb_gem_types::Version,
) -> Result<()> {
    ensure!(
        metadata.id == record.package.id,
        "Archive identity differs from resolution"
    );
    for (name, expected) in &record.package.dependencies {
        let Some(actual) = metadata.dependencies.get(name) else {
            bail!(
                "Archive dependency differs for {}: missing {name} (resolution declares {expected})",
                record.package.id
            );
        };
        ensure!(
            same_requirement(actual, expected),
            "Archive dependency differs for {}: {name} requires {actual}, resolution declares {expected}",
            record.package.id
        );
    }
    for (name, requirement) in &metadata.dependencies {
        ensure!(
            record.package.dependencies.contains_key(name),
            "Archive dependency differs for {}: unexpected {name} {requirement}",
            record.package.id
        );
    }
    if let Some(expected) = &record.declared_ruby {
        ensure!(
            same_requirement(&metadata.ruby, expected),
            "Archive Ruby requirement differs for {}: archive requires {}, resolution declares {expected}",
            record.package.id,
            metadata.ruby
        );
    }
    ensure!(
        metadata.ruby.matches(ruby),
        "{} requires Ruby {}, selected Ruby is {ruby}",
        record.package.id,
        metadata.ruby
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
