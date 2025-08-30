mod common;
use std::io;
use rb_core::butler::ButlerRuntime;
use rb_core::ruby::RubyRuntimeDetector;

#[test]
fn test_butler_env_with_detector_and_latest_ruby() -> io::Result<()> {
    // Setup sandbox and add one ruby version
    let sandbox = common::RubySandbox::new()?;
    let ruby_dir = sandbox.add_ruby_dir("3.2.1")?;
    std::fs::create_dir_all(ruby_dir.join("bin"))?;

    // Discover rubies using detector
    let rubies = RubyRuntimeDetector::discover(sandbox.root())?;
    assert_eq!(rubies.len(), 1);
    let latest = RubyRuntimeDetector::latest(&rubies).expect("should find latest ruby");
    assert_eq!(latest.version.to_string(), "3.2.1");

    // Compose ButlerRuntime with only RubyRuntime
    let butler = ButlerRuntime::new(vec![Box::new(latest.clone())]);
    let envs = butler.env_vars(Some("/usr/bin".to_string()));

    // Assert only PATH is set and correct
    let mut path_val = None;
    for (k, v) in envs {
        if k == "PATH" {
            path_val = Some(v);
        }
    }
    let path_val = path_val.expect("PATH must be set");
    assert!(path_val.contains(&latest.bin_dir().display().to_string()));
    assert!(path_val.contains("/usr/bin"));

    Ok(())
}
