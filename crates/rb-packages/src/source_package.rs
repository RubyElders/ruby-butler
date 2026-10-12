use crate::{PackageRecord, PackageSource, source_specs::SourceSpecification};
use anyhow::{Result, ensure};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) struct SourcePackage {
    pub(crate) record: PackageRecord,
    pub(crate) archive: PathBuf,
    pub(crate) directory: PathBuf,
    pub(crate) gemspec: PathBuf,
    pub(crate) serialized: String,
    pub(crate) snapshot: String,
    pub(crate) payload: Vec<PathBuf>,
}

impl SourcePackage {
    pub(crate) fn new(
        spec: SourceSpecification,
        source: &PackageSource,
        directory: &Path,
        cache: &Path,
        snapshot: &str,
    ) -> Result<Self> {
        let gem_root = directory.join(spec.gemspec.parent().unwrap());
        ensure!(
            gem_root.starts_with(directory),
            "Gemspec directory escapes source"
        );
        let payload = spec.payload(&gem_root)?;
        let bytes = crate::source_archive::archive(&gem_root, &spec.metadata, &payload)?;
        let checksum = format!("{:x}", Sha256::digest(&bytes));
        let metadata = rb_gem_archive::read(bytes.as_slice())?;
        let record = PackageRecord {
            archive_version: metadata.id.archive_version(),
            package: rb_resolver::Package {
                id: metadata.id.clone(),
                dependencies: metadata.dependencies.clone(),
                ruby: metadata.ruby.clone(),
            },
            source: source.clone(),
            checksum: checksum.clone(),
            rubygems: Some(metadata.rubygems),
        };
        record.validate()?;
        let artifact = cache
            .join(crate::constants::SOURCE_ARCHIVES_DIRECTORY)
            .join(format!("{checksum}.gem"));
        fs::create_dir_all(artifact.parent().unwrap())?;
        if crate::validation::verify(&artifact, &checksum).is_err() {
            use std::io::Write;
            let mut temporary = tempfile::NamedTempFile::new_in(artifact.parent().unwrap())?;
            temporary.write_all(&bytes)?;
            temporary.persist(&artifact).map_err(|error| error.error)?;
        }
        Ok(Self {
            record,
            archive: artifact,
            directory: directory.to_owned(),
            gemspec: spec.gemspec,
            serialized: spec.specification,
            snapshot: snapshot.into(),
            payload,
        })
    }
}
