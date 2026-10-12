use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub(crate) fn run(command: &mut Command) -> Result<String> {
    let output = command
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .context("Running source command")?;
    ensure!(
        output.status.success(),
        "Source command failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

pub(crate) fn checkout(
    root: &Path,
    remote: &str,
    options: &std::collections::BTreeMap<String, String>,
    cache: &Path,
    pinned: Option<&str>,
    offline: bool,
) -> Result<(String, PathBuf)> {
    ensure!(
        !remote.is_empty() && !remote.starts_with('-') && !remote.chars().any(char::is_control),
        "Invalid Git remote"
    );
    let location = if Path::new(remote).is_absolute() || root.join(remote).exists() {
        dunce::canonicalize(root.join(remote))
            .unwrap_or_else(|_| root.join(remote))
            .to_string_lossy()
            .into_owned()
    } else {
        remote.to_owned()
    };
    if !Path::new(&location).is_absolute() {
        if let Ok(url) = url::Url::parse(&location) {
            ensure!(
                matches!(url.scheme(), "https" | "http" | "ssh" | "git" | "file"),
                "Unsupported Git protocol"
            );
            ensure!(
                url.password().is_none()
                    && (!matches!(url.scheme(), "https" | "http") || url.username().is_empty()),
                "Git URL credentials are not supported"
            );
        } else {
            ensure!(
                location.contains(':') && !location.contains("::"),
                "Invalid Git remote"
            );
        }
    }
    let id = format!("{:x}", Sha256::digest(location.as_bytes()));
    let directory = cache.join(crate::constants::GIT_CACHE_DIRECTORY).join(id);
    fs::create_dir_all(&directory)?;
    let guard = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(directory.join(crate::constants::CACHE_LOCK_FILE))?;
    guard.lock()?;
    let repository = directory.join(crate::constants::GIT_REPOSITORY_DIRECTORY);
    if !repository.exists() {
        ensure!(!offline, "Git repository is not cached for offline use");
        let temporary = tempfile::tempdir_in(&directory)?;
        let target = temporary
            .path()
            .join(crate::constants::GIT_REPOSITORY_DIRECTORY);
        run(Command::new("git")
            .args(["clone", "--bare", "--"])
            .arg(&location)
            .arg(&target))?;
        fs::rename(target, &repository)?;
    }
    let git = |arguments: &[&str]| -> Result<String> {
        run(Command::new("git")
            .arg("--git-dir")
            .arg(&repository)
            .args(arguments))
    };
    if pinned.is_none() && !offline {
        git(&[
            "fetch",
            "origin",
            "+refs/heads/*:refs/heads/*",
            "+refs/tags/*:refs/tags/*",
        ])?;
    }
    let selector = pinned
        .map(str::to_owned)
        .or_else(|| options.get("ref").cloned())
        .or_else(|| {
            options
                .get("branch")
                .map(|branch| format!("refs/heads/{branch}"))
        })
        .or_else(|| options.get("tag").map(|tag| format!("refs/tags/{tag}")))
        .unwrap_or_else(|| crate::constants::DEFAULT_GIT_REFERENCE.into());
    ensure!(
        !selector.starts_with('-') && !selector.chars().any(char::is_control),
        "Invalid Git selector"
    );
    if selector == crate::constants::DEFAULT_GIT_REFERENCE {
        if !offline {
            git(&["fetch", "origin", "HEAD"])?;
            git(&[
                "update-ref",
                crate::constants::DEFAULT_GIT_REFERENCE,
                "FETCH_HEAD",
            ])?;
        } else if git(&[
            "rev-parse",
            "--verify",
            crate::constants::DEFAULT_GIT_REFERENCE,
        ])
        .is_err()
        {
            git(&[
                "update-ref",
                crate::constants::DEFAULT_GIT_REFERENCE,
                "HEAD",
            ])?;
        }
    }
    let resolve = || git(&["rev-parse", "--verify", &format!("{selector}^{{commit}}")]);
    let revision = match resolve() {
        Ok(revision) => revision,
        Err(error) if offline => return Err(error.context("Git commit is not cached")),
        Err(_) => {
            git(&["fetch", "origin", &selector])?;
            resolve()?
        }
    };
    ensure!(
        crate::constants::GIT_OBJECT_ID_LENGTHS.contains(&revision.len())
            && revision.bytes().all(|byte| byte.is_ascii_hexdigit()),
        "Invalid Git object ID"
    );
    git(&["update-ref", &format!("refs/rb/{revision}"), &revision])?;
    let key = format!(
        "{revision}-{}",
        options.get("submodules").is_some_and(|v| v == "true")
    );
    let checkout = directory
        .join(crate::constants::GIT_CHECKOUTS_DIRECTORY)
        .join(key);
    if !checkout.exists() {
        fs::create_dir_all(checkout.parent().unwrap())?;
        let temporary = tempfile::tempdir_in(checkout.parent().unwrap())?;
        let target = temporary.path().join("checkout");
        run(Command::new("git")
            .args(["clone", "--shared", "--no-checkout", "--"])
            .arg(&repository)
            .arg(&target))?;
        run(Command::new("git")
            .arg("-C")
            .arg(&target)
            .args(["checkout", "--detach", &revision]))?;
        run(Command::new("git")
            .arg("-C")
            .arg(&target)
            .args(["remote", "set-url", "origin", &location]))?;
        if options.get("submodules").is_some_and(|v| v == "true") {
            ensure!(!offline, "Submodule checkout is not cached for offline use");
            run(Command::new("git").arg("-C").arg(&target).args([
                "submodule",
                "update",
                "--init",
                "--recursive",
            ]))?;
        }
        fs::rename(target, &checkout)?;
    }
    let head = run(Command::new("git")
        .arg("-C")
        .arg(&checkout)
        .args(["rev-parse", "HEAD"]))?;
    let changes = run(Command::new("git").arg("-C").arg(&checkout).args([
        "status",
        "--porcelain",
        "--untracked-files=all",
    ]))?;
    ensure!(
        head == revision && changes.is_empty(),
        "Cached Git checkout has been modified"
    );
    Ok((revision, checkout))
}
