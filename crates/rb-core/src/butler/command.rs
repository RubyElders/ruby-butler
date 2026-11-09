use super::{ButlerError, ButlerRuntime, ExecutionPlan};
use log::debug;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Output, Stdio};

/// A sophisticated command execution abstraction that understands Ruby environments
/// and executes commands with appropriate context and preparation.
pub struct Command {
    program: String,
    args: Vec<String>,
    current_dir: Option<PathBuf>,
    env_vars: HashMap<String, String>,
    stdout: Option<Stdio>,
    stderr: Option<Stdio>,
    stdin: Option<Stdio>,
}

impl Command {
    /// Create a new command with the specified program
    pub fn new<S: AsRef<str>>(program: S) -> Self {
        Self {
            program: program.as_ref().to_string(),
            args: Vec::new(),
            current_dir: None,
            env_vars: HashMap::new(),
            stdout: None,
            stderr: None,
            stdin: None,
        }
    }

    /// Add an argument to the command
    pub fn arg<S: AsRef<str>>(&mut self, arg: S) -> &mut Self {
        self.args.push(arg.as_ref().to_string());
        self
    }

    /// Add multiple arguments to the command
    pub fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        for arg in args {
            self.arg(arg);
        }
        self
    }

    /// Set the current directory for command execution
    pub fn current_dir<P: AsRef<Path>>(&mut self, dir: P) -> &mut Self {
        self.current_dir = Some(dir.as_ref().to_path_buf());
        self
    }

    /// Set an environment variable
    pub fn env<K, V>(&mut self, key: K, val: V) -> &mut Self
    where
        K: AsRef<str>,
        V: AsRef<str>,
    {
        self.env_vars
            .insert(key.as_ref().to_string(), val.as_ref().to_string());
        self
    }

    /// Configure stdout
    pub fn stdout<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Self {
        self.stdout = Some(cfg.into());
        self
    }

    /// Configure stderr  
    pub fn stderr<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Self {
        self.stderr = Some(cfg.into());
        self
    }

    /// Configure stdin
    pub fn stdin<T: Into<Stdio>>(&mut self, cfg: T) -> &mut Self {
        self.stdin = Some(cfg.into());
        self
    }

    /// Execute the command with the specified butler runtime context.
    ///
    /// This method intelligently determines how to run the command:
    /// - If bundler runtime is present, all commands except bundle commands themselves
    ///   are prefixed with "bundle exec" for proper dependency isolation
    /// - Bundle commands (bundle install, bundle check, etc.) always run directly
    /// - Environment variables from the butler runtime are automatically applied
    pub fn execute_with_context(
        &mut self,
        butler_runtime: &ButlerRuntime,
    ) -> std::io::Result<Child> {
        let mut cmd = self.build_command_with_context(butler_runtime, false)?;
        cmd.spawn()
    }

    /// Execute the command and wait for completion, returning the output
    pub fn output_with_context(
        &mut self,
        butler_runtime: &ButlerRuntime,
    ) -> std::io::Result<Output> {
        let mut cmd = self.build_command_with_context(butler_runtime, false)?;
        cmd.output()
    }

    /// Execute the command and wait for completion, returning the exit status
    pub fn status_with_context(
        &mut self,
        butler_runtime: &ButlerRuntime,
    ) -> std::io::Result<std::process::ExitStatus> {
        let mut cmd = self.build_command_with_context(butler_runtime, false)?;
        cmd.status()
    }

    /// Check if the command exists in the current environment.
    ///
    /// This method uses the same resolution logic as command execution to determine
    /// if a command is available. It considers both direct command execution and
    /// bundle exec scenarios.
    pub fn command_exists(&self, butler_runtime: &ButlerRuntime) -> bool {
        self.plan(butler_runtime, true).is_ok()
    }

    #[cfg(test)]
    fn command_exists_direct(&self, runtime: &ButlerRuntime) -> bool {
        self.plan(runtime, true).is_ok()
    }

    /// Execute the command with command existence checking, returning ButlerError for missing commands.
    ///
    /// This method checks if the command exists before attempting execution and returns
    /// appropriate ButlerError::CommandNotFound if the command is not available.
    pub fn execute_with_validation(
        &mut self,
        butler_runtime: &ButlerRuntime,
    ) -> Result<Child, ButlerError> {
        self.build_command_with_context(butler_runtime, true)
            .map_err(|error| self.execution_error(error))?
            .spawn()
            .map_err(|error| self.execution_error(error))
    }

    /// Execute the command and wait for completion with command existence checking.
    ///
    /// Returns ButlerError::CommandNotFound if the command is not available.
    pub fn output_with_validation(
        &mut self,
        butler_runtime: &ButlerRuntime,
    ) -> Result<Output, ButlerError> {
        self.build_command_with_context(butler_runtime, true)
            .map_err(|error| self.execution_error(error))?
            .output()
            .map_err(|error| self.execution_error(error))
    }

    /// Execute the command and wait for completion with command existence checking.
    ///
    /// Returns ButlerError::CommandNotFound if the command is not available.
    pub fn status_with_validation(
        &mut self,
        butler_runtime: &ButlerRuntime,
    ) -> Result<std::process::ExitStatus, ButlerError> {
        self.build_command_with_context(butler_runtime, true)
            .map_err(|error| self.execution_error(error))?
            .status()
            .map_err(|error| self.execution_error(error))
    }

    fn execution_error(&self, error: std::io::Error) -> ButlerError {
        if error.kind() == std::io::ErrorKind::NotFound {
            ButlerError::CommandNotFound(self.program.clone())
        } else {
            ButlerError::General(format!(
                "Failed to execute command '{}': {error}",
                self.program
            ))
        }
    }

    /// Check if this command should be executed with bundle exec
    fn should_use_bundle_exec(&self, butler_runtime: &ButlerRuntime) -> bool {
        // Only use bundle exec if:
        // 1. Bundler runtime is configured
        // 2. The command is not a bundle command itself (bundle install, bundle check, etc.)
        if butler_runtime.uses_bundler() {
            !self.is_bundle_command()
        } else {
            false
        }
    }

    /// Check if this is a bundle command (bundle install, bundle check, etc.)
    fn is_bundle_command(&self) -> bool {
        self.program == "bundle" || self.program == "bundler"
    }

    pub fn execution_plan(&self, runtime: &ButlerRuntime) -> std::io::Result<ExecutionPlan> {
        self.plan(runtime, true)
    }

    fn plan(&self, runtime: &ButlerRuntime, validate: bool) -> std::io::Result<ExecutionPlan> {
        let directory = self
            .current_dir
            .as_ref()
            .map(|path| runtime.current_dir().join(path))
            .unwrap_or_else(|| runtime.current_dir().to_owned());
        let mut environment = runtime.execution_environment().clone();
        environment.extend(self.env_vars.iter().map(|(key, value)| {
            #[cfg(windows)]
            let key = key.to_ascii_uppercase();
            (key.into(), value.into())
        }));
        let bundled = self.should_use_bundle_exec(runtime);
        let program = if bundled { "bundle" } else { &self.program };
        if validate {
            debug!(
                "Checking command existence for '{}' with butler PATH",
                program
            );
        }
        debug!("Resolving executable '{}' with butler PATH", program);
        let search_path = environment
            .get(std::ffi::OsStr::new("PATH"))
            .map(|path| {
                std::env::join_paths(std::env::split_paths(path).map(|part| directory.join(part)))
            })
            .transpose()
            .map_err(std::io::Error::other)?;
        #[cfg(not(windows))]
        let resolved = which::which_in(program, search_path, &directory).ok();
        #[cfg(windows)]
        let resolved = resolve_windows(program, search_path.as_deref(), &directory, &environment);
        let executable = match resolved {
            Some(path) => {
                if validate {
                    debug!("Command '{}' found at: {}", program, path.display());
                    if bundled {
                        debug!("Bundle exec scenario - assuming command exists if bundle exists");
                    }
                }
                path
            }
            None if !validate => {
                debug!(
                    "Could not resolve executable '{}', using original name",
                    program
                );
                PathBuf::from(program)
            }
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Command not found: {}", self.program),
                ));
            }
        };
        let mut arguments = Vec::new();
        if bundled {
            arguments.push("exec".into());
            arguments.push(self.program.clone());
        }
        arguments.extend(self.args.clone());
        debug!(
            "Resolved executable '{}' to: {}",
            program,
            executable.display()
        );
        Ok(ExecutionPlan::new(
            executable,
            arguments,
            directory,
            environment,
        ))
    }

    fn build_command_with_context(
        &mut self,
        runtime: &ButlerRuntime,
        validate: bool,
    ) -> std::io::Result<std::process::Command> {
        let mut command = self.plan(runtime, validate)?.command();
        if let Some(stdout) = self.stdout.take() {
            command.stdout(stdout);
        }
        if let Some(stderr) = self.stderr.take() {
            command.stderr(stderr);
        }
        if let Some(stdin) = self.stdin.take() {
            command.stdin(stdin);
        }
        Ok(command)
    }

    #[cfg(test)]
    fn resolve_executable_path(&self, runtime: &ButlerRuntime) -> String {
        self.plan(runtime, false)
            .unwrap()
            .executable()
            .to_string_lossy()
            .into_owned()
    }
}

#[cfg(any(test, windows))]
fn resolve_windows(
    program: &str,
    search_path: Option<&std::ffi::OsStr>,
    directory: &Path,
    environment: &std::collections::BTreeMap<std::ffi::OsString, std::ffi::OsString>,
) -> Option<PathBuf> {
    let program = Path::new(program);
    if program.is_absolute() && program.is_file() {
        return Some(program.to_owned());
    }
    let extensions = environment
        .get(std::ffi::OsStr::new("PATHEXT"))
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .split(';')
        .filter(|extension| extension.starts_with('.'))
        .collect::<Vec<_>>();
    let candidates = if program.components().count() > 1 || program.is_absolute() {
        vec![directory.join(program)]
    } else {
        search_path
            .map(std::env::split_paths)
            .into_iter()
            .flatten()
            .map(|path| path.join(program))
            .collect()
    };
    let has_extension = program.extension().is_some_and(|extension| {
        extensions.iter().any(|candidate| {
            extension
                .to_str()
                .is_some_and(|extension| candidate[1..].eq_ignore_ascii_case(extension))
        })
    });
    for candidate in candidates {
        if has_extension {
            if candidate.is_file() {
                return Some(candidate);
            }
        } else {
            for extension in &extensions {
                let mut path = candidate.as_os_str().to_owned();
                path.push(extension);
                let path = PathBuf::from(path);
                if path.is_file() {
                    return Some(path);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ruby::{RubyRuntime, RubyType};
    use semver::Version;
    use std::path::PathBuf;

    #[test]
    fn windows_extensions_are_resolved_from_each_supplied_environment() {
        let directory = tempfile::tempdir().unwrap();
        let executable = directory.path().join("local-tool.custom");
        std::fs::write(&executable, "fixture").unwrap();
        let environment = std::collections::BTreeMap::from([("PATHEXT".into(), ".custom".into())]);
        assert_eq!(
            resolve_windows(
                "local-tool",
                Some(directory.path().as_os_str()),
                directory.path(),
                &environment
            ),
            Some(executable)
        );
        let environment = std::collections::BTreeMap::from([("PATHEXT".into(), ".exe".into())]);
        assert!(
            resolve_windows(
                "local-tool",
                Some(directory.path().as_os_str()),
                directory.path(),
                &environment
            )
            .is_none()
        );
    }

    #[test]
    fn test_butler_command_basic_creation() {
        let cmd = Command::new("echo");
        assert_eq!(cmd.program, "echo");
        assert!(cmd.args.is_empty());
    }

    #[test]
    fn test_butler_command_with_args() {
        let mut cmd = Command::new("ruby");
        cmd.arg("-v").args(["--version", "test"]);

        assert_eq!(cmd.program, "ruby");
        assert_eq!(cmd.args, vec!["-v", "--version", "test"]);
    }

    #[test]
    fn test_bundle_command_detection() {
        let bundle_cmd = Command::new("bundle");
        assert!(bundle_cmd.is_bundle_command());

        let bundler_cmd = Command::new("bundler");
        assert!(bundler_cmd.is_bundle_command());

        let ruby_cmd = Command::new("ruby");
        assert!(!ruby_cmd.is_bundle_command());
    }

    #[test]
    fn test_should_use_bundle_exec_logic() {
        // Create a minimal ruby runtime for testing
        let ruby_runtime = RubyRuntime {
            kind: RubyType::CRuby,
            version: Version::new(3, 0, 0),
            root: PathBuf::from("/test"),
        };

        // Test without bundler runtime
        let butler_runtime = ButlerRuntime::new(ruby_runtime, None);

        // Without bundler runtime, should not use bundle exec for any command
        let ruby_cmd = Command::new("ruby");
        assert!(!ruby_cmd.should_use_bundle_exec(&butler_runtime));

        let ls_cmd = Command::new("ls");
        assert!(!ls_cmd.should_use_bundle_exec(&butler_runtime));

        let rails_cmd = Command::new("rails");
        assert!(!rails_cmd.should_use_bundle_exec(&butler_runtime));

        // Bundle commands should never use bundle exec (even with bundler runtime)
        let bundle_cmd = Command::new("bundle");
        assert!(!bundle_cmd.should_use_bundle_exec(&butler_runtime));

        // Note: Testing with bundler runtime requires filesystem setup
        // and is better covered in integration tests
    }

    #[test]
    fn test_executable_resolution_fallback() {
        // Create a minimal butler runtime for testing
        use crate::ruby::{RubyRuntime, RubyType};
        use semver::Version;
        use std::path::PathBuf;

        let ruby_runtime = RubyRuntime {
            kind: RubyType::CRuby,
            version: Version::new(3, 0, 0),
            root: PathBuf::from("/nonexistent"),
        };

        let butler_runtime = ButlerRuntime::new(ruby_runtime, None);

        // Test that non-existent commands fall back to original program name
        let nonexistent_cmd = Command::new("nonexistent-command-12345");
        let resolved = nonexistent_cmd.resolve_executable_path(&butler_runtime);
        assert_eq!(resolved, "nonexistent-command-12345");

        // Test that the resolution method exists and is callable
        let gem_cmd = Command::new("gem");
        let _resolved = gem_cmd.resolve_executable_path(&butler_runtime);
        // Note: We can't assert the exact resolved path since it depends on the system
        // but we can verify the method runs without panicking
    }

    #[test]
    fn test_command_exists_for_nonexistent_command() {
        use crate::ruby::{RubyRuntime, RubyType};
        use semver::Version;
        use std::path::PathBuf;

        let ruby_runtime = RubyRuntime {
            kind: RubyType::CRuby,
            version: Version::new(3, 0, 0),
            root: PathBuf::from("/nonexistent"),
        };

        let butler_runtime = ButlerRuntime::new(ruby_runtime, None);

        // Test that clearly non-existent command returns false
        let nonexistent_cmd = Command::new("definitely-does-not-exist-12345");
        assert!(!nonexistent_cmd.command_exists(&butler_runtime));

        // Test the direct checking method as well
        assert!(!nonexistent_cmd.command_exists_direct(&butler_runtime));
    }

    #[test]
    fn test_command_exists_for_bundle_commands() {
        use crate::ruby::{RubyRuntime, RubyType};
        use semver::Version;
        use std::path::PathBuf;

        let ruby_runtime = RubyRuntime {
            kind: RubyType::CRuby,
            version: Version::new(3, 0, 0),
            root: PathBuf::from("/test"),
        };

        let butler_runtime = ButlerRuntime::new(ruby_runtime, None);

        // Bundle commands should always check directly, never through bundle exec
        let bundle_cmd = Command::new("bundle");
        assert!(!bundle_cmd.should_use_bundle_exec(&butler_runtime));

        // The existence check should go through the direct path
        // We can't test the actual result since it depends on system state,
        // but we can verify the logic flow
        let _exists = bundle_cmd.command_exists(&butler_runtime);
    }

    #[test]
    fn test_status_with_validation_for_nonexistent_command() {
        use crate::ruby::{RubyRuntime, RubyType};
        use semver::Version;
        use std::path::PathBuf;

        let ruby_runtime = RubyRuntime {
            kind: RubyType::CRuby,
            version: Version::new(3, 0, 0),
            root: PathBuf::from("/nonexistent"),
        };

        let butler_runtime = ButlerRuntime::new(ruby_runtime, None);

        // Test that validation method returns CommandNotFound error
        let mut nonexistent_cmd = Command::new("definitely-does-not-exist-12345");
        let result = nonexistent_cmd.status_with_validation(&butler_runtime);

        assert!(result.is_err());
        match result.unwrap_err() {
            ButlerError::CommandNotFound(command) => {
                assert_eq!(command, "definitely-does-not-exist-12345");
            }
            _ => panic!("Expected CommandNotFound error"),
        }
    }

    #[test]
    fn test_output_with_validation_for_nonexistent_command() {
        use crate::ruby::{RubyRuntime, RubyType};
        use semver::Version;
        use std::path::PathBuf;

        let ruby_runtime = RubyRuntime {
            kind: RubyType::CRuby,
            version: Version::new(3, 0, 0),
            root: PathBuf::from("/nonexistent"),
        };

        let butler_runtime = ButlerRuntime::new(ruby_runtime, None);

        // Test that validation method returns CommandNotFound error
        let mut nonexistent_cmd = Command::new("definitely-does-not-exist-12345");
        let result = nonexistent_cmd.output_with_validation(&butler_runtime);

        assert!(result.is_err());
        match result.unwrap_err() {
            ButlerError::CommandNotFound(command) => {
                assert_eq!(command, "definitely-does-not-exist-12345");
            }
            _ => panic!("Expected CommandNotFound error"),
        }
    }

    #[test]
    fn test_execute_with_validation_for_nonexistent_command() {
        use crate::ruby::{RubyRuntime, RubyType};
        use semver::Version;
        use std::path::PathBuf;

        let ruby_runtime = RubyRuntime {
            kind: RubyType::CRuby,
            version: Version::new(3, 0, 0),
            root: PathBuf::from("/nonexistent"),
        };

        let butler_runtime = ButlerRuntime::new(ruby_runtime, None);

        // Test that validation method returns CommandNotFound error
        let mut nonexistent_cmd = Command::new("definitely-does-not-exist-12345");
        let result = nonexistent_cmd.execute_with_validation(&butler_runtime);

        assert!(result.is_err());
        match result.unwrap_err() {
            ButlerError::CommandNotFound(command) => {
                assert_eq!(command, "definitely-does-not-exist-12345");
            }
            _ => panic!("Expected CommandNotFound error"),
        }
    }
}
