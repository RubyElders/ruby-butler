use anyhow::{Context, Result, ensure};
use rb_compact_index::GemInfo;
use rb_gem_types::{PackageId, Platform};
use rb_resolver::Package;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageRecord {
    pub package: Package,
    pub source: crate::PackageSource,
    pub archive_version: String,
    pub checksum: String,
    pub rubygems: Option<rb_gem_types::Requirement>,
}

impl PackageRecord {
    pub(crate) fn validate(&self) -> Result<()> {
        for value in [&self.package.id.name, &self.archive_version] {
            ensure!(
                !value.is_empty()
                    && value != "."
                    && value != ".."
                    && value
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric()
                            || matches!(byte, b'_' | b'-' | b'.')),
                "Invalid package archive identity"
            );
        }
        let (version, platform) = self
            .archive_version
            .split_once('-')
            .unwrap_or((&self.archive_version, "ruby"));
        ensure!(
            version.parse::<rb_gem_types::Version>()? == self.package.id.version
                && Platform::parse(platform) == self.package.id.platform,
            "Archive filename does not match package identity"
        );
        ensure!(
            self.checksum.len() == crate::constants::CHECKSUM_CHARACTERS
                && self.checksum.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "Invalid archive checksum"
        );
        self.source.validate()?;
        Ok(())
    }

    pub fn from_info(name: &str, source: &str, record: GemInfo) -> Result<Self> {
        let (version, platform) = record
            .version
            .split_once('-')
            .unwrap_or((&record.version, "ruby"));
        Ok(Self {
            package: Package {
                id: PackageId {
                    name: name.into(),
                    version: version.parse()?,
                    platform: Platform::parse(platform),
                },
                dependencies: record
                    .dependencies
                    .iter()
                    .map(|(name, value)| Ok((name.clone(), value.replace('&', ", ").parse()?)))
                    .collect::<Result<_>>()?,
                ruby: record
                    .metadata
                    .get("ruby")
                    .map(|v| v.replace('&', ", ").parse())
                    .transpose()?
                    .unwrap_or_default(),
            },
            rubygems: record
                .metadata
                .get("rubygems")
                .map(|v| v.replace('&', ", ").parse())
                .transpose()?,
            source: source_url(source)?.into(),
            archive_version: record.version.clone(),
            checksum: record
                .metadata
                .get("checksum")
                .context("Missing archive checksum")?
                .clone(),
        })
    }
}

pub trait PackageIndex: Send + Sync {
    fn candidates(&self, name: &str) -> Result<Vec<PackageRecord>>;
    fn archive(&self, record: &PackageRecord) -> Result<PathBuf>;
    fn stage_sources(
        &self,
        _home: &Path,
        plan: &crate::PackagePlan,
        _runtime: &crate::RubyEnvironment,
    ) -> Result<bool> {
        ensure!(
            plan.packages
                .iter()
                .all(|package| matches!(package.record.source, crate::PackageSource::Registry(_))),
            "This index does not provide source snapshots"
        );
        Ok(false)
    }
    fn compatibility_path(&self, _record: &PackageRecord, _home: &Path) -> Result<Option<PathBuf>> {
        Ok(None)
    }
}

#[derive(Default)]
pub struct MemoryIndex {
    pub packages: BTreeMap<String, Vec<PackageRecord>>,
    pub archives: BTreeMap<String, PathBuf>,
}

impl PackageIndex for MemoryIndex {
    fn candidates(&self, name: &str) -> Result<Vec<PackageRecord>> {
        Ok(self.packages.get(name).cloned().unwrap_or_default())
    }
    fn archive(&self, record: &PackageRecord) -> Result<PathBuf> {
        self.archives
            .get(&record.checksum)
            .cloned()
            .context("Archive is not available in the memory index")
    }
}

pub(crate) fn source_url(source: &str) -> Result<String> {
    let mut url = url::Url::parse(source)?;
    ensure!(
        matches!(url.scheme(), "http" | "https")
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none(),
        "Invalid registry source"
    );
    if !url.path().ends_with('/') {
        url.set_path(&format!("{}/", url.path()));
    }
    Ok(url.to_string())
}
