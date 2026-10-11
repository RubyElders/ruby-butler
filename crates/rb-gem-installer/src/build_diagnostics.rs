use crate::constants::MAX_BUILD_OUTPUT_BYTES;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

pub(crate) fn mkmf(directory: Option<&Path>) -> Option<String> {
    let mut file = File::open(directory?.join("mkmf.log")).ok()?;
    let bytes = file.metadata().ok()?.len();
    file.seek(SeekFrom::Start(
        bytes.saturating_sub(MAX_BUILD_OUTPUT_BYTES as u64),
    ))
    .ok()?;
    let mut tail = Vec::new();
    file.take(MAX_BUILD_OUTPUT_BYTES as u64)
        .read_to_end(&mut tail)
        .ok()?;
    Some(String::from_utf8_lossy(&tail).into_owned())
}
