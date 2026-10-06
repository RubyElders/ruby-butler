use crate::{
    GemInfo,
    cache::{atomic_write, checksum, limited_read, lock, stream_archive},
    constants::{
        ARCHIVE_CACHE_DIRECTORY, ARCHIVE_PATH_PREFIX, CONNECT_TIMEOUT, FULL_INFO_CACHE_SUFFIX,
        INDEX_CACHE_DIRECTORY, INFO_PATH_PREFIX, MAX_COMPACT_INFO_BYTES, REQUEST_TIMEOUT,
        VERSIONS_PATH,
    },
    parse,
    validation::{validate_checksum, validate_name},
};
use anyhow::{Context, Result, bail, ensure};
use base64::Engine;
use reqwest::{Url, blocking::Client as HttpClient, header};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Clone)]
pub struct Client {
    pub(crate) source: String,
    pub(crate) cache: PathBuf,
    offline: bool,
    pub(crate) client: HttpClient,
}

#[derive(Serialize, Deserialize)]
struct CachedInfo {
    etag: Option<String>,
    body: String,
}

impl Client {
    pub fn new(source: &str, cache: &Path, offline: bool) -> Result<Self> {
        let mut url = Url::parse(source)?;
        ensure!(
            matches!(url.scheme(), "https" | "http") && url.host_str().is_some(),
            "Unsupported gem source: {source}"
        );
        ensure!(
            url.username().is_empty()
                && url.password().is_none()
                && url.query().is_none()
                && url.fragment().is_none(),
            "Gem source must be a base URL without credentials, query, or fragment"
        );
        if !url.path().ends_with('/') {
            url.set_path(&format!("{}/", url.path()));
        }
        Ok(Self {
            source: url.to_string(),
            cache: cache.to_path_buf(),
            offline,
            client: HttpClient::builder()
                .timeout(REQUEST_TIMEOUT)
                .connect_timeout(CONNECT_TIMEOUT)
                .user_agent(concat!("rb-compact-index/", env!("CARGO_PKG_VERSION")))
                .build()?,
        })
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn info(&self, name: &str) -> Result<Vec<GemInfo>> {
        validate_name(name)?;
        let directory = self
            .cache
            .join(INDEX_CACHE_DIRECTORY)
            .join(checksum(self.source.as_bytes()));
        fs::create_dir_all(&directory)?;
        let _guard = lock(&directory.join(format!("{name}.lock")))?;
        let path = directory.join(format!("{name}.json"));
        let cached: Option<CachedInfo> = fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok());
        if self.offline {
            if cached.is_none() {
                let full = directory.join(format!("{name}{FULL_INFO_CACHE_SUFFIX}"));
                if full.exists() {
                    return Ok(serde_json::from_slice(&fs::read(full)?)?);
                }
            }
            return parse(
                name,
                &cached
                    .context(format!("No cached metadata for {name} (offline)"))?
                    .body,
            );
        }
        let mut request = self
            .client
            .get(format!("{}{INFO_PATH_PREFIX}{name}", self.source))
            .header(header::ACCEPT_ENCODING, "identity");
        if let Some(etag) = cached.as_ref().and_then(|c| c.etag.as_ref()) {
            request = request.header(header::IF_NONE_MATCH, etag);
        }
        let response = request
            .send()
            .with_context(|| format!("Fetching metadata for {name}"))?;
        if response.status() == reqwest::StatusCode::NOT_MODIFIED {
            return parse(
                name,
                &cached.context("Received 304 without cached metadata")?.body,
            );
        }
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            let versions = self
                .client
                .head(format!("{}{VERSIONS_PATH}", self.source))
                .send()?;
            if versions.status() == reqwest::StatusCode::NOT_FOUND {
                return self.full_info(name, &directory);
            }
        }
        let response = response.error_for_status()?;
        let etag = response
            .headers()
            .get(header::ETAG)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let digest = response
            .headers()
            .get("repr-digest")
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let bytes = limited_read(response, MAX_COMPACT_INFO_BYTES)?;
        if let Some(digest) = digest
            && let Some(value) = digest
                .split(',')
                .find_map(|part| part.trim().strip_prefix("sha-256="))
        {
            let expected =
                base64::engine::general_purpose::STANDARD.decode(value.trim_matches([':', '"']))?;
            ensure!(
                sha2::Sha256::digest(&bytes).as_slice() == expected,
                "Index digest mismatch for {name}"
            );
        }
        let body = String::from_utf8(bytes)?;
        let gems = parse(name, &body)?;
        atomic_write(&path, &serde_json::to_vec(&CachedInfo { etag, body })?)?;
        Ok(gems)
    }

    pub fn download(&self, name: &str, version: &str, expected_checksum: &str) -> Result<PathBuf> {
        validate_name(name)?;
        validate_name(version)?;
        validate_checksum(expected_checksum)?;
        let directory = self.cache.join(ARCHIVE_CACHE_DIRECTORY);
        fs::create_dir_all(&directory)?;
        let path = directory.join(format!("{}.gem", expected_checksum));
        let _guard = lock(&directory.join(format!("{}.lock", expected_checksum)))?;
        if path.exists() {
            if stream_archive(File::open(&path)?, std::io::sink())? == expected_checksum {
                return Ok(path);
            }
            if self.offline {
                bail!("Cached archive checksum mismatch for {}", name);
            }
        }
        ensure!(
            !self.offline,
            "Archive for {} {} is not cached (offline)",
            name,
            version
        );
        let url = format!(
            "{}{ARCHIVE_PATH_PREFIX}{}-{}.gem",
            self.source, name, version
        );
        let mut archive = tempfile::NamedTempFile::new_in(&directory)?;
        let digest = stream_archive(
            self.client.get(&url).send()?.error_for_status()?,
            &mut archive,
        )?;
        ensure!(
            digest == expected_checksum,
            "Archive checksum mismatch for {} {}",
            name,
            version
        );
        archive.flush()?;
        archive.persist(&path)?;
        Ok(path)
    }
}
