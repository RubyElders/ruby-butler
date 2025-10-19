use rb_core::butler::{ButlerError, ButlerRuntime};
use rb_core::gems::GemRuntime;
use rb_core::ruby::{RubyRuntime, RubyRuntimeDetector, RubyType};
use rb_tests::RubySandbox;
use semver::Version;
use std::io;
use std::path::PathBuf;

#[test]
fn test_butler_runtime_with_only_ruby() -> io::Result<()> {
    let sandbox = RubySandbox::new()?;
    let ruby_dir = sandbox.add_ruby_dir("3.1.0")?;
    std::fs::create_dir_all(ruby_dir.join("bin"))?;

    let rubies = RubyRuntimeDetector::discover(sandbox.root())?;
    assert_eq!(rubies.len(), 1);
    let ruby = &rubies[0];

    let butler = ButlerRuntime::new(ruby.clone(), None);

    // Test bin_dirs
    let bin_dirs = butler.bin_dirs();
    assert_eq!(bin_dirs.len(), 1);
    assert!(bin_dirs[0].ends_with("bin"));
    assert!(bin_dirs[0].to_string_lossy().contains("ruby-3.1.0"));

    // Test gem_dirs
    let gem_dirs = butler.gem_dirs();
    assert_eq!(gem_dirs.len(), 1);
    assert!(gem_dirs[0].to_string_lossy().contains("3.1.0"));

    // Test gem_home should be None when no GemRuntime
    assert_eq!(butler.gem_home(), None);

    // Test build_path
    let path = butler.build_path(Some("/usr/bin:/bin".to_string()));
    assert!(path.contains(&bin_dirs[0].display().to_string()));
    assert!(path.contains("/usr/bin:/bin"));

    Ok(())
}

#[test]
fn test_butler_runtime_with_ruby_and_gem() -> io::Result<()> {
    let sandbox = RubySandbox::new()?;
    let ruby_dir = sandbox.add_ruby_dir("3.2.1")?;
    std::fs::create_dir_all(ruby_dir.join("bin"))?;

    let rubies = RubyRuntimeDetector::discover(sandbox.root())?;
    let ruby = &rubies[0];

    // Create a GemRuntime
    let gem_base = sandbox.root().join(".gem");
    std::fs::create_dir_all(&gem_base)?;
    let gem_runtime = GemRuntime::for_base_dir(&gem_base, &ruby.version);

    let butler = ButlerRuntime::new(ruby.clone(), Some(gem_runtime.clone()));

    // Test bin_dirs - should have both ruby and gem bin dirs
    let bin_dirs = butler.bin_dirs();
    assert_eq!(bin_dirs.len(), 2);

    // First should be gem bin dir (higher priority)
    assert!(bin_dirs[0].to_string_lossy().contains(".gem"));
    assert!(bin_dirs[0].ends_with("bin"));

    // Second should be ruby bin dir
    assert!(bin_dirs[1].to_string_lossy().contains("ruby-3.2.1"));
    assert!(bin_dirs[1].ends_with("bin"));

    // Test gem_dirs - should have both ruby and gem dirs
    let gem_dirs = butler.gem_dirs();
    assert_eq!(gem_dirs.len(), 2);

    // Test gem_home should return the gem runtime's gem_dir
    let gem_home = butler.gem_home();
    assert!(gem_home.is_some());
    assert_eq!(gem_home.unwrap(), gem_runtime.gem_home);

    // Test build_path with multiple bin dirs
    let path = butler.build_path(None);
    assert!(path.contains(&bin_dirs[0].display().to_string()));
    assert!(path.contains(&bin_dirs[1].display().to_string()));

    Ok(())
}

#[test]
fn test_butler_runtime_with_multiple_rubies() -> io::Result<()> {
    let sandbox = RubySandbox::new()?;

    // Add multiple Ruby versions
    let ruby_dir_1 = sandbox.add_ruby_dir("3.1.0")?;
    let ruby_dir_2 = sandbox.add_ruby_dir("3.2.1")?;
    std::fs::create_dir_all(ruby_dir_1.join("bin"))?;
    std::fs::create_dir_all(ruby_dir_2.join("bin"))?;

    let rubies = RubyRuntimeDetector::discover(sandbox.root())?;
    assert_eq!(rubies.len(), 2);

    // Use the latest Ruby version
    let latest = RubyRuntimeDetector::latest(&rubies).expect("should find latest");
    assert_eq!(latest.version.to_string(), "3.2.1");

    let butler = ButlerRuntime::new(latest.clone(), None);

    // Verify we're using the correct Ruby version
    let bin_dirs = butler.bin_dirs();
    assert_eq!(bin_dirs.len(), 1);
    assert!(bin_dirs[0].to_string_lossy().contains("ruby-3.2.1"));

    Ok(())
}

#[test]
fn test_butler_runtime_path_building_platform_specific() -> io::Result<()> {
    let sandbox = RubySandbox::new()?;
    let ruby_dir = sandbox.add_ruby_dir("3.0.0")?;
    std::fs::create_dir_all(ruby_dir.join("bin"))?;

    let rubies = RubyRuntimeDetector::discover(sandbox.root())?;
    let ruby = &rubies[0];

    let gem_base = sandbox.root().join(".gem");
    let gem_runtime = GemRuntime::for_base_dir(&gem_base, &ruby.version);

    let butler = ButlerRuntime::new(ruby.clone(), Some(gem_runtime));

    // Test path building uses correct separator
    let path = butler.build_path(Some("/existing/path".to_string()));

    if cfg!(windows) {
        assert!(path.contains(";"));
    } else {
        assert!(path.contains(":"));
    }

    // Should contain both bin directories
    let bin_dirs = butler.bin_dirs();
    for bin_dir in bin_dirs {
        assert!(path.contains(&bin_dir.display().to_string()));
    }

    Ok(())
}

#[test]
fn test_butler_runtime_empty_gem_runtime() -> io::Result<()> {
    // Test with a ruby that has no gem directories
    let ruby = RubyRuntime::new(
        RubyType::CRuby,
        Version::parse("3.0.0").unwrap(),
        "/nonexistent/ruby",
    );

    let butler = ButlerRuntime::new(ruby, None);

    // Should still work, just with ruby bin/gem dirs
    let bin_dirs = butler.bin_dirs();
    assert_eq!(bin_dirs.len(), 1);
    assert!(bin_dirs[0].ends_with("bin"));

    let gem_dirs = butler.gem_dirs();
    assert_eq!(gem_dirs.len(), 1);

    assert_eq!(butler.gem_home(), None);

    Ok(())
}

#[test]
fn test_butler_runtime_discover_nonexistent_directory() {
    let nonexistent_path = PathBuf::from("completely_nonexistent_directory_butler_test");

    let result = ButlerRuntime::discover_and_compose(nonexistent_path.clone(), None);

    assert!(result.is_err());
    match result.unwrap_err() {
        ButlerError::RubiesDirectoryNotFound(path) => {
            assert_eq!(path, nonexistent_path);
        }
        _ => panic!("Expected RubiesDirectoryNotFound error"),
    }
}

#[test]
fn test_butler_runtime_discover_with_gem_base_nonexistent_directory() {
    let nonexistent_path = PathBuf::from("completely_nonexistent_directory_butler_gem_test");
    let gem_base = Some(PathBuf::from("/tmp/gems"));

    let result = ButlerRuntime::discover_and_compose_with_gem_base(
        nonexistent_path.clone(),
        None,
        gem_base,
        false,
    );

    assert!(result.is_err());
    match result.unwrap_err() {
        ButlerError::RubiesDirectoryNotFound(path) => {
            assert_eq!(path, nonexistent_path);
        }
        _ => panic!("Expected RubiesDirectoryNotFound error"),
    }
}

#[test]
fn test_butler_runtime_skip_bundler_flag() -> Result<(), Box<dyn std::error::Error>> {
    use rb_tests::BundlerSandbox;

    let sandbox = RubySandbox::new()?;
    let ruby_dir = sandbox.add_ruby_dir("3.3.0")?;
    std::fs::create_dir_all(ruby_dir.join("bin"))?;

    // Create a bundler project with Gemfile
    let bundler_sandbox = BundlerSandbox::new()?;
    bundler_sandbox.add_gemfile(
        None::<&str>,
        Some("source 'https://rubygems.org'\ngem 'rake'"),
    )?;

    // Change to the bundler project directory
    let original_dir = std::env::current_dir()?;
    std::env::set_current_dir(bundler_sandbox.root())?;

    // Discover with skip_bundler = false - should detect bundler
    let runtime_with_bundler = ButlerRuntime::discover_and_compose_with_gem_base(
        sandbox.root().to_path_buf(),
        Some("3.3.0".to_string()),
        None,
        false,
    )?;
    assert!(
        runtime_with_bundler.bundler_runtime().is_some(),
        "Bundler should be detected when skip_bundler=false"
    );

    // Discover with skip_bundler = true - should NOT detect bundler
    let runtime_without_bundler = ButlerRuntime::discover_and_compose_with_gem_base(
        sandbox.root().to_path_buf(),
        Some("3.3.0".to_string()),
        None,
        true,
    )?;
    assert!(
        runtime_without_bundler.bundler_runtime().is_none(),
        "Bundler should NOT be detected when skip_bundler=true"
    );

    // Restore original directory
    std::env::set_current_dir(original_dir)?;

    Ok(())
}
