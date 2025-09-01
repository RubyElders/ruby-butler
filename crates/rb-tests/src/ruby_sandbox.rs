use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Test sandbox for creating ruby-* directories and related files.
pub struct RubySandbox {
    td: TempDir,
}

impl RubySandbox {
    /// Create a fresh sandbox.
    pub fn new() -> io::Result<Self> {
        Ok(Self { td: TempDir::new()? })
    }

    /// Root path of the sandbox.
    pub fn root(&self) -> &Path {
        self.td.path()
    }

    /// Create an arbitrary subdirectory (e.g., "jruby-9.4.5.0" or "ruby-3.2.0-rc1").
    pub fn add_dir<S: AsRef<str>>(&self, name: S) -> io::Result<PathBuf> {
        let p = self.root().join(name.as_ref());
        fs::create_dir_all(&p)?;
        Ok(p)
    }

    /// Create a plain file at sandbox root (used to simulate "not a dir").
    pub fn add_file<S: AsRef<str>>(&self, name: S, contents: impl AsRef<[u8]>) -> io::Result<PathBuf> {
        let p = self.root().join(name.as_ref());
        fs::write(&p, contents)?;
        Ok(p)
    }

    /// Create `ruby-<ver>` directory (no bin by default).
    pub fn add_ruby_dir<S: AsRef<str>>(&self, version: S) -> io::Result<PathBuf> {
        self.add_dir(format!("ruby-{}", version.as_ref()))
    }

    /// Create a sandboxed gem base directory for testing
    pub fn gem_base_dir(&self) -> PathBuf {
        self.root().join(".gem")
    }
}
