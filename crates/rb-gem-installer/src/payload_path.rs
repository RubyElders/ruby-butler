use anyhow::{Result, ensure};
use std::path::{Component, Path, PathBuf};

pub(crate) fn normalize(path: &Path) -> Result<PathBuf> {
    ensure!(
        portable(path),
        "Nonportable payload path: {}",
        path.display()
    );
    relative(path)
}

pub(crate) fn relative(path: &Path) -> Result<PathBuf> {
    ensure!(
        path.components()
            .all(|part| matches!(part, Component::Normal(_) | Component::CurDir)),
        "Unsafe payload path: {}",
        path.display()
    );
    let normalized: PathBuf = path
        .components()
        .filter_map(|part| match part {
            Component::Normal(name) => Some(name),
            _ => None,
        })
        .collect();
    ensure!(!normalized.as_os_str().is_empty(), "Empty payload path");
    Ok(normalized)
}

pub(crate) fn link_target(path: &Path, target: &Path) -> Result<PathBuf> {
    ensure!(portable(target), "Nonportable link target");
    ensure!(!target.as_os_str().is_empty(), "Empty link target");
    let mut resolved = path.parent().unwrap_or(Path::new("")).to_path_buf();
    for part in target.components() {
        match part {
            Component::Normal(name) => resolved.push(name),
            Component::CurDir => {}
            Component::ParentDir => ensure!(resolved.pop(), "Link target escapes installation"),
            _ => anyhow::bail!("Absolute link target"),
        }
    }
    ensure!(
        !resolved.as_os_str().is_empty(),
        "Link target is installation root"
    );
    Ok(resolved)
}

fn portable(path: &Path) -> bool {
    !path
        .as_os_str()
        .as_encoded_bytes()
        .iter()
        .any(|byte| matches!(byte, b'\\' | b':'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filesystem_paths_accept_native_separators() {
        let path = PathBuf::from(".rb-native").join("architecture").join("abi");
        assert_eq!(relative(&path).unwrap(), path);
        assert!(relative(Path::new("../outside")).is_err());
        assert!(relative(&std::env::temp_dir()).is_err());
    }

    #[test]
    fn archive_paths_reject_nonportable_separators() {
        assert!(normalize(Path::new("lib\\nested.rb")).is_err());
    }
}
