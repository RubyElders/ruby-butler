use clap::{Arg, Command};
use rb_cli::help_formatter::print_custom_help;

#[test]
fn test_print_custom_help_does_not_panic() {
    let cmd = Command::new("test")
        .about("Test command")
        .arg(Arg::new("flag").short('f').help("Test flag"));

    print_custom_help(&cmd);
}

#[test]
fn test_print_custom_help_handles_subcommands() {
    let cmd = Command::new("test")
        .about("Test command")
        .subcommand(Command::new("sub1").about("Subcommand 1"))
        .subcommand(Command::new("sub2").about("Subcommand 2"));

    print_custom_help(&cmd);
}

#[test]
fn test_print_custom_help_handles_arguments() {
    let cmd = Command::new("test")
        .about("Test command")
        .arg(
            Arg::new("input")
                .short('i')
                .long("input")
                .help("Input file"),
        )
        .arg(
            Arg::new("output")
                .short('o')
                .long("output")
                .help("Output file"),
        );

    print_custom_help(&cmd);
}
