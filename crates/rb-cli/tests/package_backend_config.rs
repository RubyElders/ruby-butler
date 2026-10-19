use clap::Parser;
use rb_cli::{
    Cli,
    config::{PackageBackendKind, RbConfig, TrackedConfig},
};
use std::process::Command;

#[test]
fn defaults_to_bundler_and_cli_overrides_file() {
    let file: RbConfig = toml::from_str("package-backend = 'native'").unwrap();
    let defaults = TrackedConfig::from_merged(&RbConfig::default(), &RbConfig::default());
    assert_eq!(*defaults.package_backend.get(), PackageBackendKind::Bundler);
    let cli = Cli::try_parse_from(["rb", "sync", "--package-backend", "bundler"]).unwrap();
    let config = TrackedConfig::from_merged(&cli.config, &file);
    assert_eq!(*config.package_backend.get(), PackageBackendKind::Bundler);
    assert_eq!(
        *TrackedConfig::from_merged(&RbConfig::default(), &file)
            .package_backend
            .get(),
        PackageBackendKind::Native
    );
    assert!(Cli::try_parse_from(["rb", "--package-backend", "unknown", "sync"]).is_err());
    assert!(toml::from_str::<RbConfig>("package-backend='unknown'").is_err());
}

#[test]
fn environment_overrides_file_and_cli_overrides_environment() {
    let root = tempfile::tempdir().unwrap();
    let config = root.path().join("config.toml");
    std::fs::write(&config, "package-backend='bundler'\n").unwrap();
    let run = |extra: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_rb"))
            .args(["--config", config.to_str().unwrap()])
            .args(extra)
            .args(["info", "config"])
            .env("RB_PACKAGE_BACKEND", "native")
            .output()
            .unwrap()
    };
    let output = run(&[]);
    assert!(output.status.success(), "{:?}", output);
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Package Backend: native"), "{text}");
    assert!(text.contains("Source: environment"), "{text}");
    let output = run(&["--package-backend", "bundler"]);
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("Package Backend: bundler"), "{text}");
    assert!(text.contains("Source: CLI argument"), "{text}");
}
