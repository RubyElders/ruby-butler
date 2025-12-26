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

    // ButlerRuntime should fail with RubiesDirectoryNotFound when directory doesn't exist
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

    // ButlerRuntime should fail with RubiesDirectoryNotFound when directory doesn't exist
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

    // Create ruby executable
    let ruby_exe = ruby_dir.join("bin").join("ruby");
    std::fs::write(&ruby_exe, "#!/bin/sh\necho ruby")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&ruby_exe, std::fs::Permissions::from_mode(0o755))?;
    }

    // Create a bundler project with Gemfile
    let bundler_sandbox = BundlerSandbox::new()?;
    bundler_sandbox.add_gemfile(
        None::<&str>,
        Some("source 'https://rubygems.org'\ngem 'rake'"),
    )?;

    // Discover with skip_bundler = false - should detect bundler
    let runtime_with_bundler = ButlerRuntime::discover_and_compose_with_current_dir(
        sandbox.root().to_path_buf(),
        Some("3.3.0".to_string()),
        None,
        false,
        bundler_sandbox.root().to_path_buf(),
    )?;
    assert!(
        runtime_with_bundler.bundler_runtime().is_some(),
        "Bundler should be detected when skip_bundler=false"
    );

    // Discover with skip_bundler = true - should NOT detect bundler
    let runtime_without_bundler = ButlerRuntime::discover_and_compose_with_current_dir(
        sandbox.root().to_path_buf(),
        Some("3.3.0".to_string()),
        None,
        true,
        bundler_sandbox.root().to_path_buf(),
    )?;
    assert!(
        runtime_without_bundler.bundler_runtime().is_none(),
        "Bundler should NOT be detected when skip_bundler=true"
    );

    Ok(())
}

/// Test that bundler isolation excludes user gems
#[test]
fn test_bundler_isolation_excludes_user_gems() -> Result<(), Box<dyn std::error::Error>> {
    use rb_tests::BundlerSandbox;

    let ruby_sandbox = RubySandbox::new()?;
    let bundler_sandbox = BundlerSandbox::new()?;

    // Create Ruby installation with executable
    let ruby_dir = ruby_sandbox.add_ruby_dir("3.3.7")?;
    std::fs::create_dir_all(ruby_dir.join("bin"))?;
    let ruby_exe = ruby_dir.join("bin").join("ruby");
    std::fs::write(&ruby_exe, "#!/bin/sh\necho ruby")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&ruby_exe, std::fs::Permissions::from_mode(0o755))?;
    }

    let rubies = RubyRuntimeDetector::discover(ruby_sandbox.root())?;
    let ruby = &rubies[0];

    // Create gem runtime (user gems)
    let _gem_runtime = GemRuntime::for_base_dir(&ruby_sandbox.gem_base_dir(), &ruby.version);

    // Create bundler project
    let project_dir = bundler_sandbox.add_bundler_project("isolated-app", true)?;
    let _bundler_runtime = rb_core::BundlerRuntime::new(&project_dir, ruby.version.clone());

    // Discover runtime WITH bundler context
    let runtime_with_bundler = ButlerRuntime::discover_and_compose_with_current_dir(
        ruby_sandbox.root().to_path_buf(),
        None,
        None,
        false, // don't skip bundler
        project_dir.clone(),
    )?;

    // CRITICAL: When bundler context is present, gem_runtime should be None (isolation)
    assert!(
        runtime_with_bundler.gem_runtime().is_none(),
        "User gem runtime should NOT be available in bundler context (isolation)"
    );

    // Bundler runtime SHOULD be present
    assert!(
        runtime_with_bundler.bundler_runtime().is_some(),
        "Bundler runtime should be detected"
    );

    // bin_dirs should NOT include user gem bin (only bundler bin + ruby bin)
    let bin_dirs = runtime_with_bundler.bin_dirs();
    let has_bundler_bin = bin_dirs
        .iter()
        .any(|p| p.to_string_lossy().contains("bundler"));
    let has_user_gem_bin = bin_dirs
        .iter()
        .any(|p| p.to_string_lossy().contains(".gem") && !p.to_string_lossy().contains("bundler"));

    assert!(has_bundler_bin, "Should have bundler bin directory");
    assert!(
        !has_user_gem_bin,
        "Should NOT have user gem bin directory (isolation)"
    );

    // gem_dirs should NOT include user gem home (only bundler gems + ruby lib)
    let gem_dirs = runtime_with_bundler.gem_dirs();
    let has_bundler_gems = gem_dirs
        .iter()
        .any(|p| p.to_string_lossy().contains("bundler"));
    let has_user_gems = gem_dirs
        .iter()
        .any(|p| p.to_string_lossy().contains(".gem") && !p.to_string_lossy().contains("bundler"));

    assert!(has_bundler_gems, "Should have bundler gem directory");
    assert!(
        !has_user_gems,
        "Should NOT have user gem directory (isolation)"
    );

    Ok(())
}

/// Test that with --no-bundler flag, user gems ARE available
#[test]
fn test_no_bundler_flag_restores_user_gems() -> Result<(), Box<dyn std::error::Error>> {
    use rb_tests::BundlerSandbox;

    let ruby_sandbox = RubySandbox::new()?;
    let bundler_sandbox = BundlerSandbox::new()?;

    // Create Ruby installation with executable
    let ruby_dir = ruby_sandbox.add_ruby_dir("3.3.7")?;
    std::fs::create_dir_all(ruby_dir.join("bin"))?;
    let ruby_exe = ruby_dir.join("bin").join("ruby");
    std::fs::write(&ruby_exe, "#!/bin/sh\necho ruby")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&ruby_exe, std::fs::Permissions::from_mode(0o755))?;
    }

    let rubies = RubyRuntimeDetector::discover(ruby_sandbox.root())?;
    let _ruby = &rubies[0];

    // Create bundler project
    let project_dir = bundler_sandbox.add_bundler_project("user-gems-app", true)?;

    // Discover runtime WITH --no-bundler flag
    let runtime_no_bundler = ButlerRuntime::discover_and_compose_with_current_dir(
        ruby_sandbox.root().to_path_buf(),
        None,
        None,
        true, // skip bundler (--no-bundler)
        project_dir.clone(),
    )?;

    // Bundler should NOT be detected
    assert!(
        runtime_no_bundler.bundler_runtime().is_none(),
        "Bundler should be skipped with --no-bundler flag"
    );

    // User gem runtime SHOULD be available now
    assert!(
        runtime_no_bundler.gem_runtime().is_some(),
        "User gem runtime should be available with --no-bundler"
    );

    // bin_dirs should include user gem bin (NOT bundler bin)
    let bin_dirs = runtime_no_bundler.bin_dirs();
    let has_bundler_bin = bin_dirs
        .iter()
        .any(|p| p.to_string_lossy().contains("bundler"));
    let has_user_gem_bin = bin_dirs
        .iter()
        .any(|p| p.to_string_lossy().contains(".gem"));

    assert!(!has_bundler_bin, "Should NOT have bundler bin directory");
    assert!(has_user_gem_bin, "Should have user gem bin directory");

    // gem_dirs should include user gem home (NOT bundler gems)
    let gem_dirs = runtime_no_bundler.gem_dirs();
    let has_bundler_gems = gem_dirs
        .iter()
        .any(|p| p.to_string_lossy().contains("bundler"));
    let has_user_gems = gem_dirs
        .iter()
        .any(|p| p.to_string_lossy().contains(".gem"));

    assert!(!has_bundler_gems, "Should NOT have bundler gem directory");
    assert!(has_user_gems, "Should have user gem directory");

    Ok(())
}

/// Test that bundler bin paths include Ruby version directory
#[test]
fn test_bundler_bin_paths_include_ruby_version() -> Result<(), Box<dyn std::error::Error>> {
    use rb_tests::BundlerSandbox;

    let ruby_sandbox = RubySandbox::new()?;
    let bundler_sandbox = BundlerSandbox::new()?;

    // Create Ruby installation with executable
    let ruby_dir = ruby_sandbox.add_ruby_dir("3.3.7")?;
    std::fs::create_dir_all(ruby_dir.join("bin"))?;
    let ruby_exe = ruby_dir.join("bin").join("ruby");
    std::fs::write(&ruby_exe, "#!/bin/sh\necho ruby")?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&ruby_exe, std::fs::Permissions::from_mode(0o755))?;
    }

    let rubies = RubyRuntimeDetector::discover(ruby_sandbox.root())?;
    let _ruby = &rubies[0];

    // Create bundler project
    let project_dir = bundler_sandbox.add_bundler_project("versioned-bins", true)?;

    // Add .ruby-version file so bundler knows which Ruby to use
    std::fs::write(project_dir.join(".ruby-version"), "3.3.7")?;

    // Create the ruby-minor-versioned bundler bin directory (uses X.Y.0)
    let bundler_ruby_bin = project_dir
        .join(".rb")
        .join("vendor")
        .join("bundler")
        .join("ruby")
        .join("3.3.0")
        .join("bin");
    std::fs::create_dir_all(&bundler_ruby_bin)?;

    // Discover runtime with bundler
    let runtime = ButlerRuntime::discover_and_compose_with_current_dir(
        ruby_sandbox.root().to_path_buf(),
        None,
        None,
        false,
        project_dir.clone(),
    )?;

    // Check that bundler bin path includes ruby version
    let bin_dirs = runtime.bin_dirs();
    let bundler_bin = bin_dirs
        .iter()
        .find(|p| p.to_string_lossy().contains("bundler"))
        .expect("Should have bundler bin directory");

    // Should be: .rb/vendor/bundler/ruby/3.3.0/bin
    let path_str = bundler_bin.to_string_lossy();
    assert!(
        path_str.contains("ruby") && path_str.contains("3.3.0") && path_str.contains("bin"),
        "Bundler bin should include Ruby version path: got {}",
        bundler_bin.display()
    );

    Ok(())
}
