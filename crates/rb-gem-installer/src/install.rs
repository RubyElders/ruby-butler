use crate::{constants::MAX_ARCHIVE_BYTES, extract};
use anyhow::{Context, Result, ensure};
use rb_gem_archive::GemMetadata;
use std::{
    fs,
    io::{self, Read, Seek, SeekFrom},
    path::Path,
};

/// Verifies an archive and installs its payload into a new directory.
pub fn install(reader: impl Read, destination: impl AsRef<Path>) -> Result<GemMetadata> {
    staged(reader, destination.as_ref(), |_, _| Ok(())).map(|(metadata, ())| metadata)
}

pub fn install_with_builder(
    reader: impl Read,
    destination: impl AsRef<Path>,
    builder: &crate::NativeBuilder,
    events: impl FnMut(crate::BuildEvent),
) -> Result<(GemMetadata, crate::BuildReport)> {
    staged(reader, destination.as_ref(), |metadata, root| {
        builder.build(metadata, root, events)
    })
}

fn staged<T>(
    reader: impl Read,
    destination: &Path,
    work: impl FnOnce(&GemMetadata, &Path) -> Result<T>,
) -> Result<(GemMetadata, T)> {
    ensure!(
        destination.file_name().is_some(),
        "Installation needs a directory name"
    );
    let parent = destination
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    ensure!(parent.is_dir(), "Installation parent must exist");
    ensure!(
        fs::symlink_metadata(destination)
            .is_err_and(|error| error.kind() == io::ErrorKind::NotFound),
        "Installation destination already exists or is inaccessible"
    );
    let mut snapshot = tempfile::tempfile()?;
    let copied = io::copy(&mut reader.take(MAX_ARCHIVE_BYTES + 1), &mut snapshot)?;
    ensure!(
        copied <= MAX_ARCHIVE_BYTES,
        "Gem archive exceeds size limit"
    );
    snapshot.seek(SeekFrom::Start(0))?;
    let metadata = rb_gem_archive::read(&mut snapshot)?;
    snapshot.seek(SeekFrom::Start(0))?;
    let staging = tempfile::Builder::new()
        .prefix(".rb-gem-")
        .tempdir_in(parent)?;
    extract::extract(&mut snapshot, staging.path())?;
    let result = work(&metadata, staging.path())?;
    crate::publish::publish(staging.path(), destination).context("Cannot publish installed gem")?;
    Ok((metadata, result))
}
