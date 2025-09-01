use rb_core::ruby::RubyRuntimeDetector;
use rb_core::butler::ButlerRuntime;
use rb_tests::RubySandbox;
use semver::Version;

#[test]
fn test_ruby_detector_integration() {
    let sandbox = RubySandbox::new().expect("Failed to create sandbox");
    
    // Add various Ruby versions
    sandbox.add_ruby_dir("3.1.0").expect("Failed to create ruby-3.1.0");
    sandbox.add_ruby_dir("3.2.5").expect("Failed to create ruby-3.2.5");
    sandbox.add_ruby_dir("3.3.1").expect("Failed to create ruby-3.3.1");
    
    let runtimes = RubyRuntimeDetector::discover(sandbox.root())
        .expect("Failed to discover Ruby installations");
    
    assert_eq!(runtimes.len(), 3);
    
    // Verify sorting (highest version first)
    assert_eq!(runtimes[0].version, Version::parse("3.3.1").unwrap());
    assert_eq!(runtimes[1].version, Version::parse("3.2.5").unwrap());
    assert_eq!(runtimes[2].version, Version::parse("3.1.0").unwrap());
    
    // Verify all runtimes have proper structure
    for runtime in &runtimes {
        assert!(runtime.root.exists());
        // Note: Ruby executable path is expected but might not exist in sandbox
        // The RubyRuntimeDetector only validates directory structure, not executables
    }
}

#[test]
fn test_create_ruby_context_integration() {
    let sandbox = RubySandbox::new().expect("Failed to create sandbox");
    sandbox.add_ruby_dir("3.2.5").expect("Failed to create ruby-3.2.5");
    
    let butler_runtime = ButlerRuntime::discover_and_compose(sandbox.root().to_path_buf(), None)
        .expect("Failed to discover context");
    
    assert!(butler_runtime.has_ruby_environment());
    
    // Verify the runtime
    let runtime = butler_runtime.selected_ruby();
    assert_eq!(runtime.version, Version::parse("3.2.5").unwrap());
    assert!(runtime.root.exists());
    
    // Verify the environment variables
    let env_vars = butler_runtime.env_vars(std::env::var("PATH").ok());
    
    assert!(env_vars.contains_key("PATH"));
    assert!(env_vars.contains_key("GEM_HOME"));
    assert!(env_vars.contains_key("GEM_PATH"));
    
    // GEM_PATH should include GEM_HOME (per chruby pattern, may include additional paths)
    let gem_home = env_vars.get("GEM_HOME").unwrap();
    let gem_path = env_vars.get("GEM_PATH").unwrap();
    assert!(gem_path.contains(gem_home), "GEM_PATH should include GEM_HOME");
}

#[test]
fn test_resolve_search_dir_integration() {
    let sandbox = RubySandbox::new().expect("Failed to create sandbox");
    sandbox.add_ruby_dir("3.2.5").expect("Failed to create ruby-3.2.5");
    
    // Test with explicit root directory
    let butler_runtime = ButlerRuntime::discover_and_compose(sandbox.root().to_path_buf(), None)
        .expect("Failed to discover context");
    assert_eq!(butler_runtime.rubies_dir(), sandbox.root());
    
    // Test that resolved directory makes sense
    assert!(butler_runtime.rubies_dir().is_absolute());
    assert_eq!(butler_runtime.rubies_dir(), sandbox.root());
}
