use std::process::{Stdio, Child, Output};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use super::ButlerRuntime;

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

    /// Build the actual Command with proper context resolution
    fn build_command_with_context(&mut self, butler_runtime: &ButlerRuntime) -> std::process::Command {
        let mut cmd = if self.should_use_bundle_exec(butler_runtime) {
            // Use bundle exec for gem executables
            let mut bundle_cmd = std::process::Command::new("bundle");
            bundle_cmd.arg("exec");
            bundle_cmd.arg(&self.program);
            bundle_cmd.args(&self.args);
            bundle_cmd
        } else {
            // Use the program directly
            let mut direct_cmd = std::process::Command::new(&self.program);
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
}
