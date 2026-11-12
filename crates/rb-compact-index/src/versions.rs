use crate::{
    Client,
    cache::{atomic_write, checksum, lock},
    constants::{
        INDEX_CACHE_DIRECTORY, MAX_VERSIONS_BYTES, REGISTRY_METADATA_DIRECTORY,
        VERSIONS_CACHE_FILE, VERSIONS_PATH,
    },
    index_response::read_index_with_prefix,
};
use anyhow::{Context, Result, ensure};
use reqwest::{StatusCode, header};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

pub(crate) struct Versions {
    pub(crate) checksums: BTreeMap<String, String>,
}

#[derive(Serialize, Deserialize)]
struct CachedVersions {
    etag: Option<String>,
    body: Option<String>,
}

fn parse(body: &str) -> Result<Versions> {
    let mut lines = body.lines();
    ensure!(
        lines.by_ref().any(|line| line == "---"),
        "Missing versions index header"
    );
    let mut checksums = BTreeMap::new();
    for line in lines.filter(|line| !line.is_empty()) {
        let fields: Vec<_> = line.split_whitespace().collect();
        ensure!(
            (2..=3).contains(&fields.len()),
            "Invalid versions index entry"
        );
        ensure!(
            fields[0]
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte))
                && !matches!(fields[0], "." | ".."),
            "Invalid gem name in versions index"
        );
        checksums.insert(
            fields[0].to_owned(),
            fields.get(2).unwrap_or(&"").to_string(),
        );
    }
    Ok(Versions { checksums })
}

impl Client {
    /// Lists compact-index gem names, or returns None when that endpoint is unsupported.
    pub fn names(&self) -> Result<Option<BTreeSet<String>>> {
        Ok(self
            .versions()?
            .map(|versions| versions.checksums.keys().cloned().collect()))
    }

    pub(crate) fn versions(&self) -> Result<Option<&Versions>> {
        self.metadata
            .versions
            .get_or_init(|| self.fetch_versions().map_err(|error| format!("{error:#}")))
            .as_ref()
            .map(Option::as_ref)
            .map_err(|error| anyhow::anyhow!("{error}"))
    }

    fn fetch_versions(&self) -> Result<Option<Versions>> {
        let directory = self
            .cache
            .join(INDEX_CACHE_DIRECTORY)
            .join(checksum(self.source.as_bytes()))
            .join(REGISTRY_METADATA_DIRECTORY);
        fs::create_dir_all(&directory)?;
        let path = directory.join(VERSIONS_CACHE_FILE);
        let _guard = lock(&path.with_extension("lock"))?;
        let cached: Option<CachedVersions> = fs::read(&path).ok().and_then(|bytes| {
            serde_json::from_slice(&bytes).ok().or_else(|| {
                serde_json::from_slice::<Option<String>>(&bytes)
                    .ok()
                    .map(|body| CachedVersions { etag: None, body })
            })
        });
        if self.offline {
            return cached
                .context("No cached versions index (offline)")?
                .body
                .as_deref()
                .map(parse)
                .transpose();
        }
        let mut request = self
            .client
            .get(format!("{}{VERSIONS_PATH}", self.source))
            .header(header::ACCEPT_ENCODING, "identity");
        if let Some(cached) = &cached {
            if let Some(etag) = &cached.etag {
                request = request.header(header::IF_NONE_MATCH, etag);
            }
            if let Some(body) = &cached.body
                && !body.is_empty()
            {
                request = request.header(header::RANGE, format!("bytes={}-", body.len() - 1));
            }
        }
        let response = request.send()?;
        if response.status() == StatusCode::NOT_MODIFIED {
            let body = cached
                .and_then(|cached| cached.body)
                .context("Received 304 without cached versions")?;
            return Ok(Some(parse(&body)?));
        }
        if response.status() == StatusCode::NOT_FOUND {
            atomic_write(
                &path,
                &serde_json::to_vec(&CachedVersions {
                    etag: None,
                    body: None,
                })?,
            )?;
            return Ok(None);
        }
        let response = if response.status() == StatusCode::RANGE_NOT_SATISFIABLE {
            self.client
                .get(format!("{}{VERSIONS_PATH}", self.source))
                .header(header::ACCEPT_ENCODING, "identity")
                .send()?
        } else {
            response
        };
        let prefix = cached.as_ref().and_then(|cached| cached.body.as_deref());
        let partial = response.status() == StatusCode::PARTIAL_CONTENT;
        let etag = response
            .headers()
            .get(header::ETAG)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let bytes = read_index_with_prefix(
            response.error_for_status()?,
            MAX_VERSIONS_BYTES,
            prefix.map(str::as_bytes),
        );
        let (etag, bytes) = match bytes {
            Ok(bytes) => (etag, bytes),
            Err(_) if partial => {
                let response = self
                    .client
                    .get(format!("{}{VERSIONS_PATH}", self.source))
                    .header(header::ACCEPT_ENCODING, "identity")
                    .send()?
                    .error_for_status()?;
                let etag = response
                    .headers()
                    .get(header::ETAG)
                    .and_then(|value| value.to_str().ok())
                    .map(str::to_owned);
                (
                    etag,
                    read_index_with_prefix(response, MAX_VERSIONS_BYTES, None)?,
                )
            }
            Err(error) => return Err(error),
        };
        let body = String::from_utf8(bytes)?;
        let versions = parse(&body)?;
        atomic_write(
            &path,
            &serde_json::to_vec(&CachedVersions {
                etag,
                body: Some(body),
            })?,
        )?;
        Ok(Some(versions))
    }
}
