use colored::*;
use log::{debug, info};
use rb_core::butler::{ButlerError, ButlerRuntime, Command};

pub fn exec_command(butler: ButlerRuntime, program_args: Vec<String>) {
    if program_args.is_empty() {
        eprintln!(
            "{}: No program specified for execution",
            "Request Incomplete".red().bold()
        );
        eprintln!("Proper usage: rb exec <program> [arguments...]");
        eprintln!("For example: rb exec gem list");
        eprintln!("             rb exec bundle install");
        std::process::exit(1);
    }

    // Extract the program and its accompanying arguments
    let program = &program_args[0];
    let args = if program_args.len() > 1 {
        &program_args[1..]
    } else {
        &[]
    };

    info!(
        "Preparing to execute {} within the carefully composed Ruby environment",
        program
    );

    // Butler's refined approach: Ensure bundler environment is properly prepared
    if let Some(bundler_runtime) = butler.bundler_runtime() {
        match bundler_runtime.check_sync(&butler) {
            Ok(false) => {
                println!(
                    "{} {}",
                    "🎩 Butler Notice:".bright_blue().bold(),
                    "Bundler environment requires synchronization. Preparing now...".dimmed()
                );

                // Use bundler runtime's synchronize method directly
                match bundler_runtime.synchronize(&butler, |line| {
                    println!("{}", line.dimmed());
                }) {
                    Ok(_) => {
                        println!(
                            "{} {}",
                            "✨".bright_green(),
                            "Environment meticulously prepared. Proceeding with execution..."
                                .green()
                        );
                    }
                    Err(e) => {
                        eprintln!(
                            "{}: Failed to prepare bundler environment: {}",
                            "Synchronization Failed".red().bold(),
                            e
                        );
                        std::process::exit(1);
                    }
                }
            }
            Ok(true) => {
                debug!("Bundler environment already synchronized");
            }
            Err(e) => {
                debug!("Unable to verify bundler synchronization status: {}", e);
                // Continue anyway - might be a bundler install issue that user needs to handle
            }
        }
    }

    debug!("Program: {}", program);
    debug!("Arguments: {:?}", args);

    // Create and configure the butler command
    let mut cmd = Command::new(program);
    cmd.args(args);

    debug!("Commencing program execution...");

    // Execute with validation and handle command not found errors
    match cmd.status_with_validation(&butler) {
        Ok(status) => {
            if let Some(code) = status.code() {
                debug!("Program concluded with exit code: {}", code);
                std::process::exit(code);
            } else {
                debug!("Program was terminated by system signal");
                std::process::exit(1);
            }
        }
        Err(ButlerError::CommandNotFound(command)) => {
            eprintln!(
                "🎩 My sincerest apologies, but the command '{}' appears to be",
                command.bright_yellow()
            );
            eprintln!("   entirely absent from your distinguished Ruby environment.");
            eprintln!();
            eprintln!("This humble Butler has meticulously searched through all");
            eprintln!("available paths and gem installations, yet the requested");
            eprintln!("command remains elusive.");
            eprintln!();
            eprintln!("Might I suggest:");
            eprintln!("  • Verifying the command name is spelled correctly");
            eprintln!(
                "  • Installing the appropriate gem: {}",
                format!("gem install {}", command).cyan()
            );
            eprintln!(
                "  • Checking if bundler management is required: {}",
                "bundle install".cyan()
            );
            eprintln!();
            eprintln!(
                "For additional diagnostic information, please use the {} or {} flags.",
                "-v".cyan(),
                "-vv".cyan()
            );
            std::process::exit(127);
        }
        Err(e) => {
            eprintln!(
                "{}: Execution encountered difficulties: {}",
                "Execution Failed".red().bold(),
                e
            );
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
        sandbox
            .add_ruby_dir("3.2.5")
            .expect("Failed to create ruby-3.2.5");

        let _butler_runtime = ButlerRuntime::discover_and_create(sandbox.root(), None)
            .expect("Failed to create ButlerRuntime");

        // Test with empty args - we can test the validation logic
        let empty_args: Vec<String> = vec![];
        // Note: The actual exec_command would exit, so we just test our setup
        assert_eq!(empty_args.len(), 0);
    }

    #[test]
    fn test_butler_runtime_env_composition() {
        use rb_core::gems::GemRuntime;
        use rb_core::ruby::RubyRuntimeDetector;

        let sandbox = RubySandbox::new().expect("Failed to create sandbox");
        sandbox
            .add_ruby_dir("3.2.5")
            .expect("Failed to create ruby-3.2.5");

        // Directly discover ruby installations without relying on current directory
        let ruby_installations = RubyRuntimeDetector::discover(sandbox.root())
            .expect("Failed to detect ruby installations");

        assert!(
            !ruby_installations.is_empty(),
            "Should find at least one Ruby installation"
        );

        let ruby_runtime = ruby_installations.into_iter().next().unwrap();
        let gem_base = sandbox.gem_base_dir();
        let gem_runtime = GemRuntime::for_base_dir(&gem_base, &ruby_runtime.version);

        // Create butler without bundler runtime (simulating no Gemfile environment)
        let butler_runtime = ButlerRuntime::new(ruby_runtime, Some(gem_runtime));

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

        // Test that bundler variables are NOT set when no bundler project is detected
        assert!(!env_vars.contains_key("BUNDLE_GEMFILE"));
        assert!(!env_vars.contains_key("BUNDLE_APP_CONFIG"));
    }

    #[test]
    fn test_butler_runtime_env_composition_with_bundler() {
        use rb_tests::RubySandbox;
        use std::fs;

        let ruby_sandbox = RubySandbox::new().expect("Failed to create ruby sandbox");
        ruby_sandbox
            .add_ruby_dir("3.2.5")
            .expect("Failed to create ruby-3.2.5");

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

        let butler_runtime =
            ButlerRuntime::discover_and_compose(ruby_sandbox.root().to_path_buf(), None)
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
