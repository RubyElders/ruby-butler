use anyhow::{Result, ensure};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

pub(crate) fn selected(root: &Path, paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    enumerate(root, paths, true)
}

pub(crate) fn payload_files(root: &Path) -> Result<Vec<PathBuf>> {
    enumerate(root, &[PathBuf::from(".")], false)
}

fn enumerate(root: &Path, paths: &[PathBuf], source: bool) -> Result<Vec<PathBuf>> {
    fn visit(
        root: &Path,
        path: &Path,
        output: &mut std::collections::BTreeSet<PathBuf>,
        source: bool,
    ) -> Result<()> {
        if source
            && matches!(
                path.file_name().and_then(|name| name.to_str()),
                Some(".git" | ".rb" | ".rb-native")
            )
        {
            return Ok(());
        }
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error.into()),
        };
        if source {
            ensure!(
                dunce::canonicalize(path)?.starts_with(dunce::canonicalize(root)?),
                "Source link escapes its directory"
            );
        }
        let kind = metadata.file_type();
        if kind.is_dir() {
            for entry in fs::read_dir(path)? {
                visit(root, &entry?.path(), output, source)?;
            }
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
            output.insert(relative.to_owned());
        }
        Ok(())
    }
    let mut output = std::collections::BTreeSet::new();
    for path in paths {
        let path: PathBuf = path
            .components()
            .filter(|part| !matches!(part, std::path::Component::CurDir))
            .collect();
        visit(root, &root.join(path), &mut output, source)?;
    }
    Ok(output.into_iter().collect())
}

pub(crate) fn fingerprint(root: &Path, paths: &[PathBuf]) -> Result<String> {
    let mut hash = Sha256::new();
    for path in selected(root, paths)? {
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
            let mut file = fs::File::open(absolute)?;
            let mut buffer = [0; crate::constants::READ_BUFFER_BYTES];
            loop {
                let count = file.read(&mut buffer)?;
                if count == 0 {
                    break;
                }
                hash.update(&buffer[..count]);
            }
        }
    }
    Ok(format!("{:x}", hash.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn selected_files_cannot_escape_through_symlinked_parents() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("secret"), "not package data").unwrap();
        std::os::unix::fs::symlink(outside.path(), root.path().join("link")).unwrap();
        let error = selected(root.path(), &[PathBuf::from("link/secret")]).unwrap_err();
        assert!(error.to_string().contains("Source link escapes"));
    }

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
