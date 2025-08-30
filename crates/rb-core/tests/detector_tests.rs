use rb_core::RubyRuntimeDetector;
use rb_tests::RubySandbox;

#[test]
fn discovers_only_ruby_xyz_directories() -> std::io::Result<()> {
    let sb = RubySandbox::new()?;
    // Valid
    sb.add_ruby_dir("3.1.2")?;
    sb.add_ruby_dir("3.3.0")?;
    // Invalid names (ignored)
    sb.add_dir("jruby-9.4.5.0")?;
    sb.add_dir("ruby-3.2.0-rc1")?;
    sb.add_file("ruby-3.2.2", b"not a dir")?; // file, not dir

    let rubies = RubyRuntimeDetector::discover(sb.root())?;

    let names: Vec<_> = rubies.iter().map(|r| r.version_name()).collect();
    assert_eq!(names, vec!["CRuby-3.3.0", "CRuby-3.1.2"]); // sorted DESC

    // sanity on fields
    let r = rubies.iter().find(|r| r.version_name() == "CRuby-3.3.0").unwrap();
    assert!(r.bin_dir().ends_with("ruby-3.3.0/bin"));
    Ok(())
}

#[test]
fn latest_picks_highest_semver() -> std::io::Result<()> {
    let sb = RubySandbox::new()?;
    for n in ["3.0.6", "3.2.4", "3.3.1"] {
        sb.add_ruby_dir(n)?;
    }
    let rubies = RubyRuntimeDetector::discover(sb.root())?;
    let latest = RubyRuntimeDetector::latest(&rubies).expect("some ruby");
    assert_eq!(latest.version_name(), "CRuby-3.3.1");
    Ok(())
}

#[test]
fn ruby_executable_path_is_platform_correct() -> std::io::Result<()> {
    // Create one ruby to inspect
    let sb = RubySandbox::new()?;
    sb.add_ruby_dir("3.2.1")?;
    let rubies = RubyRuntimeDetector::discover(sb.root())?;
    let r = rubies.into_iter().find(|r| r.version_name() == "CRuby-3.2.1").unwrap();

    let exe = r.ruby_executable_path();
    if cfg!(windows) {
        assert!(exe.ends_with("ruby.exe"));
    } else {
        assert!(exe.ends_with("ruby"));
    }

    Ok(())
}
