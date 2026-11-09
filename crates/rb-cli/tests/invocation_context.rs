use clap::{CommandFactory, FromArgMatches};
use rb_cli::{
    Cli, Commands,
    config::{ConfigSource, RbConfig, TrackedConfig},
    dispatch::dispatch_command,
    runtime_helpers::CommandContext,
};
use rb_core::process_context::ProcessContext;
use std::{collections::BTreeMap, ffi::OsString};

fn parse(arguments: &[&str]) -> (clap::ArgMatches, Cli) {
    let mut command = Cli::command();
    let ids = command
        .get_arguments()
        .map(|argument| argument.get_id().clone())
        .collect::<Vec<_>>();
    for id in ids {
        command = command.mut_arg(id, |argument| argument.env(None::<&str>));
    }
    let matches = command.try_get_matches_from(arguments).unwrap();
    let cli = Cli::from_arg_matches(&matches).unwrap();
    (matches, cli)
}

#[test]
fn captured_configuration_preserves_precedence_and_sources() {
    let directory = tempfile::tempdir().unwrap();
    let inputs = ProcessContext::new(
        directory.path().into(),
        BTreeMap::from([
            ("RB_RUBIES_DIR".into(), OsString::from("environment-rubies")),
            ("RB_RUBY_VERSION".into(), "3.4.10".into()),
            ("RB_GEM_HOME".into(), "environment-gems".into()),
            ("RB_NO_BUNDLER".into(), "true".into()),
            ("RB_PACKAGE_BACKEND".into(), "native".into()),
            ("RB_AUTO_INSTALL_RUBY".into(), "false".into()),
        ]),
    );
    let file = RbConfig {
        rubies_dir: Some("file-rubies".into()),
        ruby_version: Some("3.3.0".into()),
        gem_home: Some("file-gems".into()),
        no_bundler: Some(false),
        ..Default::default()
    };
    let (matches, cli) = parse(&["rb", "help"]);
    let config = TrackedConfig::from_matches(&cli.config, &file, &matches, &inputs);
    assert_eq!(
        config.rubies_dir.get(),
        std::path::Path::new("environment-rubies")
    );
    assert_eq!(config.rubies_dir.source, ConfigSource::EnvVar);
    assert_eq!(config.ruby_version.as_ref().unwrap().get(), "3.4.10");
    assert_eq!(
        config.ruby_version.as_ref().unwrap().source,
        ConfigSource::EnvVar
    );
    assert_eq!(config.gem_home.source, ConfigSource::EnvVar);
    assert!(*config.no_bundler.get());
    assert_eq!(config.no_bundler.source, ConfigSource::EnvVar);
    assert_eq!(config.package_backend.source, ConfigSource::EnvVar);
    assert!(!*config.auto_install_ruby.get());
    let (matches, cli) = parse(&[
        "rb",
        "-R",
        "cli-rubies",
        "--package-backend",
        "bundler",
        "--auto-install-ruby",
        "help",
    ]);
    let config = TrackedConfig::from_matches(&cli.config, &file, &matches, &inputs);
    assert_eq!(config.rubies_dir.get(), std::path::Path::new("cli-rubies"));
    assert_eq!(config.rubies_dir.source, ConfigSource::Cli);
    assert_eq!(config.package_backend.source, ConfigSource::Cli);
    assert!(*config.auto_install_ruby.get());
}

#[test]
fn commands_use_explicit_directories_without_changing_the_parent() {
    let original = std::env::current_dir().unwrap();
    let environment = std::env::vars_os().collect::<BTreeMap<_, _>>();
    let first = tempfile::tempdir().unwrap();
    let second = tempfile::tempdir().unwrap();
    for directory in [first.path(), second.path()] {
        let inputs = ProcessContext::new(directory.into(), BTreeMap::new());
        let config =
            TrackedConfig::from_merged_in(&RbConfig::default(), &RbConfig::default(), &inputs);
        let mut context = CommandContext::from_inputs(config, None, inputs);
        dispatch_command(Commands::New, &mut context).unwrap();
        assert!(directory.join("rbproject.toml").is_file());
        assert_eq!(std::env::current_dir().unwrap(), original);
        assert_eq!(std::env::vars_os().collect::<BTreeMap<_, _>>(), environment);
    }
}

#[test]
fn project_inspection_and_script_listing_do_not_require_ruby() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("rbproject.toml"),
        "[project]\nname = 'inspection'\n[scripts]\nprobe = 'ruby -v'\n",
    )
    .unwrap();
    let inputs = ProcessContext::new(directory.path().into(), BTreeMap::new());
    let file = RbConfig {
        rubies_dir: Some(directory.path().join("missing")),
        ..Default::default()
    };
    let config = TrackedConfig::from_merged_in(&RbConfig::default(), &file, &inputs);
    let mut context = CommandContext::from_inputs(config, None, inputs);
    dispatch_command(
        Commands::Info {
            command: rb_cli::InfoCommands::Project,
        },
        &mut context,
    )
    .unwrap();
    dispatch_command(
        Commands::Run {
            script: None,
            args: Vec::new(),
        },
        &mut context,
    )
    .unwrap();
    assert!(!directory.path().join("missing").exists());
    assert!(!directory.path().join(".rb").exists());
}
