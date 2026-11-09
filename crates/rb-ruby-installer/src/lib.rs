#![doc = include_str!("../README.md")]

mod archive;
mod catalog;
mod download;
mod event;
mod install;
mod platform;
mod request;
mod validation;

use anyhow::Result;
pub use catalog::{Asset, CATALOG_URL, Distribution, Release};
pub use event::{Event, Phase};
pub use install::Installation;
pub use platform::Platform;
pub use request::Request;
use std::{collections::BTreeMap, ffi::OsString, path::PathBuf, time::Duration};
pub use validation::complete;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(300);

pub struct Installer {
    pub(crate) cache: PathBuf,
    pub(crate) directory: PathBuf,
    pub(crate) environment: BTreeMap<OsString, OsString>,
    pub(crate) catalog_url: String,
    pub(crate) client: reqwest::blocking::Client,
}

impl Installer {
    pub fn new(cache: impl Into<PathBuf>) -> Result<Self> {
        Self::with_inputs(
            cache,
            std::env::current_dir()?,
            std::env::vars_os().collect(),
        )
    }

    pub fn with_inputs(
        cache: impl Into<PathBuf>,
        directory: PathBuf,
        environment: BTreeMap<OsString, OsString>,
    ) -> Result<Self> {
        Ok(Self {
            directory,
            environment,
            cache: cache.into(),
            catalog_url: CATALOG_URL.into(),
            client: reqwest::blocking::Client::builder()
                .user_agent(concat!("ruby-butler/", env!("CARGO_PKG_VERSION")))
                .connect_timeout(CONNECT_TIMEOUT)
                .timeout(REQUEST_TIMEOUT)
                .build()?,
        })
    }

    pub fn with_catalog_url(mut self, url: impl Into<String>) -> Self {
        self.catalog_url = url.into();
        self
    }
}
