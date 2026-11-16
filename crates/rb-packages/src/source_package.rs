use crate::{PackageRecord, PackageSource, source_specs::SourceSpecification};
use anyhow::{Result, ensure};
use std::path::{Path, PathBuf};

pub(crate) struct SourcePackage {
    pub(crate) record: PackageRecord,
    pub(crate) metadata: rb_gem_archive::GemMetadata,
    pub(crate) directory: PathBuf,
    pub(crate) gemspec: PathBuf,
    pub(crate) serialized: String,
    pub(crate) snapshot: String,
    pub(crate) payload: Vec<PathBuf>,
    pub(crate) inputs: Vec<PathBuf>,
    pub(crate) environment: Option<std::collections::BTreeSet<String>>,
}

impl SourcePackage {
    pub(crate) fn new(
        spec: SourceSpecification,
        source: &PackageSource,
        directory: &Path,
        snapshot: &str,
    ) -> Result<Self> {
        let gem_root = directory.join(spec.gemspec.parent().unwrap());
        ensure!(
            gem_root.starts_with(directory),
            "Gemspec directory escapes source"
        );
        let inputs = spec.tracked_paths(directory)?;
        let payload = spec.payload(&gem_root)?;
        let metadata = rb_gem_archive::parse_metadata(spec.metadata.as_bytes())?;
        let checksum = crate::PackageInput::checksum(
            &metadata,
            &crate::source_files::fingerprint(&gem_root, &payload)?,
        )?;
        let record = PackageRecord {
            archive_version: metadata.id.archive_version(),
            package: rb_resolver::Package {
                id: metadata.id.clone(),
                dependencies: metadata.dependencies.clone(),
                ruby: metadata.ruby.clone(),
            },
            source: source.clone(),
            checksum,
            declared_ruby: Some(metadata.ruby.clone()),
            rubygems: Some(metadata.rubygems.clone()),
        };
        record.validate()?;
        Ok(Self {
            record,
            metadata,
            directory: directory.to_owned(),
            gemspec: spec.gemspec,
            serialized: spec.specification,
            snapshot: snapshot.into(),
            payload,
            inputs,
            environment: spec.environment,
        })
    }
}
