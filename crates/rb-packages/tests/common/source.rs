use rb_packages::{PrepareOptions, PreparedBundle};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub fn source(root: &Path) -> PathBuf {
    let directory = root.join("source-repository");
    fs::create_dir_all(directory.join("lib/fixture")).unwrap();
    fs::write(
        directory.join("fixture.gemspec"),
        include_str!("../fixtures/sources/fixture.gemspec"),
    )
    .unwrap();
    fs::write(
        directory.join("lib/fixture.rb"),
        include_str!("../fixtures/sources/lib/fixture.rb"),
    )
    .unwrap();
    fs::write(
        directory.join("lib/fixture/version.rb"),
        include_str!("../fixtures/sources/lib/fixture/version.rb"),
    )
    .unwrap();
    directory
}

#[cfg(unix)]
pub fn native_source(root: &Path) -> PathBuf {
    let source = source(root);
    fs::create_dir_all(source.join("ext/fixture")).unwrap();
    fs::write(
        source.join("ext/fixture/extconf.rb"),
        include_str!("../fixtures/sources/extconf.rb"),
    )
    .unwrap();
    fs::write(
        source.join("ext/fixture/source_native.c"),
        include_str!("../fixtures/sources/source_native.c"),
    )
    .unwrap();
    fs::write(
        source.join("lib/fixture.rb"),
        "require_relative 'fixture/version'; require 'source_native'\n",
    )
    .unwrap();
    source
}

pub fn git(directory: &Path, arguments: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(arguments)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().into()
}

pub fn commit(directory: &Path) -> String {
    git(directory, &["add", "."]);
    git(
        directory,
        &[
            "-c",
            "user.name=Fixture",
            "-c",
            "user.email=fixture@example.test",
            "commit",
            "-m",
            "Fixture",
        ],
    );
    git(directory, &["rev-parse", "HEAD"])
}

pub fn options(root: &Path) -> PrepareOptions {
    let mut options = PrepareOptions::new(root, super::ruby(), root.join(".rb/cache"));
    options
        .install
        .build
        .environment
        .insert("CCACHE_DISABLE".into(), "1".into());
    options
}

pub fn require(bundle: &PreparedBundle, code: &str) -> String {
    let output = Command::new(super::ruby())
        .envs(bundle.installed.environment())
        .env_remove("RUBYOPT")
        .env_remove("RUBYLIB")
        .env("BUNDLE_FROZEN", "true")
        .current_dir(&bundle.input.root)
        .args(["-rbundler/setup", "-e", code])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    super::stdout(&output)
}
