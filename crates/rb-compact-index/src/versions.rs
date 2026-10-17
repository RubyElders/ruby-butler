use crate::{
    Client,
    cache::{atomic_write, checksum, lock},
    client::read_index,
    constants::{
        INDEX_CACHE_DIRECTORY, MAX_VERSIONS_BYTES, REGISTRY_METADATA_DIRECTORY,
        VERSIONS_CACHE_FILE, VERSIONS_PATH,
    },
    validation::validate_name,
};
use anyhow::{Context, Result, ensure};
use std::{collections::BTreeSet, fs};

fn parse(body: &str) -> Result<BTreeSet<String>> {
    let mut lines = body.lines();
    ensure!(
        lines.by_ref().any(|line| line == "---"),
        "Missing versions index header"
    );
    let mut names = BTreeSet::new();
    for line in lines.filter(|line| !line.is_empty()) {
        let fields: Vec<_> = line.split_whitespace().collect();
        ensure!(
            (2..=3).contains(&fields.len()),
            "Invalid versions index entry"
        );
        validate_name(fields[0])?;
        names.insert(fields[0].to_owned());
    }
    Ok(names)
}

impl Client {
    /// Lists compact-index gem names, or returns None when that endpoint is unsupported.
    pub fn names(&self) -> Result<Option<BTreeSet<String>>> {
        let directory = self
            .cache
            .join(INDEX_CACHE_DIRECTORY)
            .join(checksum(self.source.as_bytes()))
            .join(REGISTRY_METADATA_DIRECTORY);
        fs::create_dir_all(&directory)?;
        let path = directory.join(VERSIONS_CACHE_FILE);
        let _guard = lock(&path.with_extension("lock"))?;
        if self.offline {
            let body: Option<String> = serde_json::from_slice(
                &fs::read(&path).context("No cached versions index (offline)")?,
            )?;
            return body.as_deref().map(parse).transpose();
        }
        let response = self
            .client
            .get(format!("{}{VERSIONS_PATH}", self.source))
            .header(reqwest::header::ACCEPT_ENCODING, "identity")
            .send()?;
        let body = if response.status() == reqwest::StatusCode::NOT_FOUND {
            None
        } else {
            Some(String::from_utf8(read_index(
                response.error_for_status()?,
                MAX_VERSIONS_BYTES,
            )?)?)
        };
        let names = body.as_deref().map(parse).transpose()?;
        atomic_write(&path, &serde_json::to_vec(&body)?)?;
        Ok(names)
    }
}
