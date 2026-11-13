use crate::{
    GemInfo,
    cache::{atomic_write, checksum, lock, stream_archive},
    constants::{
        ARCHIVE_CACHE_DIRECTORY, ARCHIVE_PATH_PREFIX, CONNECT_TIMEOUT, FULL_INFO_CACHE_SUFFIX,
        INDEX_CACHE_DIRECTORY, INFO_PATH_PREFIX, MAX_COMPACT_INFO_BYTES, REQUEST_TIMEOUT,
    },
    index_response::read_index,
    parse,
    validation::{validate_checksum, validate_name},
};
use anyhow::{Context, Result, bail, ensure};
use base64::Engine;
use md5::{Digest, Md5};
use reqwest::{Url, blocking::Client as HttpClient, header};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
};

#[derive(Default)]
pub(crate) struct MetadataState {
    pub(crate) versions: OnceLock<Result<Option<crate::versions::Versions>, String>>,
    records: Mutex<BTreeMap<(String, bool), Vec<GemInfo>>>,
}

#[derive(Clone)]
pub struct Client {
    pub(crate) source: String,
    pub(crate) cache: PathBuf,
    pub(crate) offline: bool,
    pub(crate) client: HttpClient,
    pub(crate) metadata: Arc<MetadataState>,
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
            metadata: Arc::default(),
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

    pub fn with_credentials(mut self, credentials: &str) -> Result<Self> {
        let mut headers = header::HeaderMap::new();
        let credentials = if credentials.contains(':') {
            credentials.to_owned()
        } else {
            format!("{credentials}:")
        };
        let mut authorization = header::HeaderValue::from_str(&format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(credentials)
        ))?;
        authorization.set_sensitive(true);
        headers.insert(header::AUTHORIZATION, authorization);
        self.metadata = Arc::default();
        self.client = HttpClient::builder()
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .user_agent(concat!("rb-compact-index/", env!("CARGO_PKG_VERSION")))
            .default_headers(headers)
            .build()?;
        Ok(self)
    }

    pub fn info(&self, name: &str) -> Result<Vec<GemInfo>> {
        self.fetch_info(name, false)
    }

    /// Returns no candidates for a missing gem on a working compact-index server.
    pub fn info_or_empty(&self, name: &str) -> Result<Vec<GemInfo>> {
        self.fetch_info(name, true)
    }

    /// Reads existing source metadata without refreshing the registry.
    pub fn cached_info(&self, name: &str) -> Result<Option<Vec<GemInfo>>> {
        validate_name(name)?;
        let directory = self
            .cache
            .join(INDEX_CACHE_DIRECTORY)
            .join(checksum(self.source.as_bytes()));
        let cached: Option<CachedInfo> = fs::read(directory.join(format!("{name}.json")))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok());
        if let Some(cached) = cached {
            return Ok(Some(parse(name, &cached.body)?));
        }
        match fs::read(directory.join(format!("{name}{FULL_INFO_CACHE_SUFFIX}"))) {
            Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn fetch_info(&self, name: &str, allow_missing: bool) -> Result<Vec<GemInfo>> {
        validate_name(name)?;
        let directory = self
            .cache
            .join(INDEX_CACHE_DIRECTORY)
            .join(checksum(self.source.as_bytes()));
        fs::create_dir_all(&directory)?;
        let _guard = lock(&directory.join(format!("{name}.lock")))?;
        let key = (name.to_owned(), allow_missing);
        if let Some(records) = self.metadata.records.lock().unwrap().get(&key) {
            return Ok(records.clone());
        }
        let result = self.read_info(name, allow_missing, &directory)?;
        self.metadata
            .records
            .lock()
            .unwrap()
            .insert(key, result.clone());
        Ok(result)
    }

    fn read_info(&self, name: &str, allow_missing: bool, directory: &Path) -> Result<Vec<GemInfo>> {
        if self.offline {
            return self
                .cached_info(name)?
                .context(format!("No cached metadata for {name} (offline)"));
        }
        let path = directory.join(format!("{name}.json"));
        let cached: Option<CachedInfo> = fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok());
        let versions = self.versions()?;
        let expected = versions.and_then(|versions| versions.checksums.get(name));
        if versions.is_some()
            && let Some(cached) = &cached
            && expected.is_none_or(|expected| {
                format!("{:x}", Md5::digest(cached.body.as_bytes())).eq_ignore_ascii_case(expected)
            })
        {
            return parse(name, &cached.body);
        }
        let mut request = self
            .client
            .get(format!("{}{INFO_PATH_PREFIX}{name}", self.source))
            .header(header::ACCEPT_ENCODING, "identity");
        if expected.is_none_or(String::is_empty)
            && let Some(etag) = cached.as_ref().and_then(|c| c.etag.as_ref())
        {
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
            if self.versions()?.is_none() {
                return self.full_info(name, directory);
            }
            if allow_missing {
                atomic_write(
                    &path,
                    &serde_json::to_vec(&CachedInfo {
                        etag: None,
                        body: "---\n".into(),
                    })?,
                )?;
                return Ok(Vec::new());
            }
        }
        let response = response.error_for_status()?;
        let etag = response
            .headers()
            .get(header::ETAG)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned);
        let bytes = read_index(response, MAX_COMPACT_INFO_BYTES)?;
        let body = String::from_utf8(bytes)?;
        let gems = parse(name, &body)?;
        atomic_write(&path, &serde_json::to_vec(&CachedInfo { etag, body })?)?;
        Ok(gems)
    }

    pub(crate) fn archive_directory(&self) -> PathBuf {
        self.cache
            .join(ARCHIVE_CACHE_DIRECTORY)
            .join(checksum(self.source.as_bytes()))
    }

    pub fn download(&self, name: &str, version: &str, expected_checksum: &str) -> Result<PathBuf> {
        validate_name(name)?;
        validate_name(version)?;
        let expected_checksum = expected_checksum.to_ascii_lowercase();
        validate_checksum(&expected_checksum)?;
        let directory = self.archive_directory();
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
