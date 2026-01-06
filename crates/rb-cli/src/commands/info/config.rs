use crate::config::TrackedConfig;
use colored::Colorize;
use rb_core::butler::ButlerError;

/// Display current configuration with sources
pub fn config_command(config: &TrackedConfig) -> Result<(), ButlerError> {
    println!("{}", "🎩 Current Configuration".bright_cyan().bold());
    println!();

    // Rubies directory
    println!(
        "{} {}",
        "Rubies Directory:".bright_white().bold(),
        config.rubies_dir.get().display()
    );
    println!(
        "  {} {}",
        "Source:".dimmed(),
        format!("{}", config.rubies_dir.source).yellow()
    );
    println!();

    // Ruby version
    if let Some(ref version) = config.ruby_version {
        println!(
            "{} {}",
            "Ruby Version:".bright_white().bold(),
            version.get()
        );
        println!(
            "  {} {}",
            "Source:".dimmed(),
            format!("{}", version.source).yellow()
        );
        if version.is_unresolved() {
            println!(
                "  {} {}",
                "Note:".dimmed(),
                "Will be resolved to latest available Ruby".cyan()
            );
        }
    } else {
        println!(
            "{} {}",
            "Ruby Version:".bright_white().bold(),
            "latest".dimmed()
        );
        println!("  {} {}", "Source:".dimmed(), "default".yellow());
        println!(
            "  {} {}",
            "Note:".dimmed(),
            "Will use latest available Ruby".cyan()
        );
    }
    println!();

    // Gem home
    println!(
        "{} {}",
        "Gem Home:".bright_white().bold(),
        config.gem_home.get().display()
    );
    println!(
        "  {} {}",
        "Source:".dimmed(),
        format!("{}", config.gem_home.source).yellow()
    );
    println!();

    // No bundler
    println!(
        "{} {}",
        "No Bundler:".bright_white().bold(),
        if *config.no_bundler.get() {
            "yes".green()
        } else {
            "no".dimmed()
        }
    );
    println!(
        "  {} {}",
        "Source:".dimmed(),
        format!("{}", config.no_bundler.source).yellow()
    );
    println!();

    // Working directory
    println!(
        "{} {}",
        "Working Directory:".bright_white().bold(),
        config.work_dir.get().display()
    );
    println!(
        "  {} {}",
        "Source:".dimmed(),
        format!("{}", config.work_dir.source).yellow()
    );
    println!();

    println!("{}", "Configuration sources (in priority order):".dimmed());
    println!("  {} CLI arguments", "1.".dimmed());
    println!("  {} Configuration file", "2.".dimmed());
    println!("  {} Environment variables", "3.".dimmed());
    println!("  {} Built-in defaults", "4.".dimmed());

    Ok(())
}
