use crate::{Event, Installer, Phase, Release};
use anyhow::{Context, Result, bail, ensure};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

const MAX_ARCHIVE_SIZE: u64 = 512 * 1024 * 1024;
const DOWNLOAD_BUFFER_SIZE: usize = 64 * 1024;
const CHECKSUM_LIMIT: u64 = 1024 * 1024;

impl Installer {
    pub(crate) fn checksum(&self, release: &Release, asset: &crate::Asset) -> Result<String> {
        if let Some(digest) = &asset.digest {
            return valid_digest(
                digest
                    .strip_prefix("sha256:")
                    .context("Unsupported Ruby archive checksum algorithm")?,
            );
        }
        let sums = release
            .assets
            .iter()
            .find(|asset| asset.name == "SHA256SUMS")
            .context("Ruby release provides no SHA-256 checksum")?;
        let response = self
            .client
            .get(&sums.browser_download_url)
            .send()?
            .error_for_status()?;
        let bytes = crate::catalog::read_limited(response, CHECKSUM_LIMIT)?;
        let text = std::str::from_utf8(&bytes)?;
        for line in text.lines() {
            if let Some((digest, name)) = line.split_once(char::is_whitespace)
                && name.trim().trim_start_matches('*') == asset.name
            {
                return valid_digest(digest);
            }
        }
        bail!("No SHA-256 checksum for {}", asset.name)
    }

    pub(crate) fn download(
        &self,
        asset: &crate::Asset,
        digest: &str,
        report: &mut dyn FnMut(Event),
    ) -> Result<PathBuf> {
        ensure!(
            asset.size > 0 && asset.size <= MAX_ARCHIVE_SIZE,
            "Invalid Ruby archive size"
        );
        let archives = self.cache.join("archives");
        fs::create_dir_all(&archives)?;
        let path = archives.join(format!("{digest}.tar.gz"));
        if path.is_file() {
            report(Event::Phase(Phase::Verifying));
            if verify(&path, asset.size, digest)? {
                return Ok(path);
            }
        }
        report(Event::Phase(Phase::Downloading));
        let mut response = self
            .client
            .get(&asset.browser_download_url)
            .send()?
            .error_for_status()?;
        let mut file = tempfile::NamedTempFile::new_in(&archives)?;
        let mut hash = Sha256::new();
        let mut received = 0u64;
        let mut buffer = [0u8; DOWNLOAD_BUFFER_SIZE];
        loop {
            let count = response.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            received += count as u64;
            ensure!(
                received <= asset.size,
                "Ruby archive exceeds its advertised size"
            );
            file.write_all(&buffer[..count])?;
            hash.update(&buffer[..count]);
            report(Event::Download {
                received,
                total: asset.size,
            });
        }
        report(Event::Phase(Phase::Verifying));
        ensure!(
            received == asset.size,
            "Ruby archive download is incomplete"
        );
        ensure!(
            format!("{:x}", hash.finalize()) == digest,
            "Ruby archive SHA-256 mismatch"
        );
        file.as_file().sync_all()?;
        file.persist(&path)?;
        Ok(path)
    }
}

fn valid_digest(value: &str) -> Result<String> {
    ensure!(
        value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "Invalid SHA-256 checksum"
    );
    Ok(value.to_ascii_lowercase())
}

fn verify(path: &Path, size: u64, digest: &str) -> Result<bool> {
    if fs::metadata(path)?.len() != size {
        return Ok(false);
    }
    let mut file = File::open(path)?;
    let mut hash = Sha256::new();
    std::io::copy(&mut file, &mut hash)?;
    Ok(format!("{:x}", hash.finalize()) == digest)
}
