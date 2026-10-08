pub(crate) const MAX_ARCHIVE_BYTES: u64 = 1024 * 1024 * 1024;
pub(crate) const MAX_METADATA_BYTES: u64 = 16 * 1024 * 1024;
pub(crate) const MAX_CHECKSUM_BYTES: u64 = 1024 * 1024;
pub(crate) const READ_BUFFER_BYTES: usize = 8 * 1024;
pub(crate) const MAX_SIGNATURE_BYTES: u64 = 1024 * 1024;
pub(crate) const METADATA_MEMBER: &str = "metadata.gz";
pub(crate) const DATA_MEMBER: &str = "data.tar.gz";
pub(crate) const CHECKSUM_MEMBER: &str = "checksums.yaml.gz";
