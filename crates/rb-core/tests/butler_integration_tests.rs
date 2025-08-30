mod common;
use std::io;
use rb_core::butler::ButlerRuntime;
use rb_core::ruby::{RubyRuntime, RubyType, RubyRuntimeDetector};
use rb_core::gems::GemRuntime;
use semver::Version;

#[test]
fn test_butler_runtime_with_only_ruby() -> io::Result<()> {
    let sandbox = common::RubySandbox::new()?;
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
    let sandbox = common::RubySandbox::new()?;
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
    let sandbox = common::RubySandbox::new()?;
    
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
    let sandbox = common::RubySandbox::new()?;
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
        "/nonexistent/ruby"
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
