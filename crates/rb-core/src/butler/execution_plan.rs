use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::{Path, PathBuf},
};

/// Resolved child process inputs, independent of ambient process state.
pub struct ExecutionPlan {
    executable: PathBuf,
    arguments: Vec<String>,
    directory: PathBuf,
    environment: BTreeMap<OsString, OsString>,
}

impl ExecutionPlan {
    pub(super) fn new(
        executable: PathBuf,
        arguments: Vec<String>,
        directory: PathBuf,
        environment: BTreeMap<OsString, OsString>,
    ) -> Self {
        Self {
            executable,
            arguments,
            directory,
            environment,
        }
    }

    pub fn executable(&self) -> &Path {
        &self.executable
    }
    pub fn arguments(&self) -> &[String] {
        &self.arguments
    }
    pub fn directory(&self) -> &Path {
        &self.directory
    }
    pub fn environment(&self) -> &BTreeMap<OsString, OsString> {
        &self.environment
    }

    pub fn command(&self) -> std::process::Command {
        let mut command = std::process::Command::new(&self.executable);
        command
            .args(&self.arguments)
            .current_dir(&self.directory)
            .env_clear()
            .envs(&self.environment);
        command
    }
}
