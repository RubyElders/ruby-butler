use colored::*;
use log::{debug, info, warn};
use rb_core::bundler::BundlerRuntime;
use rb_core::butler::{ButlerError, ButlerRuntime};
use rb_core::project::{ProjectRuntime, RbprojectDetector};
use rb_core::ruby::RubyType;
use std::path::PathBuf;

pub fn environment_command(
    butler_runtime: &ButlerRuntime,
    project_file: Option<PathBuf>,
) -> Result<(), ButlerError> {
    info!("Presenting current Ruby environment from the working directory");
    present_current_environment(butler_runtime, project_file)
}

fn present_current_environment(
    butler_runtime: &ButlerRuntime,
    project_file: Option<PathBuf>,
) -> Result<(), ButlerError> {
    println!("{}", "🌍 Your Current Ruby Environment".to_string().bold());
    println!();

    let current_dir = butler_runtime.current_dir();
    debug!("Current working directory: {}", current_dir.display());
    debug!("Using discovered bundler runtime from context");

    // Use bundler runtime from butler runtime
    let bundler_runtime = butler_runtime.bundler_runtime();

    // Use Ruby selection from butler runtime
    let ruby = butler_runtime.selected_ruby()?;

    // Get gem runtime from butler runtime
    let gem_runtime = butler_runtime.gem_runtime();

    // Detect or load project runtime
    let project_runtime = if let Some(path) = project_file {
        // Use specified project file
        debug!(
            "Loading project config from specified path: {}",
            path.display()
        );
        match ProjectRuntime::from_file(&path) {
            Ok(project) => {
                debug!(
                    "Loaded {} with {} scripts",
                    project.config_filename,
                    project.scripts.len()
                );
                Some(project)
            }
            Err(e) => {
                warn!(
                    "Failed to load specified project config at {}: {}",
                    path.display(),
                    e
                );
                None
            }
        }
    } else {
        // Auto-detect project file
        RbprojectDetector::discover(current_dir)
            .ok()
            .flatten()
            .inspect(|project| {
                debug!(
                    "Discovered {} with {} scripts",
                    project.config_filename,
                    project.scripts.len()
                );
            })
    };

    // Present the environment
    present_environment_details(
        ruby,
        gem_runtime,
        bundler_runtime,
        project_runtime.as_ref(),
        butler_runtime,
    );

    Ok(())
}

fn present_environment_details(
    ruby: &rb_core::ruby::RubyRuntime,
    gem_runtime: Option<&rb_core::gems::GemRuntime>,
    bundler_runtime: Option<&BundlerRuntime>,
    project_runtime: Option<&ProjectRuntime>,
    butler: &ButlerRuntime,
) {
    let label_width = [
        "Installation",
        "Gem home",
        "Gem libraries",
        "Executable paths",
        "Bundler root",
        "Gemfile",
        "Vendor directory",
        "App config",
        "Synchronized",
    ]
    .iter()
    .map(|s| s.len())
    .max()
    .unwrap_or(15);

    // Present Ruby Environment
    let ruby_type = match ruby.kind {
        RubyType::CRuby => "💎 CRuby".green(),
    };
    println!("{} {}", ruby_type, format!("({})", ruby.version).cyan());

    println!(
        "    {:<width$}: {}",
        "Installation".bright_blue().bold(),
        ruby.root.display().to_string().bright_black(),
        width = label_width
    );

    if let Some(gem_rt) = gem_runtime {
        println!(
            "    {:<width$}: {}",
            "Gem home".bright_blue().bold(),
            gem_rt.gem_home.display().to_string().bright_black(),
            width = label_width
        );
    } else {
        println!(
            "    {:<width$}: {}",
            "Gem home".bright_blue().bold(),
            "Not available".yellow(),
            width = label_width
        );
    }

    let gem_dirs = butler.gem_dirs();
    if !gem_dirs.is_empty() {
        let gem_paths = gem_dirs
            .iter()
            .map(|d| d.display().to_string())
            .collect::<Vec<_>>();
        println!(
            "    {:<width$}: {}",
            "Gem libraries".bright_blue().bold(),
            gem_paths.join(", ").bright_black(),
            width = label_width
        );
    }

    let bin_dirs = butler.bin_dirs();
    if !bin_dirs.is_empty() {
        let bin_paths = bin_dirs
            .iter()
            .map(|d| d.display().to_string())
            .collect::<Vec<_>>();
        println!(
            "    {:<width$}: {}",
            "Executable paths".bright_blue().bold(),
            bin_paths.join(", ").bright_black(),
            width = label_width
        );
    }

    // Present Bundler Environment (if detected)
    if let Some(bundler) = bundler_runtime {
        println!();
        println!("{}", "📦 Bundler Environment".green().bold());

        println!(
            "    {:<width$}: {}",
            "Bundler root".bright_blue().bold(),
            bundler.root.display().to_string().bright_black(),
            width = label_width
        );

        println!(
            "    {:<width$}: {}",
            "Gemfile".bright_blue().bold(),
            bundler.gemfile_path().display().to_string().bright_black(),
            width = label_width
        );

        println!(
            "    {:<width$}: {}",
            "App config".bright_blue().bold(),
            bundler
                .app_config_dir()
                .display()
                .to_string()
                .bright_black(),
            width = label_width
        );

        println!(
            "    {:<width$}: {}",
            "Vendor directory".bright_blue().bold(),
            bundler.vendor_dir().display().to_string().bright_black(),
            width = label_width
        );

        if let Some(version) = bundler.ruby_version() {
            println!(
                "    {:<width$}: {}",
                "Required Ruby".bright_blue().bold(),
                format!("{}", version).bright_black(),
                width = label_width
            );
        }

        let configured = if bundler.is_configured() {
            "Yes".green()
        } else {
            "No".yellow()
        };
        println!(
            "    {:<width$}: {}",
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
        println!(
            "    {:<width$}: {}",
            "Synchronized".bright_blue().bold(),
            sync_status,
            width = label_width
        );
    } else {
        println!();
        println!("{}", "📦 Bundler Environment".bright_black());
        println!("    {}", "Bundler environment not detected".bright_black());
    }

    // Present Project Environment (if detected)
    if let Some(project) = project_runtime {
        println!();
        println!("{}", "📋 Project".green().bold());

        // Display project name if available
        if let Some(name) = &project.metadata.name {
            println!(
                "    {:<width$}: {}",
                "Name".bright_blue().bold(),
                name.bright_black(),
                width = label_width
            );
        }

        // Display project description if available
        if let Some(description) = &project.metadata.description {
            println!(
                "    {:<width$}: {}",
                "Description".bright_blue().bold(),
                description.bright_black(),
                width = label_width
            );
        }

        println!(
            "    {:<width$}: {}",
            "Project file".bright_blue().bold(),
            project
                .rbproject_path()
                .display()
                .to_string()
                .bright_black(),
            width = label_width
        );

        println!(
            "    {:<width$}: {}",
            "Scripts loaded".bright_blue().bold(),
            format!("{}", project.scripts.len()).bright_black(),
            width = label_width
        );

        if !project.scripts.is_empty() {
            println!();
            println!("    {}", "Available Scripts:".bright_blue().bold());

            // Get sorted script names for consistent display
            let script_names = project.script_names();
            for name in script_names {
                let script = project.get_script(name).unwrap();
                let command = script.command();

                // Always show: name → command
                println!(
                    "      {} {} {}",
                    name.cyan().bold(),
                    "→".bright_black(),
                    command.to_string().bright_black()
                );

                // Optionally show description on next line with more indent
                if let Some(description) = script.description() {
                    println!("        {}", description.bright_black().italic());
                }
            }
        }
    } else {
        println!();
        println!("{}", "📋 Project Scripts".bright_black());
        println!("    {}", "No project config detected".bright_black());
    }

    // Present environment summary
    println!();
    println!("{}", "🎯 Environment Summary".green().bold());

    let ruby_version_text = format!("{} {}", ruby_type_as_str(&ruby.kind), ruby.version);
    println!(
        "    {:<width$}: {}",
        "Active Ruby".bright_blue().bold(),
        ruby_version_text.bright_black(),
        width = label_width
    );

    if let Some(bundler) = bundler_runtime {
        let project_name = bundler
            .root
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        println!(
            "    {:<width$}: {}",
            "Bundler project".bright_blue().bold(),
            project_name.bright_black(),
            width = label_width
        );

        if let Some(req_version) = bundler.ruby_version() {
            let matches = if ruby.version == req_version {
                "✅ Matches".green()
            } else {
                "⚠️  Mismatch".yellow()
            };
            println!(
                "    {:<width$}: {}",
                "Version match".bright_blue().bold(),
                matches,
                width = label_width
            );
        }
    }

    println!();
    println!(
        "{}",
        "Environment ready for distinguished Ruby development.".bright_black()
    );
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
        sandbox
            .add_ruby_dir("3.2.5")
            .expect("Failed to create ruby-3.2.5");

        // Create a basic ButlerRuntime for testing
        let butler_runtime =
            ButlerRuntime::discover_and_compose(sandbox.root().to_path_buf(), None)
                .expect("Failed to create butler runtime with test Ruby");

        // This will handle the environment presentation gracefully
        let _ = environment_command(&butler_runtime, None);
    }

    #[test]
    fn present_environment_details_handles_no_bundler() -> std::io::Result<()> {
        use rb_core::gems::GemRuntime;
        use rb_tests::RubySandbox;

        let ruby_sandbox = RubySandbox::new()?;
        let ruby_dir = ruby_sandbox.add_ruby_dir("3.2.5")?;
        let ruby = rb_core::ruby::RubyRuntime::new(
            rb_core::ruby::RubyType::CRuby,
            semver::Version::parse("3.2.5").unwrap(),
            &ruby_dir,
        );

        // Use sandboxed gem directory instead of real home directory
        let gem_runtime = GemRuntime::for_base_dir(&ruby_sandbox.gem_base_dir(), &ruby.version);
        let butler = ButlerRuntime::new(ruby.clone(), Some(gem_runtime.clone()));

        // Test with no bundler environment
        present_environment_details(&ruby, Some(&gem_runtime), None, None, &butler);

        Ok(())
    }

    #[test]
    fn present_environment_details_with_bundler() -> std::io::Result<()> {
        use rb_core::gems::GemRuntime;
        use rb_tests::{BundlerSandbox, RubySandbox};

        let ruby_sandbox = RubySandbox::new()?;
        let ruby_dir = ruby_sandbox.add_ruby_dir("3.2.5")?;
        let ruby = rb_core::ruby::RubyRuntime::new(
            rb_core::ruby::RubyType::CRuby,
            semver::Version::parse("3.2.5").unwrap(),
            &ruby_dir,
        );

        let bundler_sandbox = BundlerSandbox::new()?;
        let project_dir = bundler_sandbox.add_bundler_project("test-app", true)?;
        let bundler_runtime = BundlerRuntime::new(&project_dir, ruby.version.clone());

        // Use sandboxed gem directory instead of real home directory
        let gem_runtime = GemRuntime::for_base_dir(&ruby_sandbox.gem_base_dir(), &ruby.version);
        let butler = ButlerRuntime::new(ruby.clone(), Some(gem_runtime.clone()));

        // Test with bundler environment
        present_environment_details(
            &ruby,
            Some(&gem_runtime),
            Some(&bundler_runtime),
            None,
            &butler,
        );

        Ok(())
    }

    #[test]
    fn present_environment_details_with_project() -> std::io::Result<()> {
        use rb_core::gems::GemRuntime;
        use rb_core::project::{ProjectMetadata, ScriptDefinition};
        use rb_tests::RubySandbox;
        use std::collections::HashMap;

        let ruby_sandbox = RubySandbox::new()?;
        let ruby_dir = ruby_sandbox.add_ruby_dir("3.2.5")?;
        let ruby = rb_core::ruby::RubyRuntime::new(
            rb_core::ruby::RubyType::CRuby,
            semver::Version::parse("3.2.5").unwrap(),
            &ruby_dir,
        );

        // Create a project runtime with some scripts
        let mut scripts = HashMap::new();
        scripts.insert(
            "test".to_string(),
            ScriptDefinition::Detailed {
                command: "rspec".to_string(),
                description: Some("Run the test suite".to_string()),
            },
        );
        scripts.insert(
            "lint:fix".to_string(),
            ScriptDefinition::Simple("rubocop -a".to_string()),
        );

        let metadata = ProjectMetadata::default();
        let project_runtime =
            ProjectRuntime::new(ruby_sandbox.root(), "rbproject.toml", metadata, scripts);

        // Use sandboxed gem directory
        let gem_runtime = GemRuntime::for_base_dir(&ruby_sandbox.gem_base_dir(), &ruby.version);
        let butler = ButlerRuntime::new(ruby.clone(), Some(gem_runtime.clone()));

        // Test with project environment
        present_environment_details(
            &ruby,
            Some(&gem_runtime),
            None,
            Some(&project_runtime),
            &butler,
        );

        Ok(())
    }
}
