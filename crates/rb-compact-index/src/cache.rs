use crate::constants::{ARCHIVE_BUFFER_BYTES, MAX_ARCHIVE_BYTES};
use anyhow::{Context, Result, ensure};
use fs2::FileExt;
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

pub(crate) fn stream_archive(mut input: impl Read, mut output: impl Write) -> Result<String> {
    let mut digest = Sha256::new();
    let mut buffer = [0; ARCHIVE_BUFFER_BYTES];
    let mut size = 0_u64;
    loop {
        let count = input.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        size += count as u64;
        ensure!(
            size <= MAX_ARCHIVE_BYTES,
            "Gem archive exceeds 1 GiB size limit"
        );
        digest.update(&buffer[..count]);
        output.write_all(&buffer[..count])?;
    }
    Ok(format!("{:x}", digest.finalize()))
}

pub(crate) fn limited_read(reader: impl Read, limit: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(limit + 1).read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= limit,
        "Download or archive member exceeds size limit"
    );
    Ok(bytes)
}

pub(crate) fn checksum(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().context("Path has no parent")?;
    fs::create_dir_all(parent)?;
    if fs::read(path).is_ok_and(|old| old == bytes) {
        return Ok(());
    }
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    file.persist(path)?;
    Ok(())
}

pub(crate) fn lock(path: &Path) -> Result<File> {
    fs::create_dir_all(path.parent().context("Lock path has no parent")?)?;
    let file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(path)?;
    file.lock_exclusive()?;
    Ok(file)
}
