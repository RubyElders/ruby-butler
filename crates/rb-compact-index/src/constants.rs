use std::time::Duration;

pub(crate) const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
pub(crate) const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);

pub(crate) const ARCHIVE_BUFFER_BYTES: usize = 64 * 1024;
pub(crate) const MAX_ARCHIVE_BYTES: u64 = 1024 * 1024 * 1024;
pub(crate) const MAX_COMPACT_INFO_BYTES: u64 = 32 * 1024 * 1024;
pub(crate) const MAX_COMPRESSED_INDEX_BYTES: u64 = 16 * 1024 * 1024;
pub(crate) const MAX_DECOMPRESSED_INDEX_BYTES: u64 = 32 * 1024 * 1024;
pub(crate) const MAX_GEM_METADATA_BYTES: u64 = 16 * 1024 * 1024;
pub(crate) const MAX_FALLBACK_ARCHIVE_BYTES: u64 = 256 * 1024 * 1024;

pub(crate) const INDEX_CACHE_DIRECTORY: &str = "index";
pub(crate) const ARCHIVE_CACHE_DIRECTORY: &str = "gems";
pub(crate) const FULL_INFO_CACHE_SUFFIX: &str = ".full-platforms.json";

pub(crate) const INFO_PATH_PREFIX: &str = "info/";
pub(crate) const ARCHIVE_PATH_PREFIX: &str = "gems/";
pub(crate) const VERSIONS_PATH: &str = "versions";
pub(crate) const RELEASE_INDEX_PATH: &str = "specs.4.8.gz";
pub(crate) const PRERELEASE_INDEX_PATH: &str = "prerelease_specs.4.8.gz";
pub(crate) const GEM_METADATA_PATH: &str = "metadata.gz";
