use colored::*;
use rb_core::ruby::RubyType;
use rb_core::butler::ButlerRuntime;
use log::{debug, info};
use semver::Version;

pub fn runtime_command(butler_runtime: &ButlerRuntime) {
    info!("Surveying Ruby installations in distinguished directory: {}", butler_runtime.rubies_dir().display());
    present_ruby_installations(butler_runtime);
}

fn present_ruby_installations(butler_runtime: &ButlerRuntime) {
    let rubies_dir = butler_runtime.rubies_dir();
    let ruby_installations = butler_runtime.ruby_installations();
    let requested_ruby_version = butler_runtime.requested_ruby_version();
    
    println!("{}", format!("💎 Ruby Environment Survey").bold());
    println!();

    debug!("Surveying directory: {}", rubies_dir.display());
    debug!("Found {} Ruby installations", ruby_installations.len());

    if ruby_installations.is_empty() {
        butler_runtime.display_no_ruby_error();
        return;
    }

    // Collect all ruby display data first for proper alignment calculation
    let mut ruby_display_data = Vec::new();
    
    for ruby in ruby_installations {
        let ruby_type = match ruby.kind {
            RubyType::CRuby => "CRuby",
        };
        let ruby_header = format!("{} ({})", ruby_type, ruby.version);
        
        // Try to infer gem runtime and compose full ButlerRuntime
        match ruby.infer_gem_runtime() {
            Ok(gem_runtime) => {
                debug!("Inferred gem runtime for Ruby {}: {}", ruby.version, gem_runtime.gem_home.display());
                
                // Create ButlerRuntime with Ruby and Gem runtimes
                let butler = ButlerRuntime::new(ruby.clone(), Some(gem_runtime.clone()));
                
                let gem_dirs = butler.gem_dirs();
                let bin_dirs = butler.bin_dirs();
                
                ruby_display_data.push((
                    ruby_header,
                    ruby.root.display().to_string(),
                    Some(gem_runtime.gem_home.display().to_string()),
                    gem_dirs.iter().map(|d| d.display().to_string()).collect::<Vec<_>>(),
                    bin_dirs.iter().map(|d| d.display().to_string()).collect::<Vec<_>>(),
                ));
                
                debug!("Composed ButlerRuntime for Ruby {}: {} bin dirs, {} gem dirs", 
                       ruby.version, bin_dirs.len(), gem_dirs.len());
            }
            Err(e) => {
                debug!("Failed to infer gem runtime for Ruby {}: {}", ruby.version, e);
                
                // Create ButlerRuntime with Ruby only
                let butler = ButlerRuntime::new(ruby.clone(), None);
                
                let gem_dirs = butler.gem_dirs();
                let bin_dirs = butler.bin_dirs();
                
                ruby_display_data.push((
                    ruby_header,
                    ruby.root.display().to_string(),
                    None, // No gem home
                    gem_dirs.iter().map(|d| d.display().to_string()).collect::<Vec<_>>(),
                    bin_dirs.iter().map(|d| d.display().to_string()).collect::<Vec<_>>(),
                ));
            }
        }
    }
    
    // Calculate maximum label width for proper presentation
    let label_width = ["Installation", "Gem home", "Gem libraries", "Executable paths"]
        .iter()
        .map(|s| s.len())
        .max()
        .unwrap_or(12);
    
    // Present each Ruby environment with appropriate refinement
    for (ruby_header, ruby_path, gem_home, gem_paths, bin_paths) in ruby_display_data {
        // Present Ruby header with distinction
        let ruby_type = if ruby_header.starts_with("CRuby") { "💎 CRuby".green() } else { ruby_header.as_str().green() };
        let version_start = ruby_header.find('(').unwrap_or(0);
        let version = ruby_header[version_start..].cyan();
        
        println!("{} {}", ruby_type, version);
        
        // Present installation location with proper alignment
        println!("    {:<width$}: {}", 
            "Installation".bright_blue().bold(), 
            ruby_path.bright_black(),
            width = label_width
        );
        
        // Present gem home with appropriate dignity
        if let Some(gem_home) = gem_home {
            println!("    {:<width$}: {}", 
                "Gem home".bright_blue().bold(), 
                gem_home.bright_black(),
                width = label_width
            );
        } else {
            println!("    {:<width$}: {}", 
                "Gem home".bright_blue().bold(), 
                "Not available".yellow(),
                width = label_width
            );
        }
        
        // Present gem libraries with proper ceremony
        if !gem_paths.is_empty() {
            println!("    {:<width$}:", "Gem libraries".bright_blue().bold(), width = label_width);
            for gem_path in gem_paths {
                println!("    {:<width$}  {}", "", gem_path.cyan(), width = label_width);
            }
        }
        
        // Present executable paths with proper ceremony
        if !bin_paths.is_empty() {
            println!("    {:<width$}:", "Executable paths".bright_blue().bold(), width = label_width);
            for bin_path in bin_paths {
                println!("    {:<width$}  {}", "", bin_path.green(), width = label_width);
            }
        }

        println!(); // Maintain dignified spacing between entries
    }

    println!();

    // Handle Ruby selection with appropriate ceremony
    if let Some(version_str) = requested_ruby_version {
        debug!("Seeking your requested Ruby version: {}", version_str);
        
        // Attempt to locate the precise version requested
        let found = if let Ok(requested_version) = Version::parse(version_str) {
            ruby_installations.iter().find(|ruby| ruby.version == requested_version)
        } else {
            // If version parsing is unsuccessful, attempt string matching
            ruby_installations.iter().find(|ruby| ruby.version.to_string() == *version_str)
        };
        
        match found {
            Some(ruby) => {
                info!("Your requested Ruby environment has been located: {} {}", ruby_type_as_str(&ruby.kind), ruby.version);
                println!("{}: {} {} {} {}", 
                    "Environment Selected".bold(),
                    "(as requested)".bright_blue(),
                    ruby_type_as_str(&ruby.kind).green(),
                    format!("({})", ruby.version).cyan(),
                    format!("residing at {}", ruby.root.display()).bright_black()
                );
            }
            None => {
                eprintln!("{}: The requested Ruby version {} could not be located in your estate", 
                        "Selection Failed".red().bold(), version_str.cyan());
                eprintln!("Available versions in your collection: {}", 
                    ruby_installations.iter()
                        .map(|r| r.version.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                        .bright_cyan()
                );
                std::process::exit(1);
            }
        }
    } else {
        // Present the finest Ruby with appropriate recognition
        if let Some(latest) = ruby_installations.iter().max_by_key(|r| &r.version) {
            info!("Presenting your finest Ruby installation: {} {}", ruby_type_as_str(&latest.kind), latest.version);
            println!("{}: {} {} {} {}", 
                "Environment Ready".bold(),
                "(latest available)".bright_blue(),
                ruby_type_as_str(&latest.kind).green(),
                format!("({})", latest.version).cyan(),
                format!("residing at {}", latest.root.display()).bright_black()
            );
        }
    }

    println!();
    
    
    if let Some(requested) = requested_ruby_version {
        println!("{}", format!("Environment ready for distinguished Ruby development with version {}.", requested).dimmed());
    } else {
        println!("{}", format!("Environment ready for distinguished Ruby development.").dimmed());
    }
}

fn ruby_type_as_str(ruby_type: &RubyType) -> &'static str {
    match ruby_type {
        RubyType::CRuby => "CRuby",
    }
}#[cfg(test)]
mod tests {
    use rb_tests::RubySandbox;
    use rb_core::butler::ButlerRuntime;

    #[test]
    fn test_runtime_command_with_directory() {
        let sandbox = RubySandbox::new().expect("Failed to create sandbox");
        sandbox.add_ruby_dir("3.2.5").expect("Failed to create ruby-3.2.5");

        // Test using ButlerRuntime
        let path = sandbox.root().to_path_buf();
        let butler_runtime = ButlerRuntime::discover_and_compose(path, None).expect("Failed to create butler runtime");
        
        // This test just verifies the function can be called without panicking
        super::runtime_command(&butler_runtime);
    }
}
