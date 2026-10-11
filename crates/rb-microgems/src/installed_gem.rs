use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct InstalledGem {
    pub name: String,
    pub version: String,
    pub platform: String,
    pub root: PathBuf,
    pub require_paths: Vec<String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    #[serde(default)]
    pub executables: Vec<String>,
    #[serde(default = "default_bindir")]
    pub bindir: String,
}

fn default_bindir() -> String {
    "bin".into()
}
