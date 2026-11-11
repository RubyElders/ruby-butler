pub fn print_custom_help(command: &clap::Command) {
    let report = crate::commands::help::HelpReport::collect(command, true);
    crate::output::Output::default()
        .emit("help", &report, || {
            crate::presentation::help::document(&report)
        })
        .expect("writing help");
}
