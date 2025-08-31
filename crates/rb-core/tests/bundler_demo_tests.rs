use rb_core::{BundlerRuntime, BundlerRuntimeDetector};
use rb_tests::BundlerSandbox;
use semver::Version;
use std::io;

#[test]
fn demo_complete_bundler_workflow_with_ruby_version() -> io::Result<()> {
    println!("\n🎩 Demonstrating Ruby Butler's Bundler Environment with Ruby Version Detection");
    
    let sandbox = BundlerSandbox::new()?;
    
    // Create a Rails-like project with Ruby version specification
    let _project_dir = sandbox.add_dir("my-rails-app")?;
    
    // Add .ruby-version file
    sandbox.add_file(
        "my-rails-app/.ruby-version",
        "3.2.5"
    )?;
    
    // Add realistic Gemfile
    let gemfile_content = r#"source 'https://rubygems.org'

ruby '3.2.5'

gem 'rails', '~> 7.1.0'
gem 'pg', '~> 1.4'
gem 'puma', '~> 6.4'
gem 'redis', '~> 5.0'
gem 'bootsnap', require: false
gem 'sassc-rails'
gem 'image_processing', '~> 1.2'

group :development, :test do
  gem 'debug', platforms: %i[ mri mingw x64_mingw ]
  gem 'rspec-rails'
  gem 'factory_bot_rails'
end

group :development do
  gem 'web-console'
  gem 'spring'
  gem 'spring-watcher-listen'
end
"#;
    sandbox.add_file("my-rails-app/Gemfile", gemfile_content)?;
    
    // Configure bundler (simulate bundle install)
    let _configured_project = sandbox.add_bundler_project("my-rails-app", true)?;
    
    println!("  📁 Created Rails project structure");
    
    // Test discovery from nested directory (like in real development)
    let deep_dir = sandbox.add_nested_structure(&["my-rails-app", "app", "controllers", "api", "v1"])?;
    println!("  🔍 Searching from deep directory: {}", deep_dir.display());
    
    // Discover bundler environment
    let result = BundlerRuntimeDetector::discover(&deep_dir)?;
    assert!(result.is_some(), "Should find bundler project from nested directory");
    
    let bundler = result.unwrap();
    let project_name = bundler.root.file_name().unwrap().to_string_lossy();
    println!("  ✅ Found bundler project: {}", project_name);
    println!("  📂 Project root: {}", bundler.root.display());
    
    // Check Ruby version detection
    assert_eq!(bundler.ruby_version(), Some(Version::parse("3.2.5").unwrap()));
    println!("  🔸 Detected Ruby version: {}", bundler.ruby_version().as_ref().unwrap());
    
    // Check configuration
    assert!(bundler.is_configured(), "Project should be configured");
    println!("  ⚙️  Bundler configured with vendor directory");
    println!("  📦 Vendor dir: {}", bundler.vendor_dir().display());
    println!("  🔧 Bin dir: {}", bundler.bin_dir().display());
    
    // Check paths
    assert!(bundler.gemfile_path().exists());
    println!("  📋 Gemfile: {}", bundler.gemfile_path().display());
    
    println!("  🎉 Complete bundler environment successfully detected and configured!");
    println!("      Ready for Ruby runtime matching and environment composition.");
    
    Ok(())
}

#[test]
fn demo_version_precedence_and_fallbacks() -> io::Result<()> {
    println!("\n🎯 Demonstrating Ruby Version Detection Precedence");
    
    let sandbox = BundlerSandbox::new()?;
    
    // Scenario 1: .ruby-version takes precedence
    let project1 = sandbox.add_dir("precedence-test")?;
    sandbox.add_file("precedence-test/.ruby-version", "3.3.0")?;
    sandbox.add_file("precedence-test/Gemfile", r#"source 'https://rubygems.org'
ruby '3.1.0'
gem 'rails'
"#)?;
    
    let bundler1 = BundlerRuntime::new(&project1);
    assert_eq!(bundler1.ruby_version(), Some(Version::parse("3.3.0").unwrap()));
    println!("  ✅ .ruby-version (3.3.0) takes precedence over Gemfile (3.1.0)");
    
    // Scenario 2: Fallback to Gemfile
    let project2 = sandbox.add_dir("gemfile-only")?;
    sandbox.add_file("gemfile-only/Gemfile", r#"source 'https://rubygems.org'
ruby "3.2.1"
gem 'sinatra'
"#)?;
    
    let bundler2 = BundlerRuntime::new(&project2);
    assert_eq!(bundler2.ruby_version(), Some(Version::parse("3.2.1").unwrap()));
    println!("  ✅ Falls back to Gemfile ruby declaration (3.2.1)");
    
    // Scenario 3: No version specified
    let project3 = sandbox.add_dir("no-version")?;
    sandbox.add_file("no-version/Gemfile", r#"source 'https://rubygems.org'
gem 'rack'
"#)?;
    
    let bundler3 = BundlerRuntime::new(&project3);
    assert_eq!(bundler3.ruby_version(), None);
    println!("  ✅ Gracefully handles projects without Ruby version specification");
    
    println!("  🎯 Version detection precedence working as designed!");
    
    Ok(())
}
