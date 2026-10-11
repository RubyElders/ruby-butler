use crate::{
    constants::{DATA_MEMBER, MAX_FILES, MAX_PAYLOAD_BYTES, READ_BUFFER_BYTES},
    payload_links::PayloadLinks,
    payload_path,
};
use anyhow::{Context, Result, ensure};
use flate2::read::GzDecoder;
use std::{
    collections::BTreeMap,
    fs::{File, OpenOptions},
    io::{self, Read},
    path::Path,
};

pub(crate) fn extract(reader: impl Read, destination: &Path) -> Result<()> {
    let mut outer = tar::Archive::new(reader);
    for member in outer.entries()? {
        let member = member?;
        if member.path()?.as_ref() != Path::new(DATA_MEMBER) {
            continue;
        }
        let mut decoder = GzDecoder::new(member).take(MAX_PAYLOAD_BYTES + 1);
        let mut links = PayloadLinks::new();
        {
            let mut payload = tar::Archive::new(&mut decoder);
            let mut paths = BTreeMap::new();
            let mut total = 0_u64;
            for (index, entry) in payload.entries()?.enumerate() {
                ensure!(index < MAX_FILES, "Payload exceeds file count limit");
                let mut entry = entry?;
                let path = payload_path::normalize(&entry.path()?)?;
                let kind = entry.header().entry_type();
                let mode = entry.header().mode()? & 0o777;
                ensure!(
                    kind.is_file() || kind.is_dir() || kind.is_symlink(),
                    "Unsupported payload entry: {}",
                    path.display()
                );
                total = total
                    .checked_add(entry.size())
                    .context("Payload size overflow")?;
                ensure!(total <= MAX_PAYLOAD_BYTES, "Payload exceeds size limit");
                let target = destination.join(&path);
                if let Some(previous) = paths.insert(path.clone(), (kind, mode)) {
                    ensure!(
                        previous == (kind, mode) && kind.is_file(),
                        "Conflicting payload entry: {}",
                        path.display()
                    );
                    identical(File::open(&target)?, &mut entry)?;
                    continue;
                }
                if kind.is_dir() {
                    std::fs::create_dir_all(&target)?;
                } else if kind.is_symlink() {
                    let link = entry
                        .link_name()?
                        .context("Missing symlink target")?
                        .into_owned();
                    links.add(path, link)?;
                } else {
                    std::fs::create_dir_all(target.parent().context("Missing payload parent")?)?;
                    let mut options = OpenOptions::new();
                    options.write(true).create_new(true);
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::OpenOptionsExt;
                        options.mode(mode);
                    }
                    let mut file = options.open(&target)?;
                    io::copy(&mut entry, &mut file)?;
                }
            }
        }
        io::copy(&mut decoder, &mut io::sink())?;
        ensure!(
            decoder.limit() > 0,
            "Payload exceeds decompressed size limit"
        );
        links.create(destination)?;
        return Ok(());
    }
    anyhow::bail!("Gem is missing data.tar.gz")
}

fn identical(mut existing: File, mut entry: impl Read) -> Result<()> {
    let mut old = [0; READ_BUFFER_BYTES];
    let mut new = [0; READ_BUFFER_BYTES];
    loop {
        let size = existing.read(&mut old)?;
        if size == 0 {
            ensure!(
                entry.read(&mut new[..1])? == 0,
                "Conflicting duplicate payload content"
            );
            return Ok(());
        }
        entry
            .read_exact(&mut new[..size])
            .context("Conflicting duplicate payload size")?;
        ensure!(
            old[..size] == new[..size],
            "Conflicting duplicate payload content"
        );
    }
}
