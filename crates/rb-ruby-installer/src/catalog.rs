use crate::{Installer, Platform, Request};
use anyhow::{Context, Result, bail};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::Read,
    time::{Duration, SystemTime},
};

pub const CATALOG_URL: &str = "https://api.github.com/repos/spinel-coop/rv-ruby/releases/latest";
const CATALOG_LIMIT: u64 = 8 * 1024 * 1024;
const CATALOG_TTL: Duration = Duration::from_secs(3600);

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Asset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
    pub digest: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Release {
    pub tag_name: String,
    pub assets: Vec<Asset>,
}

#[derive(Clone, Debug)]
pub struct Distribution {
    pub version: Version,
    pub release: String,
    pub asset: Asset,
}

impl Release {
    pub fn distributions(&self, platform: Platform) -> Vec<Distribution> {
        let mut distributions: Vec<_> = self
            .assets
            .iter()
            .filter_map(|asset| {
                let version = asset
                    .name
                    .strip_prefix("ruby-")?
                    .strip_suffix(platform.suffix())?
                    .parse::<Version>()
                    .ok()?;
                if !version.pre.is_empty() || !version.build.is_empty() {
                    return None;
                }
                Some(Distribution {
                    version,
                    release: self.tag_name.clone(),
                    asset: asset.clone(),
                })
            })
            .collect();
        distributions.sort_by(|a, b| b.version.cmp(&a.version));
        distributions
    }

    pub fn select(&self, request: &Request, platform: Platform) -> Result<Distribution> {
        self.distributions(platform)
            .into_iter()
            .find(|distribution| request.matches(&distribution.version))
            .with_context(|| {
                format!("No prebuilt Ruby matching {request} is available for {platform:?}")
            })
    }
}

#[derive(Serialize, Deserialize)]
struct CachedRelease {
    url: String,
    etag: Option<String>,
    fetched: SystemTime,
    release: Release,
}

impl Installer {
    fn catalog_request(&self) -> reqwest::blocking::RequestBuilder {
        let request = self.client.get(&self.catalog_url);
        if self.catalog_url == CATALOG_URL
            && let Some(token) = ["GH_TOKEN", "GITHUB_TOKEN"]
                .iter()
                .filter_map(|name| self.environment.get(std::ffi::OsStr::new(name)))
                .filter_map(|value| value.to_str())
                .find(|value| !value.is_empty())
        {
            return request.bearer_auth(token);
        }
        request
    }

    pub fn catalog(&self) -> Result<Release> {
        fs::create_dir_all(&self.cache)?;
        let path = self.cache.join("catalog.json");
        let cached = fs::read(&path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<CachedRelease>(&bytes).ok())
            .filter(|cached| cached.url == self.catalog_url);
        if let Some(cached) = &cached
            && cached.fetched.elapsed().is_ok_and(|age| age < CATALOG_TTL)
        {
            return Ok(cached.release.clone());
        }
        let mut request = self.catalog_request();
        if let Some(etag) = cached.as_ref().and_then(|cached| cached.etag.as_ref()) {
            request = request.header(reqwest::header::IF_NONE_MATCH, etag);
        }
        let response = request
            .send()
            .context("Unable to fetch the Ruby release catalog")?;
        let (etag, release) = if response.status() == reqwest::StatusCode::NOT_MODIFIED {
            let cached = cached.context("Catalog returned 304 without a cached release")?;
            (cached.etag, cached.release)
        } else {
            let response = response.error_for_status()?;
            let etag = response
                .headers()
                .get(reqwest::header::ETAG)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            let bytes = read_limited(response, CATALOG_LIMIT)?;
            (
                etag,
                serde_json::from_slice(&bytes).context("Invalid Ruby release catalog")?,
            )
        };
        let cached = CachedRelease {
            url: self.catalog_url.clone(),
            etag,
            fetched: SystemTime::now(),
            release: release.clone(),
        };
        let mut file = tempfile::NamedTempFile::new_in(&self.cache)?;
        serde_json::to_writer(file.as_file_mut(), &cached)?;
        file.persist(path)?;
        Ok(release)
    }
}

pub(crate) fn read_limited(reader: impl Read, limit: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader.take(limit + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        bail!("Release metadata exceeds the size limit");
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::header::AUTHORIZATION;
    use std::collections::BTreeMap;

    fn installer(environment: &[(&str, &str)]) -> Installer {
        Installer::with_inputs(
            std::env::temp_dir(),
            std::env::temp_dir(),
            environment
                .iter()
                .map(|(name, value)| ((*name).into(), (*value).into()))
                .collect::<BTreeMap<_, _>>(),
        )
        .unwrap()
    }

    #[test]
    fn catalog_credentials_apply_only_to_the_official_api_request() {
        let source = installer(&[("GH_TOKEN", "catalog-canary")]);
        let request = source.catalog_request().build().unwrap();
        assert_eq!(request.headers()[AUTHORIZATION], "Bearer catalog-canary");
        assert!(request.headers()[AUTHORIZATION].is_sensitive());
        assert!(
            source
                .client
                .get("https://github.com/spinel-coop/rv-ruby/releases/download/test/archive.tar.gz")
                .build()
                .unwrap()
                .headers()
                .get(AUTHORIZATION)
                .is_none()
        );
        for url in [
            "https://other.example/catalog.json",
            "http://api.github.com/repos/spinel-coop/rv-ruby/releases/latest",
        ] {
            let custom = installer(&[("GH_TOKEN", "catalog-canary")]).with_catalog_url(url);
            assert!(
                custom
                    .catalog_request()
                    .build()
                    .unwrap()
                    .headers()
                    .get(AUTHORIZATION)
                    .is_none()
            );
        }
    }

    #[test]
    fn catalog_token_precedence_ignores_empty_values() {
        for (environment, expected) in [
            (
                vec![("GITHUB_TOKEN", "fallback-canary")],
                "Bearer fallback-canary",
            ),
            (
                vec![("GH_TOKEN", ""), ("GITHUB_TOKEN", "fallback-canary")],
                "Bearer fallback-canary",
            ),
            (
                vec![
                    ("GH_TOKEN", "preferred-canary"),
                    ("GITHUB_TOKEN", "fallback-canary"),
                ],
                "Bearer preferred-canary",
            ),
        ] {
            let request = installer(&environment).catalog_request().build().unwrap();
            assert_eq!(request.headers()[AUTHORIZATION], expected);
        }
        assert!(
            installer(&[])
                .catalog_request()
                .build()
                .unwrap()
                .headers()
                .get(AUTHORIZATION)
                .is_none()
        );
    }
}
