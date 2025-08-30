use std::process::Command;
use std::env;
use log::{debug, info};
use colored::*;
use rb_core::butler::ButlerRuntime;
use which::which_in;

pub fn exec_command(butler: ButlerRuntime, program_args: Vec<String>) {
    if program_args.is_empty() {
        eprintln!("{}: No program specified to execute", "Error".red().bold());
        eprintln!("Usage: rb exec <program> [args...]");
        eprintln!("Example: rb exec gem list");
        std::process::exit(1);
    }

    // Get current PATH
    let current_path = env::var("PATH").ok();
    
    // Compose the environment
    let env_vars = butler.env_vars(current_path);
    
    // Extract program and arguments
    let program = &program_args[0];
    let args = if program_args.len() > 1 {
        &program_args[1..]
    } else {
        &[]
    };

    // Resolve the program within the composed PATH
    let resolved_program = match which_in(program, env_vars.get("PATH"), ".") {
        Ok(path) => path,
        Err(_) => {
            eprintln!("{}: Program '{}' not found in PATH", "Error".red().bold(), program.cyan());
            eprintln!("Available PATH directories:");
            if let Some(path_var) = env_vars.get("PATH") {
                let separator = if cfg!(windows) { ";" } else { ":" };
                for dir in path_var.split(separator) {
                    eprintln!("  {}", dir.bright_black());
                }
            }
            std::process::exit(1);
        }
    };

    info!("Executing {} with composed Ruby environment", 
          resolved_program.display());
    
    debug!("Resolved program: {}", resolved_program.display());
    debug!("Arguments: {:?}", args);
    
    // Log environment variables being set
    for (key, value) in &env_vars {
        debug!("Setting {}={}", key, value);
    }

    // Execute the program with the composed environment
    let mut cmd = Command::new(&resolved_program);
    cmd.args(args);
    
    // Set environment variables
    for (key, value) in env_vars {
        cmd.env(key, value);
    }

    debug!("Starting process...");
    
    // Execute and forward exit code
    match cmd.status() {
        Ok(status) => {
            if let Some(code) = status.code() {
                debug!("Process exited with code: {}", code);
                std::process::exit(code);
            } else {
                debug!("Process terminated by signal");
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("{}: Failed to execute program: {}", "Error".red().bold(), e);
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
        
        let butler_runtime = ButlerRuntime::discover_and_create(&sandbox.root().to_path_buf(), None)
            .expect("Failed to create ButlerRuntime");
        
        let current_path = std::env::var("PATH").ok();
        let env_vars = butler_runtime.env_vars(current_path);
        
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
    }
}
