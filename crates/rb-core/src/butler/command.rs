use std::process::{Stdio, Child, Output};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use super::{ButlerRuntime, ButlerError};
use log::debug;

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
        self.env_vars.insert(key.as_ref().to_string(), val.as_ref().to_string());
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
    pub fn execute_with_context(&mut self, butler_runtime: &ButlerRuntime) -> std::io::Result<Child> {
        let mut cmd = self.build_command_with_context(butler_runtime);
        cmd.spawn()
    }

    /// Execute the command and wait for completion, returning the output
    pub fn output_with_context(&mut self, butler_runtime: &ButlerRuntime) -> std::io::Result<Output> {
        let mut cmd = self.build_command_with_context(butler_runtime);
        cmd.output()
    }

    /// Execute the command and wait for completion, returning the exit status
    pub fn status_with_context(&mut self, butler_runtime: &ButlerRuntime) -> std::io::Result<std::process::ExitStatus> {
        let mut cmd = self.build_command_with_context(butler_runtime);
        cmd.status()
    }

    /// Check if the command exists in the current environment.
    /// 
    /// This method uses the same resolution logic as command execution to determine
    /// if a command is available. It considers both direct command execution and
    /// bundle exec scenarios.
    pub fn command_exists(&self, butler_runtime: &ButlerRuntime) -> bool {
        if self.should_use_bundle_exec(butler_runtime) {
            // For bundle exec commands, check if both bundle and the target command exist
            let bundle_cmd = Command::new("bundle");
            if !bundle_cmd.command_exists_direct(butler_runtime) {
                debug!("Bundle command not found, cannot use bundle exec");
                return false;
            }
            
            // For bundle exec, we assume the command exists if bundle exists
            // The actual gem availability will be checked at runtime
            debug!("Bundle exec scenario - assuming command exists if bundle exists");
            true
        } else {
            // Check direct command existence
            self.command_exists_direct(butler_runtime)
        }
    }

    /// Check if a command exists directly (without bundle exec)
    fn command_exists_direct(&self, butler_runtime: &ButlerRuntime) -> bool {
        let existing_path = std::env::var("PATH").ok();
        let env_vars = butler_runtime.env_vars(existing_path);
        
        if let Some(butler_path) = env_vars.get("PATH") {
            debug!("Checking command existence for '{}' with butler PATH", self.program);
            
            match which::which_in(&self.program, Some(butler_path), std::env::current_dir().unwrap_or_default()) {
                Ok(path) => {
                    debug!("Command '{}' found at: {}", self.program, path.display());
                    true
                }
                Err(e) => {
                    debug!("Command '{}' not found: {}", self.program, e);
                    false
                }
            }
        } else {
            debug!("No butler PATH available for command existence check");
            false
        }
    }

    /// Execute the command with command existence checking, returning ButlerError for missing commands.
    /// 
    /// This method checks if the command exists before attempting execution and returns
    /// appropriate ButlerError::CommandNotFound if the command is not available.
    pub fn execute_with_validation(&mut self, butler_runtime: &ButlerRuntime) -> Result<Child, ButlerError> {
        if !self.command_exists(butler_runtime) {
            return Err(ButlerError::CommandNotFound(self.program.clone()));
        }
        
        self.execute_with_context(butler_runtime)
            .map_err(|e| ButlerError::General(format!("Failed to execute command '{}': {}", self.program, e)))
    }

    /// Execute the command and wait for completion with command existence checking.
    /// 
    /// Returns ButlerError::CommandNotFound if the command is not available.
    pub fn output_with_validation(&mut self, butler_runtime: &ButlerRuntime) -> Result<Output, ButlerError> {
        if !self.command_exists(butler_runtime) {
            return Err(ButlerError::CommandNotFound(self.program.clone()));
        }
        
        self.output_with_context(butler_runtime)
            .map_err(|e| ButlerError::General(format!("Failed to execute command '{}': {}", self.program, e)))
    }

    /// Execute the command and wait for completion with command existence checking.
    /// 
    /// Returns ButlerError::CommandNotFound if the command is not available.
    pub fn status_with_validation(&mut self, butler_runtime: &ButlerRuntime) -> Result<std::process::ExitStatus, ButlerError> {
        if !self.command_exists(butler_runtime) {
            return Err(ButlerError::CommandNotFound(self.program.clone()));
        }
        
        self.status_with_context(butler_runtime)
            .map_err(|e| ButlerError::General(format!("Failed to execute command '{}': {}", self.program, e)))
    }

    /// Check if this command should be executed with bundle exec
    fn should_use_bundle_exec(&self, butler_runtime: &ButlerRuntime) -> bool {
        // Only use bundle exec if:
        // 1. Bundler runtime is configured
        // 2. The command is not a bundle command itself (bundle install, bundle check, etc.)
        if let Some(_bundler_runtime) = butler_runtime.bundler_runtime() {
            !self.is_bundle_command()
        } else {
            false
        }
    }

    /// Check if this is a bundle command (bundle install, bundle check, etc.)
    fn is_bundle_command(&self) -> bool {
        self.program == "bundle" || self.program == "bundler"
    }

    /// Resolve the executable path for cross-platform command execution.
    /// 
    /// On Windows, this will find executables with common extensions (.exe, .cmd, .bat).
    /// On Unix systems, this preserves the original behavior.
    fn resolve_executable_path(&self, butler_runtime: &ButlerRuntime) -> String {
        // Try to resolve the executable using the which crate with the composed environment
        let existing_path = std::env::var("PATH").ok();
        let env_vars = butler_runtime.env_vars(existing_path);
        
        // Create a temporary environment with the butler runtime PATH
        if let Some(butler_path) = env_vars.get("PATH") {
            debug!("Resolving executable '{}' with butler PATH", self.program);
            
            // Use which to find the executable in the butler environment
            match which::which_in(&self.program, Some(butler_path), std::env::current_dir().unwrap_or_default()) {
                Ok(path) => {
                    let resolved = path.to_string_lossy().to_string();
                    debug!("Resolved executable '{}' to: {}", self.program, resolved);
                    resolved
                }
                Err(_) => {
                    debug!("Could not resolve executable '{}', using original name", self.program);
                    self.program.clone()
                }
            }
        } else {
            debug!("No butler PATH available, using original program name: {}", self.program);
            self.program.clone()
        }
    }

    /// Build the actual Command with proper context resolution
    fn build_command_with_context(&mut self, butler_runtime: &ButlerRuntime) -> std::process::Command {
        let mut cmd = if self.should_use_bundle_exec(butler_runtime) {
            // Use bundle exec for gem executables
            let resolved_bundle = self.resolve_bundle_executable(butler_runtime);
            let mut bundle_cmd = std::process::Command::new(resolved_bundle);
            bundle_cmd.arg("exec");
            bundle_cmd.arg(&self.program);
            bundle_cmd.args(&self.args);
            bundle_cmd
        } else {
            // Use the program directly, resolving the executable path
            let resolved_program = self.resolve_executable_path(butler_runtime);
            let mut direct_cmd = std::process::Command::new(resolved_program);
            direct_cmd.args(&self.args);
            direct_cmd
        };

        // Apply butler runtime environment variables, preserving existing PATH
        let existing_path = std::env::var("PATH").ok();
        for (key, value) in butler_runtime.env_vars(existing_path) {
            cmd.env(key, value);
        }

        // Apply additional environment variables
        for (key, value) in &self.env_vars {
            cmd.env(key, value);
        }

        // Set current directory
        if let Some(ref dir) = self.current_dir {
            cmd.current_dir(dir);
        }

        // Configure stdio
        if let Some(stdout) = self.stdout.take() {
            cmd.stdout(stdout);
        }
        if let Some(stderr) = self.stderr.take() {
            cmd.stderr(stderr);
        }
        if let Some(stdin) = self.stdin.take() {
            cmd.stdin(stdin);
        }

        cmd
    }

    /// Resolve the bundle executable path for cross-platform execution
    fn resolve_bundle_executable(&self, butler_runtime: &ButlerRuntime) -> String {
        // Create a temporary command to resolve bundle executable
        let bundle_program = "bundle".to_string();
        let temp_cmd = Command {
            program: bundle_program,
            args: Vec::new(),
            current_dir: None,
            env_vars: HashMap::new(),
            stdout: None,
            stderr: None,
            stdin: None,
        };
        temp_cmd.resolve_executable_path(butler_runtime)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ruby::{RubyRuntime, RubyType};
    use std::path::PathBuf;
    use semver::Version;

    #[test]
    fn test_butler_command_basic_creation() {
        let cmd = Command::new("echo");
        assert_eq!(cmd.program, "echo");
        assert!(cmd.args.is_empty());
    }

    #[test]
    fn test_butler_command_with_args() {
        let mut cmd = Command::new("ruby");
        cmd.arg("-v").args(&["--version", "test"]);
        
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
