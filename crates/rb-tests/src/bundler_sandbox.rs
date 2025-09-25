use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Test sandbox for creating bundler project structures with Gemfiles and directories.
pub struct BundlerSandbox {
    td: TempDir,
}

impl BundlerSandbox {
    /// Create a fresh sandbox.
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            td: TempDir::new()?,
        })
    }

    /// Root path of the sandbox.
    pub fn root(&self) -> &Path {
        self.td.path()
    }

    /// Create an arbitrary subdirectory within the sandbox.
    pub fn add_dir<S: AsRef<str>>(&self, name: S) -> io::Result<PathBuf> {
        let p = self.root().join(name.as_ref());
        fs::create_dir_all(&p)?;
        Ok(p)
    }

    /// Create a file with specified contents at the given path.
    pub fn add_file<S: AsRef<str>>(
        &self,
        path: S,
        contents: impl AsRef<[u8]>,
    ) -> io::Result<PathBuf> {
        let p = self.root().join(path.as_ref());
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&p, contents)?;
        Ok(p)
    }

    /// Create a Gemfile at the specified directory path within the sandbox.
    /// If no path is provided, creates it at the sandbox root.
    pub fn add_gemfile<S: AsRef<str>>(
        &self,
        dir_path: Option<S>,
        contents: Option<&str>,
    ) -> io::Result<PathBuf> {
        let base_dir = if let Some(path) = dir_path {
            let dir = self.root().join(path.as_ref());
            fs::create_dir_all(&dir)?;
            dir
        } else {
            self.root().to_path_buf()
        };

        let gemfile_path = base_dir.join("Gemfile");
        let default_contents = "source 'https://rubygems.org'\n\ngem 'rails'\n";
        let contents = contents.unwrap_or(default_contents);

        fs::write(&gemfile_path, contents)?;
        Ok(gemfile_path)
    }

    /// Create a bundler project structure with Gemfile and .rb directory.
    /// Optionally create the vendor/bundler structure to simulate a configured bundler.
    pub fn add_bundler_project<S: AsRef<str>>(
        &self,
        project_name: S,
        configured: bool,
    ) -> io::Result<PathBuf> {
        let project_dir = self.add_dir(&project_name)?;

        // Create Gemfile
        let gemfile_contents = format!(
            "source 'https://rubygems.org'\n\ngem 'json'\ngem 'rake'\n# Project: {}\n",
            project_name.as_ref()
        );
        let gemfile_path = project_dir.join("Gemfile");
        fs::write(&gemfile_path, gemfile_contents)?;

        // Create .rb directory
        let rb_dir = project_dir.join(".rb");
        fs::create_dir_all(&rb_dir)?;

        if configured {
            // Create vendor/bundler structure
            let vendor_bundler_dir = rb_dir.join("vendor").join("bundler");
            fs::create_dir_all(&vendor_bundler_dir)?;

            // Create bin directory
            let bin_dir = vendor_bundler_dir.join("bin");
            fs::create_dir_all(&bin_dir)?;

            // Add some fake bundler-installed executables
            let rails_exe = bin_dir.join("rails");
            fs::write(&rails_exe, "#!/usr/bin/env ruby\n# Rails executable\n")?;

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let mut perms = fs::metadata(&rails_exe)?.permissions();
                perms.set_mode(0o755);
                fs::set_permissions(&rails_exe, perms)?;
            }
        }

        Ok(project_dir)
    }

    /// Create a nested directory structure for testing parent directory traversal.
    pub fn add_nested_structure(&self, levels: &[&str]) -> io::Result<PathBuf> {
        let mut current_path = self.root().to_path_buf();

        for level in levels {
            current_path = current_path.join(level);
        }

        fs::create_dir_all(&current_path)?;
        Ok(current_path)
    }

    /// Create a complex project with multiple nested Gemfiles for testing discovery logic.
    pub fn add_complex_project(&self) -> io::Result<(PathBuf, PathBuf, PathBuf)> {
        // Root project with Gemfile
        let root_project = self.add_bundler_project("main-project", true)?;

        // Nested subproject with its own Gemfile
        let subproject_dir = root_project.join("engines").join("my-engine");
        fs::create_dir_all(&subproject_dir)?;
        let sub_gemfile = subproject_dir.join("Gemfile");
        fs::write(
            &sub_gemfile,
            "source 'https://rubygems.org'\n\ngem 'engine-specific-gem'\n",
        )?;

        // Deep nested directory without Gemfile
        let deep_dir = subproject_dir
            .join("app")
            .join("controllers")
            .join("concerns");
        fs::create_dir_all(&deep_dir)?;

        Ok((root_project, subproject_dir, deep_dir))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_empty_sandbox() -> io::Result<()> {
        let sandbox = BundlerSandbox::new()?;
        assert!(sandbox.root().exists());
        assert!(sandbox.root().is_dir());
        Ok(())
    }

    #[test]
    fn add_dir_creates_directory() -> io::Result<()> {
        let sandbox = BundlerSandbox::new()?;
        let dir_path = sandbox.add_dir("my-app")?;

        assert!(dir_path.exists());
        assert!(dir_path.is_dir());
        assert_eq!(dir_path.file_name().unwrap(), "my-app");
        Ok(())
    }

    #[test]
    fn add_file_creates_file_with_contents() -> io::Result<()> {
        let sandbox = BundlerSandbox::new()?;
        let file_path = sandbox.add_file("test.txt", "Hello, World!")?;

        assert!(file_path.exists());
        assert!(file_path.is_file());
        let contents = fs::read_to_string(&file_path)?;
        assert_eq!(contents, "Hello, World!");
        Ok(())
    }

    #[test]
    fn add_gemfile_creates_gemfile_at_root() -> io::Result<()> {
        let sandbox = BundlerSandbox::new()?;
        let gemfile_path = sandbox.add_gemfile(None::<&str>, None)?;

        assert!(gemfile_path.exists());
        assert!(gemfile_path.is_file());
        assert_eq!(gemfile_path.file_name().unwrap(), "Gemfile");
        assert_eq!(gemfile_path.parent().unwrap(), sandbox.root());

        let contents = fs::read_to_string(&gemfile_path)?;
        assert!(contents.contains("source 'https://rubygems.org'"));
        Ok(())
    }

    #[test]
    fn add_gemfile_creates_gemfile_in_subdirectory() -> io::Result<()> {
        let sandbox = BundlerSandbox::new()?;
        let gemfile_path = sandbox.add_gemfile(Some("my-project"), Some("gem 'custom-gem'"))?;

        assert!(gemfile_path.exists());
        assert_eq!(gemfile_path.file_name().unwrap(), "Gemfile");

        let contents = fs::read_to_string(&gemfile_path)?;
        assert_eq!(contents, "gem 'custom-gem'");
        Ok(())
    }

    #[test]
    fn add_bundler_project_creates_basic_structure() -> io::Result<()> {
        let sandbox = BundlerSandbox::new()?;
        let project_dir = sandbox.add_bundler_project("test-app", false)?;

        // Check project directory
        assert!(project_dir.exists());
        assert!(project_dir.is_dir());

        // Check Gemfile
        let gemfile = project_dir.join("Gemfile");
        assert!(gemfile.exists());
        let contents = fs::read_to_string(&gemfile)?;
        assert!(contents.contains("test-app"));

        // Check .rb directory
        let rb_dir = project_dir.join(".rb");
        assert!(rb_dir.exists());
        assert!(rb_dir.is_dir());

        // Should not have vendor structure (not configured)
        let vendor_dir = rb_dir.join("vendor").join("bundler");
        assert!(!vendor_dir.exists());

        Ok(())
    }

    #[test]
    fn add_bundler_project_creates_configured_structure() -> io::Result<()> {
        let sandbox = BundlerSandbox::new()?;
        let project_dir = sandbox.add_bundler_project("configured-app", true)?;

        // Check basic structure
        let gemfile = project_dir.join("Gemfile");
        let rb_dir = project_dir.join(".rb");
        assert!(gemfile.exists());
        assert!(rb_dir.exists());

        // Check vendor/bundler structure
        let vendor_dir = rb_dir.join("vendor").join("bundler");
        let bin_dir = vendor_dir.join("bin");
        assert!(vendor_dir.exists());
        assert!(bin_dir.exists());

        // Check executable
        let rails_exe = bin_dir.join("rails");
        assert!(rails_exe.exists());

        Ok(())
    }

    #[test]
    fn add_nested_structure_creates_deep_directories() -> io::Result<()> {
        let sandbox = BundlerSandbox::new()?;
        let deep_dir = sandbox.add_nested_structure(&["app", "controllers", "api", "v1"])?;

        assert!(deep_dir.exists());
        assert!(deep_dir.is_dir());
        assert!(deep_dir.ends_with("app/controllers/api/v1"));

        Ok(())
    }

    #[test]
    fn add_complex_project_creates_nested_gemfiles() -> io::Result<()> {
        let sandbox = BundlerSandbox::new()?;
        let (root_project, subproject_dir, deep_dir) = sandbox.add_complex_project()?;

        // Check root project
        assert!(root_project.join("Gemfile").exists());
        assert!(
            root_project
                .join(".rb")
                .join("vendor")
                .join("bundler")
                .exists()
        );

        // Check subproject
        assert!(subproject_dir.join("Gemfile").exists());

        // Check deep directory
        assert!(deep_dir.exists());
        assert!(!deep_dir.join("Gemfile").exists());

        Ok(())
    }
}
