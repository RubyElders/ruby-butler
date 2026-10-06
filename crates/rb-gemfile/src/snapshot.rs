use crate::{
    Evaluation, Evaluator,
    evaluator::{SOURCES, gemfile_path},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
};

/// Opt-in cache for deterministic Gemfiles, including their project directory.
/// Add inputs outside that directory with `input`. Time, network and subprocess
/// results are not cache inputs; use `Evaluator::evaluate` for such Gemfiles.
pub struct SnapshotCache {
    path: PathBuf,
    inputs: Vec<PathBuf>,
    outputs: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct CachedEvaluation {
    pub evaluation: Evaluation,
    pub reused: bool,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    fingerprint: String,
    evaluation: Evaluation,
    checksum: String,
}

impl SnapshotCache {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            inputs: Vec::new(),
            outputs: Vec::new(),
        }
    }

    /// Declares an additional file or directory that evaluation may inspect.
    pub fn input(mut self, path: impl Into<PathBuf>) -> Self {
        self.inputs.push(path.into());
        self
    }

    /// Excludes a generated file that the Gemfile must not read.
    pub fn output(mut self, path: impl Into<PathBuf>) -> Self {
        self.outputs.push(path.into());
        self
    }

    pub fn evaluate(
        &self,
        evaluator: &Evaluator,
        gemfile: impl AsRef<Path>,
    ) -> io::Result<CachedEvaluation> {
        let gemfile = gemfile_path(gemfile.as_ref())?;
        let directory = gemfile.parent().unwrap();
        let path = absolute(&self.path)?;
        let filename = path.file_name().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "Snapshot must be a file path")
        })?;
        fs::create_dir_all(path.parent().unwrap())?;
        let path = dunce::canonicalize(path.parent().unwrap())?.join(filename);
        let inputs = self
            .inputs
            .iter()
            .map(|path| absolute(path))
            .collect::<io::Result<Vec<_>>>()?;
        let ruby = evaluator.ruby_executable()?;
        if path == gemfile || path == ruby || inputs.iter().any(|input| input == &path) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Snapshot must not overwrite an input file",
            ));
        }
        let mut excluded = vec![path.clone()];
        for output in &self.outputs {
            let output = absolute(output)?;
            if output.is_dir() || output == gemfile || output == ruby || inputs.contains(&output) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "Generated output must be a file separate from explicit inputs",
                ));
            }
            excluded.push(output);
        }
        let mut lock_path = path.as_os_str().to_owned();
        lock_path.push(".lock");
        let lock_path = PathBuf::from(lock_path);
        if lock_path == gemfile || lock_path == ruby || inputs.contains(&lock_path) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Snapshot lock must not be an input file",
            ));
        }
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)?;
        lock.lock()?;
        excluded.push(lock_path);
        let before = fingerprint(evaluator, &gemfile, directory, &inputs, &ruby, &excluded)?;
        match fs::read(&path) {
            Ok(bytes) => {
                if let Ok(snapshot) = serde_json::from_slice::<Snapshot>(&bytes)
                    && snapshot.fingerprint == before
                    && snapshot.checksum == evaluation_checksum(&snapshot.evaluation)?
                {
                    return Ok(CachedEvaluation {
                        evaluation: snapshot.evaluation,
                        reused: true,
                    });
                }
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        let evaluation = evaluator.evaluate(&gemfile)?;
        let covered = evaluation.inputs.iter().all(|path| {
            !excluded.contains(path)
                && (path.starts_with(directory)
                    || inputs
                        .iter()
                        .any(|input| path == input || (input.is_dir() && path.starts_with(input))))
        });
        if covered
            && before == fingerprint(evaluator, &gemfile, directory, &inputs, &ruby, &excluded)?
        {
            let snapshot = Snapshot {
                fingerprint: before,
                checksum: evaluation_checksum(&evaluation)?,
                evaluation: evaluation.clone(),
            };
            let bytes = serde_json::to_vec(&snapshot).map_err(io::Error::other)?;
            atomic_write(&path, &bytes)?;
        }
        Ok(CachedEvaluation {
            evaluation,
            reused: false,
        })
    }
}

/// Replaces a generated project file without exposing partial contents.
pub fn write_project(path: impl AsRef<Path>, contents: &str) -> io::Result<()> {
    atomic_write(path.as_ref(), contents.as_bytes())
}

fn evaluation_checksum(evaluation: &Evaluation) -> io::Result<String> {
    let bytes = serde_json::to_vec(evaluation).map_err(io::Error::other)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn absolute(path: &Path) -> io::Result<PathBuf> {
    if path.exists() {
        return dunce::canonicalize(path);
    }
    if path.is_absolute() {
        Ok(path.to_owned())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

fn fingerprint(
    evaluator: &Evaluator,
    gemfile: &Path,
    directory: &Path,
    inputs: &[PathBuf],
    ruby: &Path,
    excluded: &[PathBuf],
) -> io::Result<String> {
    let mut hash = Sha256::new();
    field(&mut hash, b"rb-gemfile snapshot 1");
    field(&mut hash, format!("{gemfile:?}").as_bytes());
    field(&mut hash, std::env::consts::OS.as_bytes());
    field(&mut hash, std::env::consts::ARCH.as_bytes());
    for source in [
        include_str!("evaluator.rs"),
        include_str!("format.rs"),
        include_str!("manifest.rs"),
        include_str!("sources.rs"),
        include_str!("snapshot.rs"),
    ] {
        field(&mut hash, source.as_bytes());
    }
    for (name, source) in SOURCES {
        field(&mut hash, name.as_bytes());
        field(&mut hash, source.as_bytes());
    }
    let mut environment: std::collections::BTreeMap<_, _> = std::env::vars_os().collect();
    environment.extend(evaluator.environment.clone());
    environment.remove(std::ffi::OsStr::new("RUBYOPT"));
    for pair in environment {
        field(&mut hash, format!("{pair:?}").as_bytes());
    }
    let mut visited = BTreeSet::new();
    hash_path(&mut hash, ruby, excluded, &mut visited)?;
    hash_path(&mut hash, directory, excluded, &mut visited)?;
    for input in inputs {
        hash_path(&mut hash, input, excluded, &mut visited)?;
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn field(hash: &mut Sha256, bytes: &[u8]) {
    hash.update((bytes.len() as u64).to_le_bytes());
    hash.update(bytes);
}

fn hash_path(
    hash: &mut Sha256,
    path: &Path,
    excluded: &[PathBuf],
    visited: &mut BTreeSet<PathBuf>,
) -> io::Result<()> {
    if excluded.iter().any(|excluded| path == excluded) {
        return Ok(());
    }
    field(hash, format!("{path:?}").as_bytes());
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            field(hash, b"missing");
            return Ok(());
        }
        Err(error) => return Err(error),
    };
    if metadata.is_symlink() {
        field(hash, b"symlink");
        field(hash, format!("{:?}", fs::read_link(path)?).as_bytes());
        let target = dunce::canonicalize(path)?;
        if !visited.contains(&target) {
            hash_path(hash, &target, excluded, visited)?;
        }
    } else if metadata.is_dir() {
        field(hash, b"directory");
        if !visited.insert(dunce::canonicalize(path)?) {
            return Ok(());
        }
        let mut entries: Vec<_> = fs::read_dir(path)?
            .map(|entry| entry.map(|entry| entry.path()))
            .collect::<io::Result<_>>()?;
        entries.sort();
        for entry in entries {
            hash_path(hash, &entry, excluded, visited)?;
        }
    } else if metadata.is_file() {
        field(hash, b"file");
        field(hash, &fs::read(path)?);
    } else {
        return Err(io::Error::other(format!(
            "Unsupported cache input: {}",
            path.display()
        )));
    }
    Ok(())
}

fn atomic_write(path: &Path, contents: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(parent)?;
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.write_all(contents)?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(|error| error.error)?;
    Ok(())
}
