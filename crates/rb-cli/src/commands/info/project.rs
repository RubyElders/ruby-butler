use crate::config::TrackedConfig;
use colored::*;
use log::info;
use rb_core::butler::ButlerError;
use rb_core::project::ProjectRuntime;

pub(crate) fn project_command(
    config: &TrackedConfig,
    project_runtime: Option<ProjectRuntime>,
) -> Result<(), ButlerError> {
    info!("Inspecting project configuration");
    present_project_info(config, project_runtime)?;
    Ok(())
}

fn present_project_info(
    config: &TrackedConfig,
    project_runtime: Option<ProjectRuntime>,
) -> Result<(), ButlerError> {
    println!("{}", "📁 Project Configuration".to_string().bold());
    println!();

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

    println!("{}", "🔧 Effective Configuration".to_string().bold());
    println!();
    println!(
        "  {} {}",
        "Rubies Directory:".bold(),
        config.rubies_dir.get().display()
    );

    let gem_base = config.gem_home.get();
    println!("  {} {}", "Gem Home:".bold(), gem_base.display());

    if let Some(requested) = &config.ruby_version {
        let requested = requested.get();
        println!("  {} {}", "Requested Ruby:".bold(), requested);
    }

    println!();
    Ok(())
}
