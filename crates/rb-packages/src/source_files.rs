use anyhow::{Result, ensure};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn files(root: &Path) -> Result<Vec<PathBuf>> {
    enumerate(root, true)
}
pub(crate) fn payload_files(root: &Path) -> Result<Vec<PathBuf>> {
    enumerate(root, false)
}

fn enumerate(root: &Path, source: bool) -> Result<Vec<PathBuf>> {
    fn visit(root: &Path, path: &Path, output: &mut Vec<PathBuf>, source: bool) -> Result<()> {
        let mut entries = fs::read_dir(path)?.collect::<std::io::Result<Vec<_>>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            if source
                && matches!(
                    entry.file_name().to_str(),
                    Some(".git" | ".rb" | ".rb-native")
                )
            {
                continue;
            }
            let path = entry.path();
            let kind = entry.file_type()?;
            if kind.is_dir() {
                visit(root, &path, output, source)?;
            } else {
                ensure!(
                    kind.is_file() || kind.is_symlink(),
                    "Unsupported source file"
                );
                let relative = path.strip_prefix(root)?;
                ensure!(
                    !relative.components().any(|part| part
                        .as_os_str()
                        .as_encoded_bytes()
                        .iter()
                        .any(|byte| matches!(byte, b'\\' | b':'))),
                    "Nonportable source file"
                );
                if source && kind.is_symlink() {
                    ensure!(
                        dunce::canonicalize(&path)?.starts_with(dunce::canonicalize(root)?),
                        "Source link escapes its directory"
                    );
                }
                output.push(relative.to_owned());
            }
        }
        Ok(())
    }
    let mut output = Vec::new();
    visit(root, root, &mut output, source)?;
    Ok(output)
}

pub(crate) fn fingerprint(root: &Path) -> Result<String> {
    let mut hash = Sha256::new();
    for path in files(root)? {
        let name = path.as_os_str().as_encoded_bytes();
        hash.update(name.len().to_le_bytes());
        hash.update(name);
        let absolute = root.join(&path);
        let metadata = fs::symlink_metadata(&absolute)?;
        if metadata.file_type().is_symlink() {
            hash.update(b"link");
            hash.update(fs::read_link(absolute)?.as_os_str().as_encoded_bytes());
        } else {
            hash.update(b"file");
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                hash.update(metadata.permissions().mode().to_le_bytes());
            }
            hash.update(metadata.len().to_le_bytes());
            hash.update(fs::read(absolute)?);
        }
    }
    Ok(format!("{:x}", hash.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_filesystem_separators_work_in_nested_payloads() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("lib")).unwrap();
        fs::write(root.path().join("lib").join("fixture.rb"), "answer=42\n").unwrap();
        assert_eq!(
            payload_files(root.path()).unwrap(),
            [PathBuf::from("lib").join("fixture.rb")]
        );
    }
}
