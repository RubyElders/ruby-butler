use anyhow::{Result, ensure};
use rb_gem_archive::GemMetadata;
use std::path::{Path, PathBuf};

pub enum PackageInput {
    Archive(PathBuf),
    Directory {
        root: PathBuf,
        files: Vec<PathBuf>,
        metadata: Box<GemMetadata>,
    },
}

impl PackageInput {
    pub(crate) fn stage(self, destination: &Path, checksum: &str) -> Result<GemMetadata> {
        match self {
            Self::Archive(path) => {
                crate::validation::verify(&path, checksum)?;
                rb_gem_installer::install(std::fs::File::open(path)?, destination)
            }
            Self::Directory {
                root,
                files,
                metadata,
            } => {
                let files = crate::source_files::selected(&root, &files)?;
                crate::source_files::copy_files(&root, destination, &files)?;
                ensure!(
                    Self::checksum(
                        &metadata,
                        &crate::source_files::fingerprint(destination, &files)?
                    )? == checksum,
                    "Source files changed during staging"
                );
                Ok(*metadata)
            }
        }
    }

    pub(crate) fn checksum(metadata: &GemMetadata, payload: &str) -> Result<String> {
        use sha2::{Digest, Sha256};
        Ok(format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&(metadata, payload))?)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn metadata() -> GemMetadata {
        rb_gem_archive::parse_metadata(b"name: fixture\nversion: '1.0.0'\n").unwrap()
    }

    fn input(root: &Path) -> (PackageInput, String) {
        let files = vec![PathBuf::from("lib")];
        let checksum = PackageInput::checksum(
            &metadata(),
            &crate::source_files::fingerprint(root, &files).unwrap(),
        )
        .unwrap();
        (
            PackageInput::Directory {
                root: root.to_owned(),
                files,
                metadata: Box::new(metadata()),
            },
            checksum,
        )
    }

    #[test]
    fn directory_inputs_stage_private_payloads_and_detect_changes() {
        let source = tempfile::tempdir().unwrap();
        let staged = tempfile::tempdir().unwrap();
        fs::create_dir(source.path().join("lib")).unwrap();
        let file = source.path().join("lib/fixture.rb");
        fs::write(&file, "VALUE=42\n").unwrap();
        let (package, checksum) = input(source.path());
        assert_eq!(
            package
                .stage(&staged.path().join("gem"), &checksum)
                .unwrap(),
            metadata()
        );
        fs::write(staged.path().join("gem/lib/fixture.rb"), "VALUE=0\n").unwrap();
        assert_eq!(fs::read_to_string(&file).unwrap(), "VALUE=42\n");
        let (package, checksum) = input(source.path());
        fs::write(&file, "VALUE=43\n").unwrap();
        let error = package
            .stage(&staged.path().join("changed"), &checksum)
            .unwrap_err();
        assert!(error.to_string().contains("changed during staging"));
    }

    #[cfg(unix)]
    #[test]
    fn directory_inputs_preserve_links_modes_and_ignore_timestamps() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let source = tempfile::tempdir().unwrap();
        let staged = tempfile::tempdir().unwrap();
        fs::create_dir(source.path().join("lib")).unwrap();
        let file = source.path().join("lib/fixture.rb");
        fs::write(&file, "VALUE=42\n").unwrap();
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).unwrap();
        symlink("fixture.rb", source.path().join("lib/link.rb")).unwrap();
        let (package, checksum) = input(source.path());
        fs::File::options()
            .write(true)
            .open(&file)
            .unwrap()
            .set_modified(
                std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000_000),
            )
            .unwrap();
        assert_eq!(input(source.path()).1, checksum);
        let destination = staged.path().join("gem");
        package.stage(&destination, &checksum).unwrap();
        assert_eq!(
            fs::read_link(destination.join("lib/link.rb")).unwrap(),
            Path::new("fixture.rb")
        );
        assert_eq!(
            fs::metadata(destination.join("lib/fixture.rb"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o755
        );
    }

    #[test]
    fn directory_inputs_reject_escaping_paths() {
        let source = tempfile::tempdir().unwrap();
        let staged = tempfile::tempdir().unwrap();
        let package = PackageInput::Directory {
            root: source.path().to_owned(),
            files: vec!["..".into()],
            metadata: Box::new(metadata()),
        };
        assert!(
            package
                .stage(&staged.path().join("gem"), "invalid")
                .is_err()
        );
    }
}
