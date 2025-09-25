use rb_core::{BundlerRuntime, BundlerRuntimeDetector};
use rb_tests::BundlerSandbox;
use semver::Version;
use std::io;

#[test]
fn bundler_detector_integrates_with_bundler_sandbox() -> io::Result<()> {
    let sandbox = BundlerSandbox::new()?;

    // Create a configured bundler project
    let project_dir = sandbox.add_bundler_project("my-rails-app", true)?;

    // Detector should find the bundler runtime
    let result = BundlerRuntimeDetector::discover(&project_dir)?;
    assert!(result.is_some());

    let bundler_runtime = result.unwrap();
    assert_eq!(bundler_runtime.root, project_dir);
    assert!(bundler_runtime.is_configured());

    Ok(())
}

#[test]
fn bundler_detector_finds_gemfile_from_nested_directory() -> io::Result<()> {
    let sandbox = BundlerSandbox::new()?;

    // Create complex project structure
    let (root_project, _subproject, deep_dir) = sandbox.add_complex_project()?;

    // Detector should find the main project Gemfile when searching from deep directory
    let result = BundlerRuntimeDetector::discover(&deep_dir)?;
    assert!(result.is_some());

    let bundler_runtime = result.unwrap();
    // Should NOT find the root project, but rather the subproject
    assert_ne!(bundler_runtime.root, root_project);
    assert!(bundler_runtime.root.ends_with("engines/my-engine"));

    Ok(())
}

#[test]
fn bundler_detector_returns_none_for_non_bundler_directory() -> io::Result<()> {
    let sandbox = BundlerSandbox::new()?;

    // Create a directory without Gemfile
    let non_bundler_dir = sandbox.add_dir("just-a-directory")?;

    // Detector should return None
    let result = BundlerRuntimeDetector::discover(&non_bundler_dir)?;
    assert!(result.is_none());

    Ok(())
}

#[test]
fn bundler_runtime_provides_correct_paths_for_configured_project() -> io::Result<()> {
    let sandbox = BundlerSandbox::new()?;

    // Create configured project
    let project_dir = sandbox.add_bundler_project("configured-app", true)?;
    let bundler_runtime = BundlerRuntime::new(&project_dir);

    // Check all paths
    assert_eq!(bundler_runtime.gemfile_path(), project_dir.join("Gemfile"));
    assert_eq!(bundler_runtime.app_config_dir(), project_dir.join(".rb"));
    assert_eq!(
        bundler_runtime.vendor_dir(),
        project_dir.join(".rb").join("vendor").join("bundler")
    );
    assert_eq!(
        bundler_runtime.bin_dir(),
        bundler_runtime.vendor_dir().join("bin")
    );

    // Should be configured since we created vendor structure
    assert!(bundler_runtime.is_configured());

    Ok(())
}

#[test]
fn bundler_runtime_not_configured_for_basic_project() -> io::Result<()> {
    let sandbox = BundlerSandbox::new()?;

    // Create basic project (not configured)
    let project_dir = sandbox.add_bundler_project("basic-app", false)?;
    let bundler_runtime = BundlerRuntime::new(&project_dir);

    // Should not be configured since no vendor structure exists
    assert!(!bundler_runtime.is_configured());

    Ok(())
}

#[test]
fn bundler_runtime_detects_ruby_version_from_ruby_version_file() -> io::Result<()> {
    let sandbox = BundlerSandbox::new()?;
    let project_dir = sandbox.add_bundler_project("version-project", false)?;

    // Add .ruby-version file
    sandbox.add_file(
        format!(
            "{}/{}",
            project_dir.file_name().unwrap().to_str().unwrap(),
            ".ruby-version"
        ),
        "3.2.5",
    )?;

    let bundler_runtime = BundlerRuntime::new(&project_dir);
    assert_eq!(
        bundler_runtime.ruby_version(),
        Some(Version::parse("3.2.5").unwrap())
    );

    Ok(())
}

#[test]
fn bundler_runtime_detects_ruby_version_from_custom_gemfile() -> io::Result<()> {
    let sandbox = BundlerSandbox::new()?;
    let project_dir = sandbox.add_dir("custom-gemfile-app")?;

    // Create custom Gemfile with specific ruby version
    let gemfile_content = r#"source 'https://rubygems.org'

ruby '3.1.2'

gem 'rails', '~> 7.0'
gem 'pg', '~> 1.4'
gem 'puma', '~> 5.6'
"#;
    sandbox.add_file(
        format!(
            "{}/Gemfile",
            project_dir.file_name().unwrap().to_str().unwrap()
        ),
        gemfile_content,
    )?;

    let bundler_runtime = BundlerRuntime::new(&project_dir);
    assert_eq!(
        bundler_runtime.ruby_version(),
        Some(Version::parse("3.1.2").unwrap())
    );

    Ok(())
}

#[test]
fn bundler_detector_preserves_ruby_version_discovery() -> io::Result<()> {
    let sandbox = BundlerSandbox::new()?;
    let project_dir = sandbox.add_dir("detector-version-app")?;

    // Create Gemfile with ruby version
    let gemfile_content = r#"source 'https://rubygems.org'

ruby "3.3.1"

gem 'sinatra'
gem 'rackup'
"#;
    sandbox.add_file(
        format!(
            "{}/Gemfile",
            project_dir.file_name().unwrap().to_str().unwrap()
        ),
        gemfile_content,
    )?;

    // Detector should find the project and preserve Ruby version
    let result = BundlerRuntimeDetector::discover(&project_dir)?;
    assert!(result.is_some());

    let bundler_runtime = result.unwrap();
    assert_eq!(
        bundler_runtime.ruby_version(),
        Some(Version::parse("3.3.1").unwrap())
    );

    Ok(())
}

#[test]
fn bundler_runtime_handles_mixed_version_sources() -> io::Result<()> {
    let sandbox = BundlerSandbox::new()?;
    let project_dir = sandbox.add_dir("mixed-version-app")?;

    // Create Gemfile with one version
    let gemfile_content = r#"source 'https://rubygems.org'

ruby '3.0.6'

gem 'rails'
"#;
    sandbox.add_file(
        format!(
            "{}/Gemfile",
            project_dir.file_name().unwrap().to_str().unwrap()
        ),
        gemfile_content,
    )?;

    // Add .ruby-version with different version (should take precedence)
    sandbox.add_file(
        format!(
            "{}/{}",
            project_dir.file_name().unwrap().to_str().unwrap(),
            ".ruby-version"
        ),
        "3.2.3",
    )?;

    let bundler_runtime = BundlerRuntime::new(&project_dir);
    // Should prefer .ruby-version over Gemfile
    assert_eq!(
        bundler_runtime.ruby_version(),
        Some(Version::parse("3.2.3").unwrap())
    );

    Ok(())
}
