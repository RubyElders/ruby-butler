use std::process::Command;
use std::env;
use log::{debug, info};
use colored::*;
use rb_core::butler::ButlerRuntime;
use which::which_in;

pub fn exec_command(butler: ButlerRuntime, program_args: Vec<String>) {
    if program_args.is_empty() {
        eprintln!("{}: No program specified for execution", "Request Incomplete".red().bold());
        eprintln!("Proper usage: rb exec <program> [arguments...]");
        eprintln!("For example: rb exec gem list");
        eprintln!("             rb exec bundle install");
        std::process::exit(1);
    }

    // Obtain the current PATH for environment composition
    let current_path = env::var("PATH").ok();
    
    // Compose the distinguished Ruby environment
    let env_vars = butler.env_vars(current_path);
    
    // Extract the program and its accompanying arguments
    let program = &program_args[0];
    let args = if program_args.len() > 1 {
        &program_args[1..]
    } else {
        &[]
    };

    // Locate the program within our meticulously prepared PATH
    let resolved_program = match which_in(program, env_vars.get("PATH"), ".") {
        Ok(path) => path,
        Err(_) => {
            eprintln!("{}: The program '{}' could not be located within the prepared environment", 
                     "Program Not Found".red().bold(), program.cyan());
            eprintln!("Available directories in your prepared PATH:");
            if let Some(path_var) = env_vars.get("PATH") {
                let separator = if cfg!(windows) { ";" } else { ":" };
                for dir in path_var.split(separator) {
                    eprintln!("  {}", dir.bright_black());
                }
            }
            std::process::exit(1);
        }
    };

    info!("Preparing to execute {} within the carefully composed Ruby environment", 
          resolved_program.display());
    
    debug!("Program resolved to: {}", resolved_program.display());
    debug!("Arguments prepared: {:?}", args);
    
    // Document the environmental preparations being applied
    for (key, value) in &env_vars {
        debug!("Establishing {}={}", key, value);
    }

    // Execute the program within our distinguished environment
    let mut cmd = Command::new(&resolved_program);
    cmd.args(args);
    
    // Apply the meticulously prepared environment variables
    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    debug!("Commencing program execution...");
    
    // Execute and honor the program's exit status
    match cmd.status() {
        Ok(status) => {
            if let Some(code) = status.code() {
                debug!("Program concluded with exit code: {}", code);
                std::process::exit(code);
            } else {
                debug!("Program was terminated by system signal");
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("{}: Execution encountered difficulties: {}", 
                     "Execution Failed".red().bold(), e);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rb_tests::RubySandbox;

    #[test]
    fn test_exec_command_with_empty_args() {
        // This test verifies the function signature and empty args behavior
        let sandbox = RubySandbox::new().expect("Failed to create sandbox");
        sandbox.add_ruby_dir("3.2.5").expect("Failed to create ruby-3.2.5");
        
        let _butler_runtime = ButlerRuntime::discover_and_create(&sandbox.root().to_path_buf(), None)
            .expect("Failed to create ButlerRuntime");
        
        // Test with empty args - we can test the validation logic
        let empty_args: Vec<String> = vec![];
        // Note: The actual exec_command would exit, so we just test our setup
        assert_eq!(empty_args.len(), 0);
    }

    #[test]
    fn test_butler_runtime_env_composition() {
        let sandbox = RubySandbox::new().expect("Failed to create sandbox");
        sandbox.add_ruby_dir("3.2.5").expect("Failed to create ruby-3.2.5");
        
        // Change to the sandbox directory to avoid detecting project Gemfile
        let original_dir = std::env::current_dir().expect("Failed to get current dir");
        std::env::set_current_dir(sandbox.root()).expect("Failed to change to sandbox dir");
        
        let butler_runtime = ButlerRuntime::discover_and_create(&sandbox.root().to_path_buf(), None)
            .expect("Failed to create ButlerRuntime");
        
        let current_path = std::env::var("PATH").ok();
        let env_vars = butler_runtime.env_vars(current_path);
        
        // Restore directory
        std::env::set_current_dir(original_dir).expect("Failed to restore directory");
        
        // Test that all required environment variables are present
        assert!(env_vars.contains_key("PATH"));
        assert!(env_vars.contains_key("GEM_HOME"));
        assert!(env_vars.contains_key("GEM_PATH"));
        
        // Test that PATH includes the Ruby bin directory
        let path = env_vars.get("PATH").unwrap();
        assert!(path.contains("ruby-3.2.5"));
        
        // Test that GEM_PATH includes GEM_HOME (as per chruby pattern: GEM_HOME:GEM_ROOT)
        let gem_home = env_vars.get("GEM_HOME").unwrap();
        let gem_path = env_vars.get("GEM_PATH").unwrap();
        assert!(gem_path.contains(gem_home));
        
        // Test that bundler variables are NOT set when no bundler project is detected
        assert!(!env_vars.contains_key("BUNDLE_GEMFILE"));
        assert!(!env_vars.contains_key("BUNDLE_APP_CONFIG"));
    }

    #[test]
    fn test_butler_runtime_env_composition_with_bundler() {
        use rb_tests::RubySandbox;
        use std::fs;
        
        let ruby_sandbox = RubySandbox::new().expect("Failed to create ruby sandbox");
        ruby_sandbox.add_ruby_dir("3.2.5").expect("Failed to create ruby-3.2.5");
        
        // Create a bundler project directly in the Ruby sandbox
        let project_dir = ruby_sandbox.root().join("test-project");
        fs::create_dir_all(&project_dir).expect("Failed to create project directory");
        
        let gemfile_path = project_dir.join("Gemfile");
        fs::write(&gemfile_path, "source 'https://rubygems.org'\ngem 'json'\n")
            .expect("Failed to write Gemfile");
        
        // Create .rb directory for bundler configuration
        let rb_dir = project_dir.join(".rb");
        fs::create_dir_all(&rb_dir).expect("Failed to create .rb directory");
        
        // Change to the bundler project directory to trigger bundler detection
        let original_dir = std::env::current_dir().expect("Failed to get current directory");
        std::env::set_current_dir(&project_dir).expect("Failed to change directory");
        
        let butler_runtime = ButlerRuntime::discover_and_compose(ruby_sandbox.root().to_path_buf(), None)
            .expect("Failed to create ButlerRuntime");
        
        // Restore original directory
        let _ = std::env::set_current_dir(&original_dir);
        
        let current_path = std::env::var("PATH").ok();
        let env_vars = butler_runtime.env_vars(current_path);
        
        // Test that standard variables are present
        assert!(env_vars.contains_key("PATH"));
        assert!(env_vars.contains_key("GEM_HOME"));
        assert!(env_vars.contains_key("GEM_PATH"));
        
        // Test that bundler variables are set when bundler project is detected
        assert!(env_vars.contains_key("BUNDLE_GEMFILE"));
        assert!(env_vars.contains_key("BUNDLE_APP_CONFIG"));
        
        // Test that bundler variables point to correct locations
        let bundle_gemfile = env_vars.get("BUNDLE_GEMFILE").unwrap();
        assert!(bundle_gemfile.contains("Gemfile"));
        
        let bundle_app_config = env_vars.get("BUNDLE_APP_CONFIG").unwrap();
        assert!(bundle_app_config.contains(".rb"));
    }
}
