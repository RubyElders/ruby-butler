use crate::{Cli, output::Output};
use clap::CommandFactory;
use rb_core::butler::ButlerError;
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct ArgumentReport {
    pub name: String,
    pub short: Option<char>,
    pub long: Option<String>,
    pub help: String,
    pub required: bool,
    pub environment: Option<String>,
    pub possible_values: Vec<String>,
}
#[derive(Serialize)]
pub(crate) struct HelpReport {
    pub name: String,
    pub about: String,
    pub aliases: Vec<String>,
    pub arguments: Vec<ArgumentReport>,
    pub commands: Vec<HelpReport>,
    #[serde(skip)]
    pub command: clap::Command,
    #[serde(skip)]
    pub custom: bool,
}
impl HelpReport {
    pub(crate) fn collect(command: &clap::Command, custom: bool) -> Self {
        Self {
            name: command.get_name().into(),
            about: command
                .get_about()
                .map(ToString::to_string)
                .unwrap_or_default(),
            aliases: command.get_all_aliases().map(str::to_owned).collect(),
            arguments: command
                .get_arguments()
                .filter(|arg| !arg.is_hide_set())
                .map(|arg| ArgumentReport {
                    name: arg.get_id().to_string(),
                    short: arg.get_short(),
                    long: arg.get_long().map(str::to_owned),
                    help: arg.get_help().map(ToString::to_string).unwrap_or_default(),
                    required: arg.is_required_set(),
                    environment: arg.get_env().map(|v| v.to_string_lossy().into_owned()),
                    possible_values: arg
                        .get_value_parser()
                        .possible_values()
                        .map(|values| values.map(|v| v.get_name().into()).collect())
                        .unwrap_or_default(),
                })
                .collect(),
            commands: command
                .get_subcommands()
                .filter(|cmd| !cmd.is_hide_set())
                .map(|cmd| Self::collect(cmd, false))
                .collect(),
            command: command.clone(),
            custom,
        }
    }
}
pub fn help_command(subcommand: Option<String>) -> Result<(), ButlerError> {
    help_with_output(subcommand, &Output::default())
}
pub fn help_with_output(subcommand: Option<String>, output: &Output) -> Result<(), ButlerError> {
    let mut command = Cli::command();
    let custom = subcommand.is_none();
    if let Some(name) = subcommand {
        if let Some(selected) = command.find_subcommand_mut(&name) {
            command = selected.clone();
        } else {
            eprintln!("Unknown command: {}", name);
            eprintln!("Run 'rb help' to see available commands");
            std::process::exit(1);
        }
    }
    let report = HelpReport::collect(&command, custom);
    output.emit("help", &report, || {
        crate::presentation::help::document(&report)
    })
}
