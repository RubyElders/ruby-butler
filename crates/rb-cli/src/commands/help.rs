use crate::Cli;
use crate::help_formatter::print_custom_help;
use rb_core::butler::ButlerError;

pub fn help_command(subcommand: Option<String>) -> Result<(), ButlerError> {
    use clap::CommandFactory;
    let mut cmd = Cli::command();

    if let Some(subcommand_name) = subcommand {
        if let Some(subcommand) = cmd.find_subcommand_mut(&subcommand_name) {
            let _ = subcommand.print_help();
        } else {
            eprintln!("Unknown command: {}", subcommand_name);
            eprintln!("Run 'rb help' to see available commands");
            std::process::exit(1);
        }
    } else {
        print_custom_help(&cmd);
        return Ok(());
    }
    println!();
    Ok(())
}
