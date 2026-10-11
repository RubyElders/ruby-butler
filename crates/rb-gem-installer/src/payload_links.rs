use crate::{constants::MAX_LINK_DEPTH, payload_path};
use anyhow::{Context, Result, ensure};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    fs,
    path::{Component, Path, PathBuf},
};

pub(crate) struct PayloadLinks(BTreeMap<PathBuf, (PathBuf, PathBuf)>);

impl PayloadLinks {
    pub(crate) fn new() -> Self {
        Self(BTreeMap::new())
    }

    pub(crate) fn add(&mut self, path: PathBuf, target: PathBuf) -> Result<()> {
        let resolved = payload_path::link_target(&path, &target)?;
        self.0.insert(path, (target, resolved));
        Ok(())
    }

    fn resolve(&self, target: &Path) -> Result<PathBuf> {
        let mut pending: VecDeque<_> = target
            .components()
            .filter_map(|part| match part {
                Component::Normal(name) => Some(name.to_os_string()),
                _ => None,
            })
            .collect();
        let mut resolved = PathBuf::new();
        let mut followed = BTreeSet::new();
        while let Some(part) = pending.pop_front() {
            resolved.push(part);
            if let Some((_, target)) = self.0.get(&resolved) {
                ensure!(followed.insert(resolved.clone()), "Cyclic payload symlink");
                let mut replacement: VecDeque<_> = target
                    .components()
                    .filter_map(|part| match part {
                        Component::Normal(name) => Some(name.to_os_string()),
                        _ => None,
                    })
                    .collect();
                ensure!(
                    followed.len() <= MAX_LINK_DEPTH,
                    "Payload symlink chain exceeds depth limit"
                );
                replacement.append(&mut pending);
                pending = replacement;
                resolved.clear();
            }
        }
        Ok(resolved)
    }

    pub(crate) fn create(self, root: &Path) -> Result<()> {
        let mut directories = BTreeMap::new();
        for (link, (_, target)) in &self.0 {
            for parent in link.ancestors().skip(1) {
                ensure!(
                    !self.0.contains_key(parent),
                    "Payload symlink cannot be a link parent"
                );
            }
            let resolved = self.resolve(target)?;
            let metadata = fs::symlink_metadata(root.join(&resolved))
                .with_context(|| format!("Missing link target: {}", resolved.display()))?;
            ensure!(
                metadata.is_file() || metadata.is_dir(),
                "Link target must be a payload file or directory"
            );
            directories.insert(link.clone(), metadata.is_dir());
        }
        for (path, (target, _)) in self.0 {
            let destination = root.join(&path);
            fs::create_dir_all(destination.parent().context("Missing link parent")?)?;
            #[cfg(unix)]
            std::os::unix::fs::symlink(target, destination)?;
            #[cfg(windows)]
            if directories[&path] {
                std::os::windows::fs::symlink_dir(target, destination)?;
            } else {
                std::os::windows::fs::symlink_file(target, destination)?;
            }
            #[cfg(not(any(unix, windows)))]
            anyhow::bail!("Symlinks are not supported on this platform");
        }
        Ok(())
    }
}
