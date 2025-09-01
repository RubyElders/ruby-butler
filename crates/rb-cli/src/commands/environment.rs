use colored::*;
use rb_core::ruby::RubyType;
use rb_core::bundler::BundlerRuntime;
use rb_core::butler::ButlerRuntime;
use log::{debug, info};

pub fn environment_command(butler_runtime: &ButlerRuntime) {
    info!("Presenting current Ruby environment from the working directory");
    present_current_environment(butler_runtime);
}

fn present_current_environment(butler_runtime: &ButlerRuntime) {
    println!("{}", format!("🌍 Your Current Ruby Environment").bold());
    println!();

    let current_dir = butler_runtime.current_dir();
    debug!("Current working directory: {}", current_dir.display());
    debug!("Using discovered bundler runtime from context");

    // Use bundler runtime from butler runtime
    let bundler_runtime = butler_runtime.bundler_runtime();

    // Use Ruby selection from butler runtime
    let ruby = butler_runtime.selected_ruby();

    // Get gem runtime from butler runtime
    let gem_runtime = butler_runtime.gem_runtime();

    // Present the environment
    present_environment_details(ruby, gem_runtime, bundler_runtime, butler_runtime);
}

fn present_environment_details(
    ruby: &rb_core::ruby::RubyRuntime,
    gem_runtime: Option<&rb_core::gems::GemRuntime>,
    bundler_runtime: Option<&BundlerRuntime>,
    butler: &ButlerRuntime,
) {
    let label_width = ["Installation", "Gem home", "Gem libraries", "Executable paths", "Bundler root", "Gemfile", "Vendor directory", "App config", "Synchronized"].iter().map(|s| s.len()).max().unwrap_or(15);

    // Present Ruby Environment
    let ruby_type = match ruby.kind {
        RubyType::CRuby => "💎 CRuby".green(),
    };
    println!("{} {}", ruby_type, format!("({})", ruby.version).cyan());
    
    println!("    {:<width$}: {}", 
        "Installation".bright_blue().bold(), 
        ruby.root.display().to_string().bright_black(),
        width = label_width
    );

    if let Some(gem_rt) = gem_runtime {
        println!("    {:<width$}: {}", 
            "Gem home".bright_blue().bold(), 
            gem_rt.gem_home.display().to_string().bright_black(),
            width = label_width
        );
    } else {
        println!("    {:<width$}: {}", 
            "Gem home".bright_blue().bold(), 
            "Not available".yellow(),
            width = label_width
        );
    }

    let gem_dirs = butler.gem_dirs();
    if !gem_dirs.is_empty() {
        let gem_paths = gem_dirs.iter().map(|d| d.display().to_string()).collect::<Vec<_>>();
        println!("    {:<width$}: {}", 
            "Gem libraries".bright_blue().bold(), 
            gem_paths.join(", ").bright_black(),
            width = label_width
        );
    }

    let bin_dirs = butler.bin_dirs();
    if !bin_dirs.is_empty() {
        let bin_paths = bin_dirs.iter().map(|d| d.display().to_string()).collect::<Vec<_>>();
        println!("    {:<width$}: {}", 
            "Executable paths".bright_blue().bold(), 
            bin_paths.join(", ").bright_black(),
            width = label_width
        );
    }

    // Present Bundler Environment (if detected)
    if let Some(bundler) = bundler_runtime {
        println!();
        println!("{}", "📦 Bundler Environment".green().bold());
        
        println!("    {:<width$}: {}", 
            "Bundler root".bright_blue().bold(), 
            bundler.root.display().to_string().bright_black(),
            width = label_width
        );
        
        println!("    {:<width$}: {}", 
            "Gemfile".bright_blue().bold(), 
            bundler.gemfile_path().display().to_string().bright_black(),
            width = label_width
        );
        
        println!("    {:<width$}: {}", 
            "App config".bright_blue().bold(), 
            bundler.app_config_dir().display().to_string().bright_black(),
            width = label_width
        );
        
        println!("    {:<width$}: {}", 
            "Vendor directory".bright_blue().bold(), 
            bundler.vendor_dir().display().to_string().bright_black(),
            width = label_width
        );

        if let Some(version) = bundler.ruby_version() {
            println!("    {:<width$}: {}", 
                "Required Ruby".bright_blue().bold(), 
                format!("{}", version).bright_black(),
                width = label_width
            );
        }

        let configured = if bundler.is_configured() { "Yes".green() } else { "No".yellow() };
        println!("    {:<width$}: {}", 
            "Configured".bright_blue().bold(), 
            configured,
            width = label_width
        );

        // Check synchronization status
        let sync_status = if !bundler.is_configured() {
            "⚠️  Out of sync".yellow()
        } else {
            match bundler.check_sync(butler) {
                Ok(true) => "✅ Synchronized".green(),
                Ok(false) => "⚠️  Out of sync".yellow(),
                Err(_) => "❓ Unknown".bright_black(),
            }
        };
        println!("    {:<width$}: {}", 
            "Synchronized".bright_blue().bold(), 
            sync_status,
            width = label_width
        );
    } else {
        println!();
        println!("{}", "📦 Bundler Environment".bright_black());
        println!("    {}", "Bundler environment not detected".bright_black());
    }

    // Present environment summary
    println!();
    println!("{}", "🎯 Environment Summary".green().bold());
    
    let ruby_version_text = format!("{} {}", ruby_type_as_str(&ruby.kind), ruby.version);
    println!("    {:<width$}: {}", 
        "Active Ruby".bright_blue().bold(), 
        ruby_version_text.bright_black(),
        width = label_width
    );
    
    if let Some(bundler) = bundler_runtime {
        let project_name = bundler.root.file_name().unwrap_or_default().to_string_lossy();
        println!("    {:<width$}: {}", 
            "Bundler project".bright_blue().bold(), 
            project_name.bright_black(),
            width = label_width
        );
        
        if let Some(req_version) = bundler.ruby_version() {
            let matches = if ruby.version == req_version { "✅ Matches".green() } else { "⚠️  Mismatch".yellow() };
            println!("    {:<width$}: {}", 
                "Version match".bright_blue().bold(), 
                matches,
                width = label_width
            );
        }
    }

    println!();
    println!("{}", "Environment ready for distinguished Ruby development.".bright_black());
}

fn ruby_type_as_str(ruby_type: &RubyType) -> &'static str {
    match ruby_type {
        RubyType::CRuby => "CRuby",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rb_core::butler::ButlerRuntime;
    use rb_tests::RubySandbox;

    #[test]
    fn environment_command_exists() {
        // Test that the function exists and can be called with a Ruby installation
        let sandbox = RubySandbox::new().expect("Failed to create sandbox");
        sandbox.add_ruby_dir("3.2.5").expect("Failed to create ruby-3.2.5");
        
        // Create a basic ButlerRuntime for testing
        let butler_runtime = ButlerRuntime::discover_and_compose(sandbox.root().to_path_buf(), None)
            .expect("Failed to create butler runtime with test Ruby");
        
        // This will handle the environment presentation gracefully
        environment_command(&butler_runtime);
    }

    #[test]
    fn present_environment_details_handles_no_bundler() -> std::io::Result<()> {
        use rb_tests::RubySandbox;
        
        let ruby_sandbox = RubySandbox::new()?;
        let ruby_dir = ruby_sandbox.add_ruby_dir("3.2.5")?;
        let ruby = rb_core::ruby::RubyRuntime::new(
            rb_core::ruby::RubyType::CRuby, 
            semver::Version::parse("3.2.5").unwrap(), 
            &ruby_dir
        );
        
        let gem_runtime = ruby.infer_gem_runtime().ok();
        let butler = ButlerRuntime::new(ruby.clone(), gem_runtime.clone());
        
        // Test with no bundler environment
        present_environment_details(&ruby, gem_runtime.as_ref(), None, &butler);
        
        Ok(())
    }

    #[test]
    fn present_environment_details_with_bundler() -> std::io::Result<()> {
        use rb_tests::{RubySandbox, BundlerSandbox};
        
        let ruby_sandbox = RubySandbox::new()?;
        let ruby_dir = ruby_sandbox.add_ruby_dir("3.2.5")?;
        let ruby = rb_core::ruby::RubyRuntime::new(
            rb_core::ruby::RubyType::CRuby, 
            semver::Version::parse("3.2.5").unwrap(), 
            &ruby_dir
        );
        
        let bundler_sandbox = BundlerSandbox::new()?;
        let project_dir = bundler_sandbox.add_bundler_project("test-app", true)?;
        let bundler_runtime = BundlerRuntime::new(&project_dir);
        
        let gem_runtime = ruby.infer_gem_runtime().ok();
        let butler = ButlerRuntime::new(ruby.clone(), gem_runtime.clone());
        
        // Test with bundler environment
        present_environment_details(&ruby, gem_runtime.as_ref(), Some(&bundler_runtime), &butler);
        
        Ok(())
    }
}
