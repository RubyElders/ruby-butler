use crate::{
    InstalledBundle, PrepareOptions, PreparedBundle, ProjectInput,
    constants::{MANIFEST_FILE, PREPARED_STATE, PROJECT_LOCK, STATE_DIRECTORY},
};
use anyhow::{Context, Result, ensure};
use rb_gemfile::{Evaluator, SnapshotCache};
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
    anchor: PathBuf,
    inputs: BTreeSet<PathBuf>,
    input_hash: String,
    output_hash: String,
    installed: InstalledBundle,
    lockfile: PathBuf,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    state: State,
    checksum: String,
}

pub(crate) struct Candidate {
    anchor: PathBuf,
    before: String,
}

pub(crate) fn lock(root: &Path) -> Result<File> {
    let state = root.join(STATE_DIRECTORY);
    fs::create_dir_all(&state)?;
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(state.join(PROJECT_LOCK))?;
    file.lock()?;
    Ok(file)
}

/// Returns a prepared installation when its tracked inputs and files still match.
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
    ensure!(state.anchor == anchor(options)?, "Project input changed");
    ensure!(
        state.input_hash == inputs(options, &state.anchor, &state.inputs)?,
        "Preparation inputs changed"
    );
    ensure!(
        state.output_hash == outputs(options, &state.installed, &state.lockfile)?,
        "Installed bundle changed"
    );
    let mut installed = state.installed;
    installed.reused += installed.installed;
    installed.installed = 0;
    Ok(installed)
}

impl Candidate {
    pub(crate) fn capture(options: &PrepareOptions) -> Result<Self> {
        let anchor = anchor(options)?;
        let before = inputs(options, &anchor, &BTreeSet::new())?;
        Ok(Self { anchor, before })
    }

    pub(crate) fn observe(
        &self,
        options: &PrepareOptions,
        input: &ProjectInput,
        runtime: &crate::RubyEnvironment,
    ) -> Result<(BTreeSet<PathBuf>, String)> {
        ensure!(
            self.before == inputs(options, &self.anchor, &BTreeSet::new())?,
            "Project changed during evaluation"
        );
        let mut paths: BTreeSet<_> = input
            .evaluation_inputs
            .iter()
            .chain(&runtime.configuration_inputs)
            .cloned()
            .collect();
        for dependency in input
            .manifest
            .dependencies
            .values()
            .flat_map(|dependency| std::iter::once(dependency).chain(&dependency.variants))
        {
            if dependency.enabled
                && let Some(path) = &dependency.path
            {
                paths.insert(path.into());
            }
        }
        ensure!(
            input.evaluation_inputs.iter().all(|file| {
                file.starts_with(&input.root)
                    || paths
                        .iter()
                        .any(|directory| directory.is_dir() && file.starts_with(directory))
            }),
            "External evaluation inputs are not covered by project or path-source directories"
        );
        Ok((paths.clone(), inputs(options, &self.anchor, &paths)?))
    }

    pub(crate) fn publish(
        self,
        options: &PrepareOptions,
        prepared: &PreparedBundle,
        paths: BTreeSet<PathBuf>,
        before: String,
    ) -> Result<()> {
        ensure!(
            self.before == inputs(options, &self.anchor, &BTreeSet::new())?,
            "Project changed during preparation"
        );
        ensure!(
            before == inputs(options, &self.anchor, &paths)?,
            "Source inputs changed during preparation"
        );
        let state = State {
            configuration: configuration(options)?,
            input_hash: before,
            output_hash: outputs(options, &prepared.installed, &prepared.input.lockfile)?,
            lockfile: prepared.input.lockfile.clone(),
            installed: prepared.installed.clone(),
            anchor: self.anchor,
            inputs: paths,
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
    if let Some(file) = std::env::var_os("BUNDLE_GEMFILE").filter(|file| !file.is_empty()) {
        return Ok(dunce::canonicalize(std::env::current_dir()?.join(file))?);
    }
    ["rbproject.kdl", "rbproject.toml", "gems.rb", "Gemfile"]
        .iter()
        .map(|name| options.project.join(name))
        .find(|path| path.is_file())
        .map(dunce::canonicalize)
        .transpose()?
        .context("No project input found")
}

fn inputs(options: &PrepareOptions, anchor: &Path, paths: &BTreeSet<PathBuf>) -> Result<String> {
    let mut cache =
        crate::input::snapshot_cache(&options.project, PREPARED_STATE).input(&options.project);
    for path in paths {
        cache = cache.input(path);
    }
    Ok(cache.fingerprint(&Evaluator::new(&options.ruby), anchor)?)
}

fn outputs(
    options: &PrepareOptions,
    installed: &InstalledBundle,
    lockfile: &Path,
) -> Result<String> {
    let gemfile = installed.gemfile.as_ref().context("No runtime Gemfile")?;
    let mut cache = SnapshotCache::new(options.project.join(STATE_DIRECTORY).join(PREPARED_STATE))
        .input(&installed.bundle_path)
        .input(lockfile)
        .input(options.project.join("Gemfile.lock"))
        .input(options.project.join("gems.locked"))
        .input(options.project.join(STATE_DIRECTORY).join(MANIFEST_FILE));
    let mut filename = gemfile.as_os_str().to_owned();
    filename.push(".lock");
    cache = cache.input(PathBuf::from(filename));
    Ok(cache.fingerprint(&Evaluator::new(&options.ruby), gemfile)?)
}

fn configuration(options: &PrepareOptions) -> Result<String> {
    let executable = std::env::current_exe()?;
    let metadata = fs::metadata(&executable)?;
    let data = format!(
        "prepared state 1 {options:?} {executable:?} {:?}",
        (metadata.len(), metadata.modified()?)
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
