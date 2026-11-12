use crate::cache::limited_read;
use anyhow::{Context, Result, ensure};
use base64::Engine;
use sha2::Digest;

pub(crate) fn read_index(response: reqwest::blocking::Response, limit: u64) -> Result<Vec<u8>> {
    read_index_with_prefix(response, limit, None)
}

pub(crate) fn read_index_with_prefix(
    response: reqwest::blocking::Response,
    limit: u64,
    prefix: Option<&[u8]>,
) -> Result<Vec<u8>> {
    let digest = response
        .headers()
        .get("repr-digest")
        .and_then(|v| v.to_str().ok())
        .map(str::to_owned);
    let range = response
        .headers()
        .get(reqwest::header::CONTENT_RANGE)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned);
    let partial = response.status() == reqwest::StatusCode::PARTIAL_CONTENT;
    let mut bytes = limited_read(response, limit)?;
    if partial {
        let prefix = prefix.context("Partial index without cached prefix")?;
        let range = range
            .as_deref()
            .and_then(|value| value.strip_prefix("bytes "))
            .context("Missing index content range")?;
        let (bounds, total) = range
            .split_once('/')
            .context("Invalid index content range")?;
        let (start, end) = bounds
            .split_once('-')
            .context("Invalid index content range")?;
        let (start, end, total) = (
            start.parse::<usize>()?,
            end.parse::<usize>()?,
            total.parse::<usize>()?,
        );
        ensure!(
            start == prefix.len().saturating_sub(1)
                && !prefix.is_empty()
                && end >= start
                && total.checked_sub(1) == Some(end)
                && end.checked_sub(start).and_then(|size| size.checked_add(1)) == Some(bytes.len()),
            "Invalid index content range"
        );
        ensure!(bytes.first() == prefix.last(), "Index prefix changed");
        ensure!(total as u64 <= limit, "Index exceeds size limit");
        let mut combined = prefix[..start].to_vec();
        combined.append(&mut bytes);
        bytes = combined;
    }
    if let Some(digest) = digest
        && let Some(value) = digest
            .split(',')
            .find_map(|part| part.trim().strip_prefix("sha-256="))
    {
        let expected =
            base64::engine::general_purpose::STANDARD.decode(value.trim_matches([':', '"']))?;
        ensure!(
            sha2::Sha256::digest(&bytes).as_slice() == expected,
            "Index digest mismatch"
        );
    }
    Ok(bytes)
}
