use crate::{
    InstalledBundle, PrepareOptions, PreparedBundle, ProjectInput,
    constants::{MANIFEST_FILE, PREPARED_STATE, PROJECT_LOCK, STATE_DIRECTORY},
};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs::{self, File},
    path::{Path, PathBuf},
};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    configuration: String,
    bundle_fingerprint: String,
    anchor: PathBuf,
    inputs: BTreeSet<PathBuf>,
    environment: Option<BTreeSet<String>>,
    input_hash: String,
    metadata_hash: String,
    installed: InstalledBundle,
    lockfile: PathBuf,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    state: State,
    checksum: String,
}

#[derive(Clone)]
pub(crate) struct Candidate {
    anchor: PathBuf,
    before: String,
}

pub(crate) struct ProjectLock(File);

impl std::ops::Deref for ProjectLock {
    type Target = File;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Drop for ProjectLock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

pub(crate) fn lock(root: &Path) -> Result<ProjectLock> {
    let state = root.join(STATE_DIRECTORY);
    fs::create_dir_all(&state)?;
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(state.join(PROJECT_LOCK))?;
    file.lock()?;
    Ok(ProjectLock(file))
}

/// Returns a successful installation when its dependency inputs and metadata still match.
pub fn prepared(options: &PrepareOptions) -> Result<Option<InstalledBundle>> {
    if !options.record_state {
        return Ok(None);
    }
    let filename = options.project.join(STATE_DIRECTORY).join(PREPARED_STATE);
    if !filename.is_file() {
        return Ok(None);
    }
    let lock_path = options.project.join(STATE_DIRECTORY).join(PROJECT_LOCK);
    let _lock = match File::open(lock_path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if _lock.try_lock_shared().is_err() {
        return Ok(None);
    }
    let _lock = ProjectLock(_lock);
    Ok(attempt(read(options, &filename)))
}

fn read(options: &PrepareOptions, filename: &Path) -> Result<InstalledBundle> {
    let envelope: Envelope = serde_json::from_slice(&fs::read(filename)?)?;
    let state = envelope.state;
    ensure!(
        checksum(&state)? == envelope.checksum,
        "Prepared state checksum changed"
    );
    ensure!(
        state.configuration == configuration(options)?,
        "Preparation options changed"
    );
    ensure!(
        std::fs::read_to_string(
            state
                .installed
                .bundle_path
                .join(crate::constants::BUNDLE_FINGERPRINT)
        )? == state.bundle_fingerprint,
        "Resolved bundle changed"
    );
    ensure!(state.anchor == anchor(options)?, "Project input changed");
    ensure!(
        state.input_hash
            == inputs(
                options,
                &state.anchor,
                &state.inputs,
                state.environment.as_ref()
            )?,
        "Preparation inputs changed"
    );
    ensure!(
        state.installed.gem_home.is_dir(),
        "Installation directory is missing"
    );
    ensure!(
        state.metadata_hash == metadata(options, &state.installed, &state.lockfile)?,
        "Dependency metadata changed"
    );
    let mut installed = state.installed;
    installed.reused += installed.installed;
    installed.installed = 0;
    Ok(installed)
}

impl Candidate {
    pub(crate) fn capture(options: &PrepareOptions) -> Result<Self> {
        let anchor = anchor(options)?;
        let before = inputs(options, &anchor, &BTreeSet::new(), Some(&BTreeSet::new()))?;
        Ok(Self { anchor, before })
    }

    pub(crate) fn observe(
        &self,
        options: &PrepareOptions,
        input: &ProjectInput,
        source_inputs: &BTreeSet<PathBuf>,
    ) -> Result<(BTreeSet<PathBuf>, String)> {
        ensure!(
            self.before
                == inputs(
                    options,
                    &self.anchor,
                    &BTreeSet::new(),
                    Some(&BTreeSet::new())
                )?,
            "Project changed during evaluation"
        );
        let mut paths: BTreeSet<_> = input.evaluation_inputs.iter().cloned().collect();
        paths.extend(source_inputs.iter().cloned());
        Ok((
            paths.clone(),
            inputs(
                options,
                &self.anchor,
                &paths,
                input.evaluation_environment.as_ref(),
            )?,
        ))
    }

    pub(crate) fn publish(
        self,
        options: &PrepareOptions,
        prepared: &PreparedBundle,
        paths: BTreeSet<PathBuf>,
        before: String,
    ) -> Result<()> {
        ensure!(
            self.before
                == inputs(
                    options,
                    &self.anchor,
                    &BTreeSet::new(),
                    Some(&BTreeSet::new())
                )?,
            "Project changed during preparation"
        );
        ensure!(
            before
                == inputs(
                    options,
                    &self.anchor,
                    &paths,
                    prepared.input.evaluation_environment.as_ref()
                )?,
            "Source inputs changed during preparation"
        );
        let state = State {
            configuration: configuration(options)?,
            bundle_fingerprint: prepared.fingerprint.clone(),
            input_hash: before,
            metadata_hash: metadata(options, &prepared.installed, &prepared.input.lockfile)?,
            lockfile: prepared.input.lockfile.clone(),
            installed: prepared.installed.clone(),
            anchor: self.anchor,
            inputs: paths,
            environment: prepared.input.evaluation_environment.clone(),
        };
        let checksum = checksum(&state)?;
        rb_gemfile::write_project(
            options.project.join(STATE_DIRECTORY).join(PREPARED_STATE),
            &serde_json::to_string(&Envelope { state, checksum })?,
        )?;
        Ok(())
    }
}

pub(crate) fn invalidate(root: &Path) -> Result<()> {
    match fs::remove_file(root.join(STATE_DIRECTORY).join(PREPARED_STATE)) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn anchor(options: &PrepareOptions) -> Result<PathBuf> {
    if let Some(file) = &options.project_file {
        return Ok(dunce::canonicalize(options.project.join(file))?);
    }
    if let Some(file) = &options.gemfile {
        return Ok(dunce::canonicalize(file)?);
    }
    ["rbproject.kdl", "rbproject.toml", "gems.rb", "Gemfile"]
        .iter()
        .map(|name| options.project.join(name))
        .find(|path| path.is_file())
        .map(dunce::canonicalize)
        .transpose()?
        .context("No project input found")
}

fn inputs(
    options: &PrepareOptions,
    anchor: &Path,
    paths: &BTreeSet<PathBuf>,
    environment: Option<&BTreeSet<String>>,
) -> Result<String> {
    let mut cache =
        crate::input::snapshot_cache(&options.project, PREPARED_STATE, options.gemfile.as_deref());
    if let Some(environment) = environment {
        let mut names = environment.clone();
        names.extend(
            crate::receipt::BUILD_ENVIRONMENT
                .iter()
                .map(|name| name.to_string()),
        );
        cache = cache.environment_inputs(names);
    }
    for name in [
        "gem.kdl",
        "gem.toml",
        "rbproject.kdl",
        "rbproject.toml",
        "gems.rb",
        "Gemfile",
        ".ruby-version",
    ] {
        cache = cache.input(options.project.join(name));
    }
    for path in paths {
        cache = cache.input(path);
    }
    Ok(cache.fingerprint(&options.evaluator(), anchor)?)
}

fn metadata(
    options: &PrepareOptions,
    installed: &InstalledBundle,
    lockfile: &Path,
) -> Result<String> {
    let gemfile = installed.gemfile.as_ref().context("No runtime Gemfile")?;
    let mut companion = gemfile.as_os_str().to_owned();
    companion.push(".lock");
    let paths = BTreeSet::from([
        gemfile.clone(),
        PathBuf::from(companion),
        lockfile.to_owned(),
        options.project.join("Gemfile.lock"),
        options.project.join("gems.locked"),
        options.project.join(STATE_DIRECTORY).join(MANIFEST_FILE),
    ]);
    let files = paths
        .into_iter()
        .map(|path| {
            let bytes = match fs::read(&path) {
                Ok(bytes) => Some(bytes),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => return Err(error.into()),
            };
            Ok((path, bytes))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(&files)?)))
}

fn configuration(options: &PrepareOptions) -> Result<String> {
    let data = format!(
        "prepared state {} {options:?}",
        crate::constants::PREPARED_STATE_VERSION
    );
    Ok(format!("{:x}", Sha256::digest(data.as_bytes())))
}

fn checksum(state: &State) -> Result<String> {
    Ok(format!("{:x}", Sha256::digest(serde_json::to_vec(state)?)))
}

pub(crate) fn attempt<T>(result: Result<T>) -> Option<T> {
    match result {
        Ok(value) => Some(value),
        Err(error) => {
            log::debug!("Prepared environment cache unavailable: {error:#}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn completed_preparation_releases_inherited_file_descriptors() {
        let root = tempfile::tempdir().unwrap();
        let writer = lock(root.path()).unwrap();
        let inherited = writer.try_clone().unwrap();
        drop(writer);
        let reader = File::open(root.path().join(STATE_DIRECTORY).join(PROJECT_LOCK)).unwrap();
        assert!(reader.try_lock_shared().is_ok());
        drop(inherited);
    }

    #[test]
    fn busy_preparation_is_a_cache_miss_without_waiting() {
        let root = tempfile::tempdir().unwrap();
        let writer = lock(root.path()).unwrap();
        fs::write(root.path().join(STATE_DIRECTORY).join(PREPARED_STATE), "{}").unwrap();
        let mut options =
            PrepareOptions::new(root.path(), "missing-ruby", root.path().join("cache"));
        options.record_state = true;
        let (sender, receiver) = std::sync::mpsc::channel();
        let reader = std::thread::spawn(move || sender.send(prepared(&options)).unwrap());
        let result = receiver.recv_timeout(std::time::Duration::from_secs(5));
        drop(writer);
        reader.join().unwrap();
        assert!(
            result
                .expect("Cache check waited for an active preparation")
                .unwrap()
                .is_none()
        );
    }
}
