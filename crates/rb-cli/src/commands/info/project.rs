use colored::*;
use log::{debug, info};
use rb_core::butler::{ButlerError, ButlerRuntime};
use rb_core::project::{ProjectRuntime, RbprojectDetector};
use std::path::PathBuf;

pub fn project_command(
    butler_runtime: &ButlerRuntime,
    project_file: Option<PathBuf>,
) -> Result<(), ButlerError> {
    info!("Inspecting project configuration");
    present_project_info(butler_runtime, project_file)?;
    Ok(())
}

fn present_project_info(
    butler_runtime: &ButlerRuntime,
    project_file: Option<PathBuf>,
) -> Result<(), ButlerError> {
    println!("{}", "📁 Project Configuration".to_string().bold());
    println!();

    // Load project file - either specified or discovered
    let project_runtime = if let Some(path) = project_file {
        debug!(
            "Loading project config from specified path: {}",
            path.display()
        );
        ProjectRuntime::from_file(&path).ok()
    } else {
        let current_dir = std::env::current_dir()
            .map_err(|e| ButlerError::General(format!("Failed to get current directory: {}", e)))?;
        RbprojectDetector::discover(current_dir.as_path())
            .ok()
            .flatten()
    };

    match project_runtime {
        Some(project_runtime) => {
            println!(
                "  {} {}",
                "Project File:".bold(),
                project_runtime.rbproject_path().display()
            );
            println!();

            if let Some(name) = &project_runtime.metadata.name {
                println!("  {} {}", "Name:".bold(), name.cyan());
            }

            if let Some(description) = &project_runtime.metadata.description {
                println!("  {} {}", "Description:".bold(), description.dimmed());
            }

            if !project_runtime.scripts.is_empty() {
                println!();
                println!("  {}", "Scripts:".bold());
                for (name, script) in &project_runtime.scripts {
                    if let Some(desc) = script.description() {
                        println!(
                            "    {} → {} {}",
                            name.cyan(),
                            script.command().dimmed(),
                            format!("({})", desc).bright_black()
                        );
                    } else {
                        println!("    {} → {}", name.cyan(), script.command().dimmed());
                    }
                }
            }
        }
        None => {
            println!(
                "  {}",
                "No rbproject.toml found in current directory or parents".dimmed()
            );
            println!();
            println!("  {} Run {} to create one.", "Tip:".bold(), "rb new".cyan());
        }
    }

    println!();

    // Show effective configuration
    println!("{}", "🔧 Effective Configuration".to_string().bold());
    println!();
    println!(
        "  {} {}",
        "Rubies Directory:".bold(),
        butler_runtime.rubies_dir().display()
    );

    if let Some(gem_base) = butler_runtime.gem_base_dir() {
        println!("  {} {}", "Gem Home:".bold(), gem_base.display());
    }

    if let Some(requested) = butler_runtime.requested_ruby_version() {
        println!("  {} {}", "Requested Ruby:".bold(), requested);
    }

    println!();
    Ok(())
}
