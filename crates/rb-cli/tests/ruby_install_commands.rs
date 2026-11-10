#![cfg(unix)]
#[path = "common/ruby_install.rs"]
mod support;
use rb_tests::HttpServer;
use std::fs;
use support::{Fixture, success};

#[test]
fn explicit_install_respects_working_directory_global_paths_and_disabled_automatic_install() {
    let fixture = Fixture::new();
    fixture.seed();
    success(
        fixture
            .command()
            .args(["--no-auto-install-ruby", "install", "ruby", "3.4.10"])
            .output()
            .unwrap(),
    );
    assert!(fixture.installed());
    let output = success(
        fixture
            .command()
            .args(["install", "ruby", "3.4.10"])
            .output()
            .unwrap(),
    );
    assert!(!String::from_utf8_lossy(&output.stderr).contains("Downloading"));
    assert!(
        fixture
            .root
            .path()
            .join("rubies/ruby-3.4.10/.rb-install.json")
            .is_file()
    );
}

#[test]
fn exec_and_run_install_before_launch_and_reuse_without_reporter() {
    for backend in ["bundler", "native"] {
        for args in [vec!["x", "ruby", "sentinel"], vec!["r", "smoke"]] {
            let fixture = Fixture::new();
            fixture.seed();
            fs::write(fixture.root.path().join(".ruby-version"), "3.4.10\n").unwrap();
            fs::write(
                fixture.root.path().join("rbproject.toml"),
                "[scripts]\nsmoke = 'ruby sentinel'\n",
            )
            .unwrap();
            let output = success(
                fixture
                    .command()
                    .args(["--package-backend", backend, "-B"])
                    .args(&args)
                    .output()
                    .unwrap(),
            );
            assert!(fixture.installed());
            assert!(String::from_utf8_lossy(&output.stdout).contains("Ruby command executed"));
            assert!(String::from_utf8_lossy(&output.stderr).contains("Validating installation"));
            let output = success(
                fixture
                    .command()
                    .args(["--package-backend", backend, "-B"])
                    .args(&args)
                    .output()
                    .unwrap(),
            );
            assert!(output.stderr.is_empty(), "{:?}", output);
        }
    }
}

#[test]
fn prefixed_version_file_installs_and_reuses_cruby() {
    for backend in ["bundler", "native"] {
        let fixture = Fixture::new();
        fixture.seed();
        fs::write(fixture.root.path().join(".ruby-version"), "ruby-3.4.10\n").unwrap();
        for first in [true, false] {
            let output = success(
                fixture
                    .command()
                    .args(["--package-backend", backend, "-B", "x", "ruby", "sentinel"])
                    .output()
                    .unwrap(),
            );
            assert!(fixture.installed());
            assert!(String::from_utf8_lossy(&output.stdout).contains("Ruby command executed"));
            if first {
                assert!(String::from_utf8_lossy(&output.stderr).contains("Installed Ruby 3.4.10"));
            } else {
                assert!(output.stderr.is_empty(), "{output:?}");
            }
        }
    }
}

#[test]
fn verbose_execution_reports_one_scan_before_runtime_composition() {
    let fixture = Fixture::new();
    fixture.seed();
    success(
        fixture
            .command()
            .args(["install", "ruby", "3.4.10"])
            .output()
            .unwrap(),
    );
    fs::write(fixture.root.path().join(".ruby-version"), "3.4.10\n").unwrap();
    fs::write(
        fixture.root.path().join("rbproject.toml"),
        "[scripts]\nsmoke = 'ruby sentinel'\n",
    )
    .unwrap();
    for backend in ["bundler", "native"] {
        for args in [vec!["x", "ruby", "sentinel"], vec!["r", "smoke"]] {
            let output = success(
                fixture
                    .command()
                    .args(["-V", "-B", "--package-backend", backend])
                    .args(args)
                    .output()
                    .unwrap(),
            );
            let log = String::from_utf8(output.stderr).unwrap();
            let start = "Starting Ruby discovery in directory:";
            let summary = "Discovered 1 Ruby installations in";
            let composition = "Environment composition complete: Ruby 3.4.10";
            assert_eq!(log.matches(start).count(), 1, "{log}");
            assert_eq!(log.matches(summary).count(), 1, "{log}");
            assert!(
                log.find(start).unwrap() < log.find(summary).unwrap(),
                "{log}"
            );
            assert!(
                log.find(summary).unwrap() < log.find(composition).unwrap(),
                "{log}"
            );
        }
    }
}

#[test]
fn sync_installs_before_package_preparation() {
    let fixture = Fixture::new();
    fixture.seed();
    fs::write(
        fixture.root.path().join("Gemfile"),
        "source 'https://rubygems.org'\nruby '3.4.10'\n",
    )
    .unwrap();
    let output = success(
        fixture
            .command()
            .args(["--package-backend", "bundler", "sync"])
            .output()
            .unwrap(),
    );
    assert!(fixture.installed());
    let text = String::from_utf8_lossy(&output.stderr);
    assert_eq!(text.lines().next(), Some("[overall] Preparing your bundle"));
    assert_eq!(
        text.matches("[overall] Preparing your bundle").count(),
        1,
        "{text}"
    );
    assert!(
        text.contains("[Installing Ruby 3.4.10 | overall] Validating installation"),
        "{text}"
    );
    assert!(
        !text.contains("[overall] complete: Installed Ruby"),
        "{text}"
    );
    assert!(
        text.contains("[overall] complete: Everything is already in order"),
        "{text}"
    );
    assert!(
        text.find("Validating installation").unwrap() < text.find("checking dependencies").unwrap(),
        "{text}"
    );
}

#[test]
fn disabled_install_never_uses_another_installed_version() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.root.path().join("rubies/ruby-4.0.7/bin")).unwrap();
    fs::write(fixture.root.path().join(".ruby-version"), "3.4.10\n").unwrap();
    for args in [
        vec!["x", "ruby", "sentinel"],
        vec!["r", "smoke"],
        vec!["sync"],
    ] {
        let output = fixture
            .command()
            .args(["--no-auto-install-ruby"])
            .args(args)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("rb install ruby 3.4.10"),
            "{:?}",
            output
        );
        assert!(!fixture.installed());
    }
}

#[test]
fn diagnostics_completion_and_listing_do_not_install() {
    let fixture = Fixture::new();
    fixture.seed();
    fs::write(
        fixture.root.path().join("rbproject.toml"),
        "[scripts]\nsmoke = 'ruby sentinel'\n",
    )
    .unwrap();
    for args in [
        vec!["info", "config"],
        vec!["info", "runtime"],
        vec!["help"],
        vec!["__bash_complete", "rb r", "4"],
        vec!["r"],
        vec!["x"],
    ] {
        let output = fixture.command().args(args).output().unwrap();
        assert!(!fixture.installed());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("Installing Ruby"));
    }
    let output = success(
        fixture
            .command()
            .args(["info", "runtime", "--available"])
            .output()
            .unwrap(),
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("3.4.10"));
    assert!(!fixture.installed());
}

#[test]
fn project_manifest_and_global_ruby_override_are_used() {
    for (name, contents) in [
        ("custom.toml", "[bundle]\nruby = ['~> 3.4.0']\n"),
        ("custom.kdl", "bundle { ruby \"~> 3.4.0\"; }\n"),
    ] {
        let fixture = Fixture::new();
        fixture.seed();
        fs::write(fixture.root.path().join(name), contents).unwrap();
        success(
            fixture
                .command()
                .args(["-P", name, "install", "ruby"])
                .output()
                .unwrap(),
        );
        assert!(fixture.installed());
        let output = fixture
            .command()
            .args([
                "-P",
                name,
                "-r",
                "3.3.9",
                "--no-auto-install-ruby",
                "x",
                "ruby",
            ])
            .output()
            .unwrap();
        assert!(String::from_utf8_lossy(&output.stderr).contains("rb install ruby 3.3.9"));
    }
}

#[test]
fn gemfile_version_is_used_when_project_only_defines_scripts() {
    let fixture = Fixture::new();
    fs::write(
        fixture.root.path().join("rbproject.toml"),
        "[scripts]\nsmoke='ruby sentinel'\n",
    )
    .unwrap();
    fs::write(fixture.root.path().join("Gemfile"), "ruby '3.4.10'\n").unwrap();
    let output = fixture
        .command()
        .args(["--no-auto-install-ruby", "x", "ruby"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("rb install ruby 3.4.10"));
}

#[test]
fn custom_gemfile_and_working_directory_are_respected() {
    let fixture = Fixture::new();
    fixture.seed();
    let project = fixture.root.path().join("project");
    fs::create_dir(&project).unwrap();
    fs::write(project.join("custom.rb"), "ruby '3.4.10'\n").unwrap();
    let output = success(
        fixture
            .command()
            .args(["-C", "project", "install", "ruby"])
            .env("BUNDLE_GEMFILE", "custom.rb")
            .output()
            .unwrap(),
    );
    assert!(fixture.installed(), "{:?}", output);
    assert!(!project.join("rubies").exists());
}

#[test]
fn config_and_environment_can_disable_install_and_cli_can_enable_it_again() {
    for environment in [false, true] {
        let fixture = Fixture::new();
        fixture.seed();
        if !environment {
            fs::write(
                fixture.root.path().join("config.toml"),
                "auto-install-ruby = false\n",
            )
            .unwrap();
        }
        let mut command = fixture.command();
        command.args(["-B", "-r", "3.4.10", "x", "ruby", "sentinel"]);
        if environment {
            command.env("RB_AUTO_INSTALL_RUBY", "false");
        }
        let output = command.output().unwrap();
        assert!(!output.status.success());
        assert!(!fixture.installed());
        assert!(String::from_utf8_lossy(&output.stderr).contains("rb install ruby 3.4.10"));
        let mut command = fixture.command();
        command.args([
            "--auto-install-ruby",
            "-B",
            "-r",
            "3.4.10",
            "x",
            "ruby",
            "sentinel",
        ]);
        if environment {
            command.env("RB_AUTO_INSTALL_RUBY", "false");
        }
        success(command.output().unwrap());
        assert!(fixture.installed());
    }
}

#[test]
fn literal_gemfile_requirements_keep_all_bounds() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.root.path().join("rubies/ruby-4.0.7/bin")).unwrap();
    fs::write(
        fixture.root.path().join("Gemfile"),
        "ruby '>= 3.3', '< 3.4'\n",
    )
    .unwrap();
    let output = fixture
        .command()
        .args(["--no-auto-install-ruby", "x", "ruby"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let text = String::from_utf8_lossy(&output.stderr);
    assert!(
        text.contains("Ruby >= 3.3, < 3.4 is not installed"),
        "{text}"
    );
}

#[test]
fn selected_gemfile_does_not_inherit_an_unselected_gemfile_version() {
    let fixture = Fixture::new();
    fs::write(fixture.root.path().join("Gemfile"), "ruby '3.3.9'\n").unwrap();
    fs::write(
        fixture.root.path().join("custom.rb"),
        "source 'https://rubygems.org'\n",
    )
    .unwrap();
    let output = fixture
        .command()
        .env("BUNDLE_GEMFILE", "custom.rb")
        .args(["--no-auto-install-ruby", "x", "ruby"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let text = String::from_utf8_lossy(&output.stderr);
    assert!(text.contains("rb install ruby latest"), "{text}");
    assert!(!text.contains("3.3.9"), "{text}");
}

#[test]
fn validation_can_activate_bundled_gems_without_the_callers_bundle_environment() {
    let fixture = Fixture::new();
    fixture.seed();
    success(
        fixture
            .command()
            .env("RUBYGEMS_GEMDEPS", "-")
            .env("RUBYOPT", "-rmissing_host_gem")
            .env("GEM_HOME", "/missing/gems")
            .env("GEM_PATH", "/missing/gems")
            .env("RUBYLIB", "/missing/ruby")
            .args(["install", "ruby", "3.4.10"])
            .output()
            .unwrap(),
    );
    assert!(fixture.installed());
}

#[test]
fn download_counts_are_formatted_as_sizes_until_the_phase_finishes() {
    let fixture = Fixture::new();
    fixture.seed();
    let cache = fixture.root.path().join("cache/rb/rubies");
    let catalog_path = cache.join("catalog.json");
    let mut catalog: serde_json::Value =
        serde_json::from_slice(&fs::read(&catalog_path).unwrap()).unwrap();
    let asset = &mut catalog["release"]["assets"][0];
    let digest = asset["digest"]
        .as_str()
        .unwrap()
        .strip_prefix("sha256:")
        .unwrap();
    let archive = cache.join(format!("archives/{digest}.tar.gz"));
    let bytes = fs::read(&archive).unwrap();
    fs::remove_file(archive).unwrap();
    let server = HttpServer::new();
    asset["browser_download_url"] = serde_json::json!(format!("{}/ruby.tar.gz", server.url));
    server.route("/ruby.tar.gz", 200, bytes);
    fs::write(catalog_path, serde_json::to_vec(&catalog).unwrap()).unwrap();
    let output = success(
        fixture
            .command()
            .args(["install", "ruby", "3.4.10"])
            .output()
            .unwrap(),
    );
    let text = String::from_utf8_lossy(&output.stderr);
    assert!(
        text.lines().any(|line| line.contains("Downloading")
            && line.contains(" B / ")
            && line.contains("(100%)")),
        "{text}"
    );
    assert!(!text.contains("Downloading 1/1"), "{text}");
    assert!(
        text.find("Downloading").unwrap() < text.find("Verifying archive").unwrap(),
        "{text}"
    );
}

#[test]
fn nested_command_completion_comes_from_the_cli_grammar_without_installing() {
    let fixture = Fixture::new();
    for (line, expected) in [
        ("rb install ", vec!["ruby"]),
        ("rb install r", vec!["ruby"]),
        ("rb install ruby ", vec![]),
        ("rb r", vec!["r", "run"]),
        (
            "rb i ",
            vec!["config", "env", "project", "runtime", "setup"],
        ),
    ] {
        let output = success(
            fixture
                .command()
                .args(["__bash_complete", line, &line.len().to_string()])
                .output()
                .unwrap(),
        );
        let mut candidates: Vec<_> = String::from_utf8(output.stdout)
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect();
        candidates.sort();
        assert_eq!(candidates, expected, "{line}");
        assert!(output.stderr.is_empty(), "{line}");
        assert!(!fixture.root.path().join("rubies").exists());
        assert!(!fixture.root.path().join("cache").exists());
    }
}

#[test]
fn installation_reports_the_selected_version_and_distinguishes_reuse() {
    let fixture = Fixture::new();
    fixture.seed();
    let first = success(
        fixture
            .command()
            .args(["install", "ruby", "3.4"])
            .output()
            .unwrap(),
    );
    let text = String::from_utf8_lossy(&first.stderr);
    assert_eq!(text.lines().next(), Some("[overall] Installing Ruby 3.4"));
    assert!(text.contains("Selected Ruby 3.4.10"), "{text}");
    assert!(text.contains("Installed Ruby 3.4.10"), "{text}");
    assert!(
        text.find("Selected Ruby 3.4.10").unwrap() < text.find("Installed Ruby 3.4.10").unwrap(),
        "{text}"
    );
    assert!(!text.contains("Selecting Ruby 1/1"), "{text}");
    let second = success(
        fixture
            .command()
            .args(["install", "ruby", "3.4"])
            .output()
            .unwrap(),
    );
    let text = String::from_utf8_lossy(&second.stderr);
    assert!(text.contains("Selected Ruby 3.4.10"), "{text}");
    assert!(text.contains("Ruby 3.4.10 is already installed"), "{text}");
    assert!(
        text.find("Selected Ruby 3.4.10").unwrap()
            < text.find("Ruby 3.4.10 is already installed").unwrap(),
        "{text}"
    );
    assert!(!text.contains("Extracting archive"), "{text}");
    let exact = success(
        fixture
            .command()
            .args(["install", "ruby", "3.4.10"])
            .output()
            .unwrap(),
    );
    let text = String::from_utf8_lossy(&exact.stderr);
    assert!(text.contains("Selected Ruby 3.4.10"), "{text}");
    assert!(text.contains("Ruby 3.4.10 is already installed"), "{text}");
    assert!(
        text.find("Selected Ruby 3.4.10").unwrap()
            < text.find("Ruby 3.4.10 is already installed").unwrap(),
        "{text}"
    );
    assert!(!text.contains("Extracting archive"), "{text}");
}

#[test]
fn automatic_install_failure_stops_the_shared_preparation() {
    let fixture = Fixture::new();
    fixture.seed();
    fs::write(fixture.root.path().join(".ruby-version"), "3.3.0\n").unwrap();
    let output = fixture.command().arg("sync").output().unwrap();
    assert!(!output.status.success());
    let text = String::from_utf8_lossy(&output.stderr);
    assert!(text.contains("[overall] Preparing your bundle"), "{text}");
    assert!(
        text.contains("[Installing Ruby 3.3.0 | overall] failed"),
        "{text}"
    );
    assert!(text.contains("[overall] failed"), "{text}");
    assert!(!text.contains("[overall] complete"), "{text}");
    assert!(!text.contains("checking dependencies"), "{text}");
    assert!(!fixture.installed());
}
