use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize)]
pub(crate) struct PackageCache {
    pub paths: Vec<PathBuf>,
    #[serde(flatten)]
    pub inputs: rb_packages::CacheInputs,
}
