use anyhow::Result;
use flate2::{Compression, write::GzEncoder};
use std::{io::Write, path::Path};

pub(crate) fn archive(
    root: &Path,
    metadata: &str,
    files: &[std::path::PathBuf],
) -> Result<Vec<u8>> {
    let mut payload = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::default()));
    payload.follow_symlinks(false);
    payload.mode(tar::HeaderMode::Deterministic);
    for path in files {
        payload.append_path_with_name(root.join(path), path)?;
    }
    let payload = payload.into_inner()?.finish()?;
    let mut metadata_gzip = GzEncoder::new(Vec::new(), Compression::default());
    metadata_gzip.write_all(metadata.as_bytes())?;
    let mut outer = tar::Builder::new(Vec::new());
    for (name, bytes) in [
        (
            crate::constants::GEM_METADATA_MEMBER,
            metadata_gzip.finish()?,
        ),
        (crate::constants::GEM_PAYLOAD_MEMBER, payload),
    ] {
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        outer.append_data(&mut header, name, bytes.as_slice())?;
    }
    Ok(outer.into_inner()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        time::{Duration, SystemTime},
    };

    #[test]
    fn source_archives_ignore_checkout_timestamps_but_keep_payload_changes() {
        let root = tempfile::tempdir().unwrap();
        let file = root.path().join("source.rb");
        fs::write(&file, "VALUE = 1\n").unwrap();
        let set_time = |seconds| {
            fs::File::options()
                .write(true)
                .open(&file)
                .unwrap()
                .set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(seconds))
                .unwrap();
        };
        let files = [std::path::PathBuf::from("source.rb")];
        set_time(1_000_000);
        let first = archive(root.path(), "metadata", &files).unwrap();
        set_time(2_000_000);
        let second = archive(root.path(), "metadata", &files).unwrap();
        assert!(
            first == second,
            "Unchanged source contents must have identical archives"
        );
        fs::write(&file, "VALUE = 2\n").unwrap();
        assert!(first != archive(root.path(), "metadata", &files).unwrap());
    }

    #[cfg(unix)]
    #[test]
    fn source_archives_preserve_executable_bits_and_links() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let root = tempfile::tempdir().unwrap();
        let script = root.path().join("command");
        fs::write(&script, "#!/bin/sh\n").unwrap();
        fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
        symlink("command", root.path().join("alias")).unwrap();
        let bytes = archive(root.path(), "metadata", &["command".into(), "alias".into()]).unwrap();
        let mut outer = tar::Archive::new(bytes.as_slice());
        let mut payload = Vec::new();
        for entry in outer.entries().unwrap() {
            let mut entry = entry.unwrap();
            if entry.path().unwrap() == Path::new(crate::constants::GEM_PAYLOAD_MEMBER) {
                std::io::Read::read_to_end(&mut entry, &mut payload).unwrap();
            }
        }
        let mut inner = tar::Archive::new(flate2::read::GzDecoder::new(payload.as_slice()));
        let mut entries = inner.entries().unwrap();
        let script = entries.next().unwrap().unwrap();
        assert_eq!(script.path().unwrap(), Path::new("command"));
        assert_eq!(script.header().mode().unwrap() & 0o777, 0o755);
        let alias = entries.next().unwrap().unwrap();
        assert!(alias.header().entry_type().is_symlink());
        assert_eq!(alias.link_name().unwrap().unwrap(), Path::new("command"));
        assert!(entries.next().is_none());
    }
}
