use crate::packages::PackageRuntime;
use crate::packages::{PackageBackend, PackageEnvironment, PackageStatus};
use colored::*;
use log::{debug, info};
use rb_core::butler::ButlerError;
use rb_core::project::ProjectRuntime;
use rb_core::ruby::RubyType;

pub(crate) fn environment_command(
    runtime: &PackageRuntime,
    project_runtime: Option<ProjectRuntime>,
    backend: &PackageBackend,
) -> Result<(), ButlerError> {
    info!("Presenting current Ruby environment from the working directory");
    let environment = backend
        .inspect(runtime)
        .map_err(|error| ButlerError::General(format!("{error:#}")))?;
    let synchronized = if environment
        .details
        .as_ref()
        .is_some_and(|details| details.configured)
    {
        backend
            .check(runtime)
            .map(|status| matches!(status, PackageStatus::Ready(_)))
            .map_err(|error| {
                debug!("Unable to check package synchronization: {error:#}");
                error
            })
            .ok()
    } else {
        Some(false)
    };
    present_current_environment(&environment, project_runtime, synchronized)
}

fn present_current_environment(
    environment: &PackageEnvironment,
    project_runtime: Option<ProjectRuntime>,
    synchronized: Option<bool>,
) -> Result<(), ButlerError> {
    println!("{}", "🌍 Your Current Ruby Environment".to_string().bold());
    println!();

    let current_dir = environment.current_dir();
    debug!("Current working directory: {}", current_dir.display());
    let ruby = environment.ruby()?;

    // Present the environment
    present_environment_details(ruby, project_runtime.as_ref(), environment, synchronized);

    Ok(())
}

fn present_environment_details(
    ruby: &rb_core::ruby::RubyRuntime,
    project_runtime: Option<&ProjectRuntime>,
    environment: &PackageEnvironment,
    synchronized: Option<bool>,
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

    let gem_home = environment.gem_home();
    println!(
        "    {:<width$}: {}",
        "Gem home".bright_blue().bold(),
        gem_home
            .map(|home| home.display().to_string())
            .unwrap_or_else(|| "Not available".into())
            .bright_black(),
        width = label_width
    );

    let gem_dirs = environment.gem_dirs();
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

    let bin_dirs = environment.bin_dirs();
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

    println!();
    println!(
        "{}",
        format!("📦 {} Environment", environment.name)
            .green()
            .bold()
    );
    if let Some(details) = &environment.details {
        for (label, path) in [
            ("Project root", Some(&details.root)),
            ("Gemfile", details.gemfile.as_ref()),
            ("App config", details.configuration.as_ref()),
            ("Installation", Some(&details.installation)),
        ] {
            if let Some(path) = path {
                println!(
                    "    {:<width$}: {}",
                    label.bright_blue().bold(),
                    path.display().to_string().bright_black(),
                    width = label_width
                );
            }
        }
        if let Some(version) = &details.required_ruby {
            println!(
                "    {:<width$}: {}",
                "Required Ruby".bright_blue().bold(),
                version.to_string().bright_black(),
                width = label_width
            );
        }
        let configured = if details.configured {
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
        let status = match synchronized {
            Some(true) => "✅ Synchronized".green(),
            Some(false) => "⚠️  Out of sync".yellow(),
            None => "❓ Unknown".bright_black(),
        };
        println!(
            "    {:<width$}: {}",
            "Synchronized".bright_blue().bold(),
            status,
            width = label_width
        );
    } else {
        println!("    {}", "Package environment not detected".bright_black());
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

    if let Some(details) = &environment.details {
        let project_name = details
            .root
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        println!(
            "    {:<width$}: {}",
            "Package project".bright_blue().bold(),
            project_name.bright_black(),
            width = label_width
        );
        if let Some(version) = &details.required_ruby {
            let matches = if ruby.version == *version {
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
