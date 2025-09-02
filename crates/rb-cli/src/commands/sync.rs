use log::debug;
use rb_core::butler::ButlerRuntime;
use rb_core::bundler::SyncResult;

pub fn sync_command(
    rubies_dir: Option<std::path::PathBuf>,
    requested_ruby_version: Option<String>,
    gem_home: Option<std::path::PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    debug!("Starting sync command");
    
    // Resolve search directory
    let search_dir = crate::resolve_search_dir(rubies_dir);
    
    // Discover and compose the butler runtime with optional custom gem base
    let runtime = ButlerRuntime::discover_and_compose_with_gem_base(
        search_dir, 
        requested_ruby_version, 
        gem_home
    )?;
    
    // Check if bundler runtime is available
    let bundler_runtime = match runtime.bundler_runtime() {
        Some(bundler) => bundler,
        None => {
            println!("⚠️  Bundler Environment Not Detected");
            println!();
            println!("No Gemfile found in the current directory or its ancestors.");
            println!("The sync command requires a bundler-managed project to operate.");
            println!();
            println!("To create a new bundler project:");
            println!("  • Create a Gemfile with: echo 'source \"https://rubygems.org\"' > Gemfile");
            println!("  • Then run: rb sync");
            return Err("No bundler environment detected".into());
        }
    };
    
    println!("🔄 Synchronizing Bundler Environment");
    println!();
    println!("📂 Project: {}", bundler_runtime.root.display());
    println!("📄 Gemfile: {}", bundler_runtime.gemfile_path().display());
    println!("📦 Vendor:  {}", bundler_runtime.vendor_dir().display());
    println!();
    
    // Perform synchronization
    match bundler_runtime.synchronize(&runtime, |line| {
        println!("{}", line);
    }) {
        Ok(SyncResult::AlreadySynced) => {
            println!("✅ Environment Already Synchronized");
            println!();
            println!("Your bundler environment is meticulously prepared and ready for distinguished service.");
            println!("All dependencies are satisfied and properly installed.");
        }
        Ok(SyncResult::Synchronized) => {
            println!();
            println!("✅ Environment Successfully Synchronized");
            println!();
            println!("Your bundler environment has been meticulously prepared with all required dependencies.");
            println!("The installation is complete and ready for distinguished service.");
        }
        Err(e) => {
            println!();
            println!("❌ Synchronization Failed");
            println!();
            
            let error_msg = e.to_string();
            
            // Check for common error patterns and provide helpful guidance
            if error_msg.contains("extconf.rb failed") || 
               error_msg.contains("native extension") ||
               error_msg.contains("development tools") ||
               error_msg.contains("compiler failed") ||
               error_msg.contains("Makefile") {
                
                println!("🔧 Native Extension Compilation Failed");
                println!();
                println!("Some gems in your Gemfile require native extensions to be compiled.");
                println!("This requires development tools to be installed on your system.");
                println!();
                println!("📋 Required Development Tools:");
                println!("  • Build essentials (gcc, make, etc.)");
                println!("  • Ruby development headers");
                println!("  • Platform-specific libraries");
                println!();
                println!("🚀 Installation Commands:");
                println!("  Ubuntu/Debian: sudo apt-get install build-essential ruby-dev");
                println!("  CentOS/RHEL:   sudo yum groupinstall 'Development Tools' && sudo yum install ruby-devel");
                println!("  Alpine Linux:  sudo apk add build-base ruby-dev");
                println!("  macOS:         xcode-select --install");
                println!();
                println!("💡 Alternative Solutions:");
                println!("  • Use pre-compiled gem versions if available");
                println!("  • Consider using --platform ruby to force source compilation");
                println!("  • Use Docker with a development-ready base image");
                
            } else if error_msg.contains("not found") && error_msg.contains("bundler") {
                println!("📦 Bundler Not Found");
                println!();
                println!("The bundler executable is not available in your Ruby environment.");
                println!();
                println!("🚀 Installation:");
                println!("  gem install bundler");
                
            } else if error_msg.contains("permission") || error_msg.contains("Permission") {
                println!("🔒 Permission Denied");
                println!();
                println!("Unable to write to the gem installation directory.");
                println!();
                println!("💡 Solutions:");
                println!("  • Ensure write permissions to the vendor directory");
                println!("  • Check file system permissions");
                println!("  • Consider using a user-specific gem directory");
                
            } else {
                println!("⚠️  Bundle Installation Error");
                println!();
                println!("Details: {}", error_msg);
            }
            
            println!();
            println!("🔍 For detailed error information, run:");
            println!("  rb exec bundle install --verbose");
            
            return Err(e.into());
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rb_tests::BundlerSandbox;
    
    #[test]
    fn test_sync_command_with_no_gemfile() -> Result<(), Box<dyn std::error::Error>> {
        let sandbox = BundlerSandbox::new()?;
        let project_dir = sandbox.add_dir("no_gemfile_project")?;
        
        // Change to project directory
        let original_dir = std::env::current_dir()?;
        std::env::set_current_dir(&project_dir)?;
        
        // Should return error when no bundler environment detected
        let result = sync_command(None, None, None);
        
        // Restore directory (ignore errors in case directory was deleted)
        let _ = std::env::set_current_dir(original_dir);
        
        // Should return error when no bundler environment detected
        match result {
            Ok(()) => panic!("Expected error when no Gemfile found, but command succeeded"),
            Err(e) => {
                let error_msg = e.to_string();
                if error_msg.contains("No bundler environment detected") {
                    Ok(()) // This is what we expect - proper error for no bundler
                } else if error_msg.contains("Os { code: 2") || 
                         error_msg.contains("No such file or directory") ||
                         error_msg.contains("Bundler executable not found") {
                    Ok(()) // Also acceptable in test environment without bundler
                } else {
                    Err(e) // Unexpected error
                }
            }
        }
    }
}
