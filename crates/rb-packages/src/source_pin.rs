use crate::{PackageRecord, PackageSource};
use anyhow::Result;
use rb_gem_types::{PackageId, Requirement};
use std::collections::BTreeMap;

pub(crate) struct SourcePin {
    pub(crate) id: PackageId,
    pub(crate) source: PackageSource,
    pub(crate) checksum: Option<String>,
    dependencies: BTreeMap<String, Requirement>,
}

impl SourcePin {
    pub(crate) fn new(
        id: PackageId,
        source: PackageSource,
        checksum: Option<String>,
        package: &rb_lockfile::LockedPackage,
    ) -> Result<Self> {
        let dependencies = package
            .dependencies
            .iter()
            .map(|(name, requirements)| {
                Ok((
                    name.clone(),
                    requirements
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                        .parse()?,
                ))
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            id,
            source,
            checksum,
            dependencies,
        })
    }
    pub(crate) fn matches(&self, record: &PackageRecord) -> bool {
        self.id == record.package.id
            && self.source == record.source
            && self
                .checksum
                .as_ref()
                .is_none_or(|sum| sum.eq_ignore_ascii_case(&record.checksum))
    }
    pub(crate) fn dependencies_match(&self, record: &PackageRecord) -> bool {
        self.dependencies.len() == record.package.dependencies.len()
            && self.dependencies.iter().all(|(name, requirement)| {
                record
                    .package
                    .dependencies
                    .get(name)
                    .is_some_and(|expected| {
                        crate::validation::same_requirement(requirement, expected)
                    })
            })
    }
}
