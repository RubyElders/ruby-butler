use anyhow::Result;
use std::path::Path;

pub(crate) fn publish(source: &Path, destination: &Path) -> Result<()> {
    #[cfg(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        target_os = "ios"
    ))]
    rustix::fs::renameat_with(
        rustix::fs::CWD,
        source,
        rustix::fs::CWD,
        destination,
        rustix::fs::RenameFlags::NOREPLACE,
    )?;
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let source: Vec<_> = source.as_os_str().encode_wide().chain(Some(0)).collect();
        let destination: Vec<_> = destination
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        anyhow::ensure!(
            !source[..source.len() - 1].contains(&0)
                && !destination[..destination.len() - 1].contains(&0),
            "Installation path contains a null character"
        );
        let moved = unsafe {
            windows_sys::Win32::Storage::FileSystem::MoveFileExW(
                source.as_ptr(),
                destination.as_ptr(),
                0,
            )
        };
        if moved == 0 {
            return Err(std::io::Error::last_os_error().into());
        }
    }
    #[cfg(not(any(
        target_os = "linux",
        target_os = "android",
        target_os = "macos",
        target_os = "ios",
        windows
    )))]
    anyhow::bail!(
        "Atomic installation is not supported on this platform: {} -> {}",
        source.display(),
        destination.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn publication_cannot_replace_an_empty_destination() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source");
        let destination = root.path().join("destination");
        std::fs::create_dir(&source).unwrap();
        std::fs::write(source.join("file"), b"content").unwrap();
        std::fs::create_dir(&destination).unwrap();
        assert!(super::publish(&source, &destination).is_err());
        assert!(source.join("file").is_file());
        assert_eq!(std::fs::read_dir(&destination).unwrap().count(), 0);
    }
}
