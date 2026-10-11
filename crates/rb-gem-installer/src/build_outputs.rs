use crate::{
    constants::{MAX_FILES, MAX_PAYLOAD_BYTES},
    payload_path,
};
use anyhow::{Context, Result, ensure};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn copy(source: &Path, destination: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    visit(source, source, &mut files)?;
    ensure!(files.len() <= MAX_FILES, "Too many build outputs");
    let mut bytes = 0_u64;
    for relative in &files {
        let from = source.join(relative);
        bytes = bytes
            .checked_add(fs::metadata(&from)?.len())
            .context("Build output size overflow")?;
        ensure!(
            bytes <= MAX_PAYLOAD_BYTES,
            "Build outputs exceed size limit"
        );
        let to = destination.join(relative);
        fs::create_dir_all(to.parent().context("Missing output parent")?)?;
        if fs::symlink_metadata(&to).is_ok() {
            ensure!(
                !fs::symlink_metadata(&to)?.file_type().is_symlink(),
                "Build destination is a symlink"
            );
        }
        fs::copy(from, to)?;
    }
    Ok(files)
}

fn visit(root: &Path, directory: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let path = entry.path();
        ensure!(
            !kind.is_symlink(),
            "Build output is a symlink: {}",
            path.display()
        );
        if kind.is_dir() {
            visit(root, &path, files)?;
        } else {
            ensure!(kind.is_file(), "Build output is not a regular file");
            files.push(payload_path::relative(path.strip_prefix(root)?)?);
            ensure!(files.len() <= MAX_FILES, "Too many build outputs");
        }
    }
    Ok(())
}

pub(crate) fn native_files(
    root: &Path,
    directory: &Path,
    suffix: &str,
    files: &mut Vec<PathBuf>,
) -> Result<()> {
    if !directory.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let path = entry.path();
        let kind = entry.file_type()?;
        if kind.is_dir() {
            native_files(root, &path, suffix, files)?;
        } else if path.is_file()
            && path.extension().is_some_and(|extension| {
                extension == suffix
                    || extension == "so"
                    || extension == "dll"
                    || extension == "dylib"
                    || extension == "bundle"
            })
        {
            files.push(path.strip_prefix(root)?.to_path_buf());
            ensure!(files.len() <= MAX_FILES, "Too many native artifacts");
        }
    }
    Ok(())
}
