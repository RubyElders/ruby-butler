use anyhow::Result;
use std::{fs, path::Path};

pub(crate) fn copy_tree(source: &Path, destination: &Path) -> Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let to = destination.join(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copy_tree(&entry.path(), &to)?;
        } else if kind.is_symlink() {
            #[cfg(unix)]
            std::os::unix::fs::symlink(fs::read_link(entry.path())?, to)?;
            #[cfg(windows)]
            {
                if entry.path().is_dir() {
                    std::os::windows::fs::symlink_dir(fs::read_link(entry.path())?, to)?;
                } else {
                    std::os::windows::fs::symlink_file(fs::read_link(entry.path())?, to)?;
                }
            }
        } else {
            fs::copy(entry.path(), to)?;
        }
    }
    Ok(())
}
