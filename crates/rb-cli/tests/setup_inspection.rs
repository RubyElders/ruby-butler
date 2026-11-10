#![cfg(unix)]
#[path = "common/ruby_install.rs"]
mod support;
use std::fs;
use support::{Fixture, success};

#[test]
fn inspection_without_ruby_does_not_evaluate_install_or_create_project_state() {
    for backend in ["bundler", "native"] {
        let fixture = Fixture::new();
        fs::write(fixture.root.path().join(".ruby-version"), "ruby-4.0.9\n").unwrap();
        fs::write(
            fixture.root.path().join("Gemfile"),
            "raise 'must not evaluate'\n",
        )
        .unwrap();
        let output = success(
            fixture
                .command()
                .args(["--package-backend", backend, "info", "setup"])
                .output()
                .unwrap(),
        );
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["backend"], backend);
        assert_eq!(value["ruby"]["version"], "4.0.9");
        assert_eq!(value["ruby"]["installed"], false);
        assert!(value["cache"].is_null());
        assert!(!fixture.root.path().join(".rb").exists());
        assert!(!fixture.root.path().join("rubies").exists());
    }
}

#[test]
fn exact_selection_and_catalog_selection_share_normal_globals_without_installing() {
    let fixture = Fixture::new();
    fixture.seed();
    let output = success(
        fixture
            .command()
            .args(["-r", "3.4", "info", "setup", "--resolve-ruby"])
            .output()
            .unwrap(),
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["ruby"]["version"], "3.4.10");
    assert_eq!(value["ruby"]["installed"], false);
    assert!(!fixture.installed());
    assert!(!fixture.root.path().join(".rb").exists());
}

#[test]
fn probe_reports_backend_installation_and_tracks_selected_dependency_helpers() {
    use std::process::Command;
    let ruby = which::which(std::env::var_os("RB_GEM_BUILD_RUBY").unwrap_or_else(|| "ruby".into()))
        .unwrap();
    let version = Command::new(&ruby)
        .args(["-e", "print RUBY_VERSION"])
        .output()
        .unwrap();
    assert!(version.status.success());
    let version = String::from_utf8(version.stdout).unwrap();
    for backend in ["bundler", "native"] {
        let fixture = Fixture::new();
        let bin = fixture
            .root
            .path()
            .join(format!("rubies/ruby-{version}/bin"));
        fs::create_dir_all(&bin).unwrap();
        std::os::unix::fs::symlink(&ruby, bin.join("ruby")).unwrap();
        fs::write(
            fixture.root.path().join("Gemfile"),
            "source 'https://rubygems.org'\neval_gemfile 'Dependencies'\n",
        )
        .unwrap();
        fs::write(fixture.root.path().join("Dependencies"), "gem 'rake'\n").unwrap();
        fs::write(fixture.root.path().join("Gemfile.lock"), "fixture\n").unwrap();
        let inspect = || {
            let output = success(
                fixture
                    .command()
                    .args([
                        "--package-backend",
                        backend,
                        "-r",
                        &version,
                        "info",
                        "setup",
                        "--probe",
                    ])
                    .output()
                    .unwrap(),
            );
            serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()
        };
        let first = inspect();
        assert_eq!(first["ruby"]["installed"], true);
        assert_eq!(first["cache"]["fingerprint"].as_str().unwrap().len(), 64);
        let path = first["cache"]["paths"][0].as_str().unwrap();
        assert!(path.contains(if backend == "bundler" {
            "/.rb/vendor/bundler"
        } else {
            "/.rb/bundle/"
        }));
        fs::write(fixture.root.path().join("application.rb"), "unrelated\n").unwrap();
        assert_eq!(first["cache"], inspect()["cache"]);
        fs::write(
            fixture.root.path().join("Dependencies"),
            "gem 'rake', '13.3.1'\n",
        )
        .unwrap();
        assert_ne!(
            first["cache"]["fingerprint"],
            inspect()["cache"]["fingerprint"]
        );
        assert!(!fixture.root.path().join(".rb/prepared.json").exists());
    }
}

#[test]
fn inspection_respects_project_config_working_directory_and_external_gemfile() {
    let fixture = Fixture::new();
    let child = fixture.root.path().join("nested");
    fs::create_dir(&child).unwrap();
    fs::write(
        fixture.root.path().join("config.toml"),
        "ruby-version='3.4.10'\npackage-backend='native'\n",
    )
    .unwrap();
    fs::write(child.join("custom.toml"), "[dependencies]\n").unwrap();
    let output = success(
        fixture
            .command()
            .args(["-C", "nested", "-P", "custom.toml", "info", "setup"])
            .output()
            .unwrap(),
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["working_directory"], child.to_string_lossy().as_ref());
    assert_eq!(
        value["project"]["project_file"],
        child.join("custom.toml").to_string_lossy().as_ref()
    );
    assert_eq!(value["ruby"]["version"], "3.4.10");
    fs::write(child.join("Dependencies"), "raise 'must not evaluate'\n").unwrap();
    let output = success(
        fixture
            .command()
            .env("BUNDLE_GEMFILE", "nested/Dependencies")
            .args(["info", "setup"])
            .output()
            .unwrap(),
    );
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        value["project"]["gemfile"],
        child.join("Dependencies").to_string_lossy().as_ref()
    );
}

#[test]
fn both_backends_probe_the_external_gemfile_and_its_lockfile() {
    use std::process::Command;
    let ruby = which::which(std::env::var_os("RB_GEM_BUILD_RUBY").unwrap_or_else(|| "ruby".into()))
        .unwrap();
    let version = String::from_utf8(
        Command::new(&ruby)
            .args(["-e", "print RUBY_VERSION"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    for backend in ["bundler", "native"] {
        let fixture = Fixture::new();
        let bin = fixture
            .root
            .path()
            .join(format!("rubies/ruby-{version}/bin"));
        fs::create_dir_all(&bin).unwrap();
        std::os::unix::fs::symlink(&ruby, bin.join("ruby")).unwrap();
        let external = fixture.root.path().join("external");
        fs::create_dir(&external).unwrap();
        fs::write(
            external.join("Dependencies"),
            "source 'https://rubygems.org'\ngem 'rake'\n",
        )
        .unwrap();
        fs::write(external.join("Dependencies.lock"), "fixture\n").unwrap();
        fs::write(
            fixture.root.path().join("Gemfile"),
            "raise 'wrong project'\n",
        )
        .unwrap();
        let output = success(
            fixture
                .command()
                .env("BUNDLE_GEMFILE", "external/Dependencies")
                .args([
                    "-r",
                    &version,
                    "--package-backend",
                    backend,
                    "info",
                    "setup",
                    "--probe",
                ])
                .output()
                .unwrap(),
        );
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            value["project"]["root"],
            external.to_string_lossy().as_ref()
        );
        assert_eq!(
            value["cache"]["lockfile"],
            external
                .join("Dependencies.lock")
                .to_string_lossy()
                .as_ref()
        );
        assert_eq!(value["cache"]["complete"], true);
    }
}

#[test]
fn disabled_packages_do_not_probe_gemfiles_for_either_backend() {
    for backend in ["bundler", "native"] {
        let fixture = Fixture::new();
        fixture.seed();
        success(
            fixture
                .command()
                .args(["install", "ruby", "3.4.10"])
                .output()
                .unwrap(),
        );
        fs::write(
            fixture.root.path().join("Gemfile"),
            "raise 'must not evaluate'\n",
        )
        .unwrap();
        let output = success(
            fixture
                .command()
                .args([
                    "-r",
                    "3.4.10",
                    "-B",
                    "--package-backend",
                    backend,
                    "info",
                    "setup",
                    "--probe",
                ])
                .output()
                .unwrap(),
        );
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(value["project"].is_null());
        assert!(value["cache"].is_null());
    }
}
