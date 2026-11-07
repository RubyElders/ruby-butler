use anyhow::{Context, Result, bail, ensure};
use flate2::read::GzDecoder;
use std::{
    fs::File,
    path::{Component, Path, PathBuf},
};

const MAX_UNPACKED_SIZE: u64 = 2 * 1024 * 1024 * 1024;
const MAX_ENTRIES: usize = 100_000;
const ROOT_COMPONENTS: usize = 2;

pub(crate) fn extract(archive: &Path, stage: &Path) -> Result<PathBuf> {
    let mut archive = tar::Archive::new(GzDecoder::new(File::open(archive)?));
    let mut root = None;
    let mut size = 0u64;
    for (index, entry) in archive.entries()?.enumerate() {
        ensure!(index < MAX_ENTRIES, "Ruby archive has too many entries");
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        let parts: Vec<_> = path.components().collect();
        ensure!(
            parts
                .iter()
                .all(|part| matches!(part, Component::Normal(_))),
            "Unsafe Ruby archive path: {}",
            path.display()
        );
        if parts.len() < ROOT_COMPONENTS {
            ensure!(
                entry.header().entry_type().is_dir(),
                "Invalid Ruby archive root"
            );
            continue;
        }
        let prefix: PathBuf = parts[..ROOT_COMPONENTS].iter().collect();
        if let Some(root) = &root {
            ensure!(root == &prefix, "Ruby archive has inconsistent roots");
        } else {
            root = Some(prefix.clone());
        }
        let kind = entry.header().entry_type();
        ensure!(
            kind.is_file() || kind.is_dir() || kind.is_symlink() || kind.is_hard_link(),
            "Unsupported Ruby archive entry"
        );
        size = size
            .checked_add(entry.size())
            .context("Ruby archive size overflow")?;
        ensure!(
            size <= MAX_UNPACKED_SIZE,
            "Ruby archive exceeds the unpacked size limit"
        );
        if let Some(target) = entry.link_name()? {
            let base = if kind.is_symlink() {
                path.parent().unwrap()
            } else {
                Path::new("")
            };
            let mut resolved = base.to_path_buf();
            for component in target.components() {
                match component {
                    Component::Normal(part) => resolved.push(part),
                    Component::CurDir => (),
                    Component::ParentDir => {
                        ensure!(resolved.pop(), "Unsafe Ruby archive link");
                    }
                    _ => bail!("Unsafe Ruby archive link"),
                }
            }
            ensure!(
                resolved.starts_with(&prefix),
                "Ruby archive link escapes the installation"
            );
        }
        ensure!(
            entry.unpack_in(stage)?,
            "Ruby archive entry escapes staging"
        );
    }
    Ok(stage.join(root.context("Ruby archive is empty")?))
}
