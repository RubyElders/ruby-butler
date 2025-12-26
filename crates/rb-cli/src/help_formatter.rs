use colored::Colorize;

/// Print custom help with command grouping
pub fn print_custom_help(cmd: &clap::Command) {
    // Print header
    if let Some(about) = cmd.get_about() {
        println!("{}", about);
    }
    println!();

    // Print usage
    let bin_name = cmd.get_name();
    println!(
        "{} {} {} {} {}",
        "Usage:".green().bold(),
        bin_name.cyan().bold(),
        "[OPTIONS]".cyan(),
        "COMMAND".cyan().bold(),
        "[COMMAND_OPTIONS]".cyan()
    );
    println!();

    // Group commands
    let runtime_commands = ["runtime", "environment", "exec", "sync", "run"];
    let utility_commands = ["init", "config", "version", "help", "shell-integration"];

    // Print runtime commands
    println!("{}", "Commands:".green().bold());
    for subcmd in cmd.get_subcommands() {
        let name = subcmd.get_name();
        if runtime_commands.contains(&name) {
            print_command_line(subcmd);
        }
    }
    println!();

    // Print utility commands
    println!("{}", "Utility Commands:".green().bold());
    for subcmd in cmd.get_subcommands() {
        let name = subcmd.get_name();
        if utility_commands.contains(&name) {
            print_command_line(subcmd);
        }
    }
    println!();

    // Print options
    println!("{}", "Options:".green().bold());
    for arg in cmd.get_arguments() {
        if arg.get_id() == "help" || arg.get_id() == "version" {
            continue;
        }
        print_argument_line(arg);
    }
}

/// Helper to print a command line
fn print_command_line(subcmd: &clap::Command) {
    let name = subcmd.get_name();
    let about = subcmd
        .get_about()
        .map(|s| s.to_string())
        .unwrap_or_default();
    let aliases: Vec<_> = subcmd.get_all_aliases().collect();

    if aliases.is_empty() {
        println!("  {:18} {}", name.cyan().bold(), about);
    } else {
        let alias_str = format!("[aliases: {}]", aliases.join(", "));
        println!("  {:18} {} {}", name.cyan().bold(), about, alias_str.cyan());
    }
}

/// Helper to print an argument line
fn print_argument_line(arg: &clap::Arg) {
    let short = arg
        .get_short()
        .map(|c| format!("-{}", c))
        .unwrap_or_default();
    let long = arg
        .get_long()
        .map(|s| format!("--{}", s))
        .unwrap_or_default();

    let flag = if !short.is_empty() && !long.is_empty() {
        format!("{}, {}", short, long)
    } else if !short.is_empty() {
        short
    } else {
        long
    };

    // Only show value placeholder if it actually takes values (not boolean flags)
    let value_name = if arg.get_num_args().unwrap_or_default().takes_values()
        && arg.get_action().takes_values()
    {
        format!(
            " <{}>",
            arg.get_id().as_str().to_uppercase().replace('_', "-")
        )
    } else {
        String::new()
    };

    let help = arg.get_help().map(|s| s.to_string()).unwrap_or_default();

    // Show env var if available
    let env_var = if let Some(env) = arg.get_env() {
        format!(" [env: {}]", env.to_string_lossy())
    } else {
        String::new()
    };

    // Calculate visual width for alignment (without ANSI codes)
    let visual_width = flag.len() + value_name.len();
    let padding = if visual_width < 31 {
        31 - visual_width
    } else {
        1
    };

    // Color the flag and value name, but keep help text uncolored
    let colored_flag = flag.cyan().bold();
    let colored_value = if !value_name.is_empty() {
        value_name.cyan().to_string()
    } else {
        String::new()
    };
    let colored_env = if !env_var.is_empty() {
        format!(" {}", env_var.cyan())
    } else {
        String::new()
    };

    println!(
        "  {}{}{}{}{}",
        colored_flag,
        colored_value,
        " ".repeat(padding),
        help,
        colored_env
    );
}
