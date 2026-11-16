use crate::{constants::INSTALL_RECEIPT, copy_tree::copy_tree, receipt::Receipt};
use anyhow::Result;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn stage(
    root: &Path,
    gem: &rb_microgems::InstalledGem,
    receipt: &Receipt,
    runtime: &crate::RubyEnvironment,
) -> Result<PathBuf> {
    let key = crate::installation_cache::key(
        receipt,
        &format!("{}-{}", gem.name, receipt.archive_version),
        runtime,
    )?;
    fs::create_dir_all(root)?;
    let root = dunce::canonicalize(root)?;
    let target = root.join(&key);
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(root.join(format!("{key}.lock")))?;
    lock.lock()?;
    if !target.join(INSTALL_RECEIPT).is_file() {
        let temporary = tempfile::tempdir_in(&root)?;
        let staged = temporary.path().join("gem");
        copy_tree(&gem.root, &staged)?;
        if target.exists() {
            fs::remove_dir_all(&target)?;
        }
        fs::rename(staged, &target)?;
    }
    Ok(target)
}
