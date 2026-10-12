use anyhow::Result;
use flate2::{Compression, write::GzEncoder};
use std::{io::Write, path::Path};

pub(crate) fn archive(
    root: &Path,
    metadata: &str,
    files: &[std::path::PathBuf],
) -> Result<Vec<u8>> {
    let mut payload = tar::Builder::new(GzEncoder::new(Vec::new(), Compression::default()));
    payload.follow_symlinks(false);
    for path in files {
        payload.append_path_with_name(root.join(path), path)?;
    }
    let payload = payload.into_inner()?.finish()?;
    let mut metadata_gzip = GzEncoder::new(Vec::new(), Compression::default());
    metadata_gzip.write_all(metadata.as_bytes())?;
    let mut outer = tar::Builder::new(Vec::new());
    for (name, bytes) in [
        (
            crate::constants::GEM_METADATA_MEMBER,
            metadata_gzip.finish()?,
        ),
        (crate::constants::GEM_PAYLOAD_MEMBER, payload),
    ] {
        let mut header = tar::Header::new_gnu();
        header.set_size(bytes.len() as u64);
        header.set_mode(0o644);
        header.set_cksum();
        outer.append_data(&mut header, name, bytes.as_slice())?;
    }
    Ok(outer.into_inner()?)
}
