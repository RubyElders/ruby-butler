use crate::Package;
use std::{collections::BTreeMap, convert::Infallible, error::Error};

/// Supplies all candidates for one gem; transport and source selection belong to the caller.
pub trait PackageProvider {
    type Error: Into<Box<dyn Error + Send + Sync>>;

    fn candidates(&self, name: &str) -> Result<Vec<Package>, Self::Error>;
}

/// An offline candidate index for resolution and tests.
#[derive(Clone, Debug, Default)]
pub struct InMemoryIndex {
    packages: BTreeMap<String, Vec<Package>>,
}

impl InMemoryIndex {
    pub fn new(packages: impl IntoIterator<Item = Package>) -> Self {
        let mut index = Self::default();
        for package in packages {
            index.insert(package);
        }
        index
    }

    pub fn insert(&mut self, package: Package) {
        self.packages
            .entry(package.id.name.clone())
            .or_default()
            .push(package);
    }
}

impl PackageProvider for InMemoryIndex {
    type Error = Infallible;

    fn candidates(&self, name: &str) -> Result<Vec<Package>, Self::Error> {
        Ok(self.packages.get(name).cloned().unwrap_or_default())
    }
}
