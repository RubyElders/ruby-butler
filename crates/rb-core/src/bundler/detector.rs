use log::{debug, info};
use std::path::Path;

use super::BundlerRuntime;

pub struct BundlerRuntimeDetector;

impl BundlerRuntimeDetector {
    /// Discover a BundlerRuntime by searching for Gemfile in the current directory
    /// and walking up the directory tree until one is found or we reach the root.
    pub fn discover(start_dir: &Path) -> std::io::Result<Option<BundlerRuntime>> {
        debug!(
            "Starting Bundler discovery from directory: {}",
            start_dir.display()
        );

        let mut current_dir = start_dir.to_path_buf();

        loop {
            debug!("Checking directory for Gemfile: {}", current_dir.display());
            let gemfile_path = current_dir.join("Gemfile");

            if gemfile_path.exists() && gemfile_path.is_file() {
                info!("Found Gemfile at: {}", gemfile_path.display());
                let bundler_runtime = BundlerRuntime::new(&current_dir);
                debug!("Created BundlerRuntime for root: {}", current_dir.display());
                return Ok(Some(bundler_runtime));
            } else {
                debug!("No Gemfile found in: {}", current_dir.display());
            }

            // Move up one directory
            match current_dir.parent() {
                Some(parent) => {
                    current_dir = parent.to_path_buf();
                    debug!("Moving up to parent directory: {}", current_dir.display());
                }
                None => {
                    debug!("Reached filesystem root, no Gemfile found");
                    break;
                }
            }
        }

        info!(
            "No Bundler project found starting from: {}",
            start_dir.display()
        );
        Ok(None)
    }

    /// Convenience method to discover from current working directory
    pub fn discover_from_cwd() -> std::io::Result<Option<BundlerRuntime>> {
        let cwd = std::env::current_dir()?;
        debug!(
            "Discovering Bundler runtime from current working directory: {}",
            cwd.display()
        );
        Self::discover(&cwd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rb_tests::BundlerSandbox;
    use semver;
    use std::io;

    #[test]
    fn discover_finds_gemfile_in_current_directory() -> io::Result<()> {
        let sandbox = BundlerSandbox::new()?;
        let project_dir = sandbox.add_bundler_project("current-dir-app", false)?;

        let result = BundlerRuntimeDetector::discover(&project_dir)?;

        assert!(result.is_some());
        let bundler_runtime = result.unwrap();
        assert_eq!(bundler_runtime.root, project_dir);
        assert_eq!(bundler_runtime.gemfile_path(), project_dir.join("Gemfile"));

        Ok(())
    }

    #[test]
    fn discover_finds_gemfile_in_parent_directory() -> io::Result<()> {
        let sandbox = BundlerSandbox::new()?;
        let project_dir = sandbox.add_bundler_project("parent-dir-app", true)?;
        let sub_dir = sandbox.add_nested_structure(&[
            project_dir.file_name().unwrap().to_str().unwrap(),
            "app",
            "controllers",
        ])?;

        let result = BundlerRuntimeDetector::discover(&sub_dir)?;

        assert!(result.is_some());
        let bundler_runtime = result.unwrap();
        assert_eq!(bundler_runtime.root, project_dir);
        assert_eq!(bundler_runtime.gemfile_path(), project_dir.join("Gemfile"));

        Ok(())
    }

    #[test]
    fn discover_returns_none_when_no_gemfile_found() -> io::Result<()> {
        let sandbox = BundlerSandbox::new()?;
        let sub_dir = sandbox.add_dir("no-gemfile-here")?;

        let result = BundlerRuntimeDetector::discover(&sub_dir)?;

        assert!(result.is_none());

        Ok(())
    }

    #[test]
    fn discover_stops_at_first_gemfile_found() -> io::Result<()> {
        let sandbox = BundlerSandbox::new()?;

        // Create complex project with nested Gemfiles
        let (_root_project, subproject, deep_dir) = sandbox.add_complex_project()?;

        // Search from deep directory - should find subproject Gemfile, not root
        let result = BundlerRuntimeDetector::discover(&deep_dir)?;

        assert!(result.is_some());
        let bundler_runtime = result.unwrap();
        assert_eq!(bundler_runtime.root, subproject);
        assert_eq!(bundler_runtime.gemfile_path(), subproject.join("Gemfile"));

        Ok(())
    }

    #[test]
    fn discover_skips_directories_and_finds_parent_gemfile() -> io::Result<()> {
        let sandbox = BundlerSandbox::new()?;
        let project_dir = sandbox.add_bundler_project("deep-project", false)?;
        let deep_dir = sandbox.add_nested_structure(&[
            project_dir.file_name().unwrap().to_str().unwrap(),
            "lib",
            "my_gem",
            "deep",
            "nested",
        ])?;

        let result = BundlerRuntimeDetector::discover(&deep_dir)?;

        assert!(result.is_some());
        let bundler_runtime = result.unwrap();
        assert_eq!(bundler_runtime.root, project_dir);
        assert_eq!(bundler_runtime.gemfile_path(), project_dir.join("Gemfile"));

        Ok(())
    }

    #[test]
    fn discover_detects_ruby_version_from_project() -> io::Result<()> {
        let sandbox = BundlerSandbox::new()?;
        let project_dir = sandbox.add_dir("ruby-version-app")?;

        // Create Gemfile with ruby version
        let gemfile_content = r#"source 'https://rubygems.org'

ruby '3.2.1'

gem 'rails'
"#;
        sandbox.add_file(
            format!(
                "{}/Gemfile",
                project_dir.file_name().unwrap().to_str().unwrap()
            ),
            gemfile_content,
        )?;

        let result = BundlerRuntimeDetector::discover(&project_dir)?;

        assert!(result.is_some());
        let bundler_runtime = result.unwrap();
        assert_eq!(
            bundler_runtime.ruby_version(),
            Some(semver::Version::parse("3.2.1").unwrap())
        );

        Ok(())
    }
}
