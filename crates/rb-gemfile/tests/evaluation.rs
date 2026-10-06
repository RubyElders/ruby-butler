use rb_gemfile::{Evaluator, Manifest, Require};
use std::path::Path;

fn evaluator() -> Evaluator {
    Evaluator::new(std::env::var_os("RB_GEMFILE_RUBY").unwrap_or_else(|| "ruby".into()))
}

fn write(root: &Path, name: &str, content: &str) {
    let path = root.join(name);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, content).unwrap();
}

#[test]
fn evaluates_requirements_groups_and_require_settings_without_bundler() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "Gemfile",
        include_str!(
            "fixtures/evaluation/evaluates_requirements_groups_and_require_settings_without_bundler-1.rb"
        ),
    );
    let result = evaluator().evaluate(root.path().join("Gemfile")).unwrap();
    let manifest = result.manifest;
    assert_eq!(
        manifest.bundle.source.as_deref(),
        Some("https://rubygems.org")
    );
    assert_eq!(manifest.bundle.ruby, [">= 3.2", "< 5"]);
    assert_eq!(manifest.bundle.engine.as_deref(), Some("jruby"));
    assert_eq!(manifest.bundle.optional_groups, ["development", "test"]);
    assert_eq!(
        manifest.dependencies["rails"].version,
        ["~> 8.0", ">= 8.0.2"]
    );
    assert_eq!(manifest.dependencies["rails"].groups, ["default"]);
    assert_eq!(
        manifest.dependencies["debug"].require,
        Some(Require::Enabled(false))
    );
    assert_eq!(
        manifest.dependencies["rake"].groups,
        ["development", "test", "tools"]
    );
    assert_eq!(
        manifest.dependencies["rake"].require,
        Some(Require::Names(vec!["rake".into(), "rake/task".into()]))
    );
    assert_eq!(
        manifest.dependencies["rack"].require,
        Some(Require::Name("rack/builder".into()))
    );
    let toml = manifest.to_toml().unwrap();
    assert_eq!(toml::from_str::<Manifest>(&toml).unwrap(), manifest);
}

#[test]
fn preserves_platforms_and_evaluated_install_conditions() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "Gemfile",
        include_str!(
            "fixtures/evaluation/preserves_platforms_and_evaluated_install_conditions-1.rb"
        ),
    );
    let manifest = evaluator()
        .evaluate(root.path().join("Gemfile"))
        .unwrap()
        .manifest;
    assert_eq!(manifest.dependencies["jdbc"].platforms, ["jruby"]);
    assert_eq!(
        manifest.dependencies["jdbc"].force_ruby_platform,
        Some(true)
    );
    assert!(!manifest.dependencies["disabled"].enabled);
    assert!(!manifest.dependencies["also_disabled"].enabled);
    assert!(manifest.dependencies["enabled"].enabled);
    assert_eq!(
        manifest.dependencies["enabled"].platforms,
        ["ruby", "windows"]
    );
}

#[test]
fn scopes_sources_and_resolves_included_paths() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "Gemfile",
        include_str!("fixtures/evaluation/scopes_sources_and_resolves_included_paths-1.rb"),
    );
    write(
        root.path(),
        "nested/Gemfile",
        "path '../local' do\n gem 'local'\nend\n",
    );
    let result = evaluator().evaluate(root.path().join("Gemfile")).unwrap();
    let deps = result.manifest.dependencies;
    assert_eq!(
        deps["private-gem"].source.as_deref(),
        Some("https://gems.example.test")
    );
    assert_eq!(
        deps["git-gem"].git.as_deref(),
        Some("https://example.test/repo.git")
    );
    assert_eq!(deps["git-gem"].rev.as_deref(), Some("abc123"));
    assert_eq!(deps["git-gem"].submodules, Some(true));
    assert_eq!(
        deps["custom"].git.as_deref(),
        Some("https://example.test/custom.git")
    );
    assert_eq!(deps["custom"].tag.as_deref(), Some("v1"));
    assert_eq!(
        Path::new(deps["local"].path.as_ref().unwrap()),
        dunce::canonicalize(root.path()).unwrap().join("local")
    );
    assert!(deps["normal"].source.is_none());
    assert!(deps["normal"].git.is_none());
    assert_eq!(result.inputs.len(), 2);
}

#[test]
fn loads_project_gemspec_and_ruby_version_file() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "Gemfile",
        "ruby file: '.ruby-version'\ngemspec development_group: :tools\n",
    );
    write(root.path(), ".ruby-version", "ruby-4.0.6\n");
    write(
        root.path(),
        "sample.gemspec",
        include_str!("fixtures/evaluation/loads_project_gemspec_and_ruby_version_file-1.rb"),
    );
    let result = evaluator().evaluate(root.path().join("Gemfile")).unwrap();
    assert_eq!(result.manifest.bundle.ruby, ["4.0.6"]);
    assert_eq!(result.manifest.dependencies["sample"].version, ["= 1.2.3"]);
    assert!(result.manifest.dependencies["sample"].path.is_some());
    assert_eq!(result.manifest.dependencies["rake"].groups, ["tools"]);
    assert_eq!(result.inputs.len(), 3);
}

#[test]
fn keeps_gemfile_output_out_of_the_protocol() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "Gemfile",
        "puts 'not JSON'; warn 'a warning'; gem 'rake'\n",
    );
    let result = evaluator().evaluate(root.path().join("Gemfile")).unwrap();
    assert_eq!(result.stdout.lines().collect::<Vec<_>>(), ["not JSON"]);
    assert_eq!(result.stderr.lines().collect::<Vec<_>>(), ["a warning"]);
    assert!(result.manifest.dependencies.contains_key("rake"));
}

#[test]
fn rejects_unsupported_or_ambiguous_declarations() {
    for (source, message) in [
        ("gem 'rake', mystery: true", "Unsupported gem options"),
        (
            "gem 'rake', '>= 12'; gem 'rake', '< 10'",
            "Conflicting declarations",
        ),
        ("gem 'rake', git: 'url', path: '.'", "Conflicting sources"),
        ("gem 'rake', branch: 'main'", "Git options require"),
        (
            "gem 'rake', git: 'url', branch: 'main', tag: 'v1'",
            "Conflicting Git selectors",
        ),
        ("eval_gemfile 'Gemfile'", "Recursive eval_gemfile"),
        ("source 'one'; source 'two'", "Multiple global sources"),
        (
            "plugin 'unknown', mystery: true",
            "Plugin options are unsupported",
        ),
        ("gem 'rake', '~> nonsense'", "Illformed requirement"),
        (
            "gem 'rake', git: 'url', ref: 'one', rev: 'two'",
            "Specify ref or rev",
        ),
        ("gem 'broken", "SyntaxError"),
    ] {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "Gemfile", source);
        let error = evaluator()
            .evaluate(root.path().join("Gemfile"))
            .unwrap_err()
            .to_string();
        assert!(error.contains(message), "{source}: {error}");
        assert!(error.contains("Gemfile"), "{error}");
    }
}

#[test]
fn separate_evaluations_do_not_leak_state() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "Gemfile", "gem 'first'\n");
    let evaluator = evaluator();
    assert!(
        evaluator
            .evaluate(root.path().join("Gemfile"))
            .unwrap()
            .manifest
            .dependencies
            .contains_key("first")
    );
    write(root.path(), "Gemfile", "gem 'second'\n");
    let second = evaluator.evaluate(root.path().join("Gemfile")).unwrap();
    assert_eq!(second.manifest.dependencies.len(), 1);
    assert!(second.manifest.dependencies.contains_key("second"));
}

#[test]
fn accepts_positional_option_hashes_and_identical_duplicates() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "Gemfile",
        "gem 'rake', {require: false}\ngem 'rake', require: false\n",
    );
    let result = evaluator().evaluate(root.path().join("Gemfile")).unwrap();
    assert_eq!(result.manifest.dependencies.len(), 1);
    assert_eq!(
        result.manifest.dependencies["rake"].require,
        Some(Require::Enabled(false))
    );
}

#[test]
fn reports_missing_inputs_and_missing_ruby() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
        evaluator()
            .evaluate(root.path().join("missing"))
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::NotFound
    );
    write(root.path(), "Gemfile", "gem 'rake'");
    assert_eq!(
        Evaluator::new(root.path().join("missing-ruby"))
            .evaluate(root.path().join("Gemfile"))
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::NotFound
    );
    write(root.path(), "Gemfile", "eval_gemfile 'missing'");
    let error = evaluator()
        .evaluate(root.path().join("Gemfile"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("missing"), "{error}");
}

#[test]
fn preserves_duplicate_declarations_with_different_activation_settings() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "Gemfile",
        include_str!(
            "fixtures/evaluation/preserves_duplicate_declarations_with_different_activation_settings-1.rb"
        ),
    );
    let manifest = evaluator()
        .evaluate(root.path().join("Gemfile"))
        .unwrap()
        .manifest;
    let dependency = &manifest.dependencies["rake"];
    assert_eq!(dependency.groups, ["development"]);
    assert_eq!(dependency.platforms, ["ruby"]);
    assert_eq!(dependency.variants.len(), 1);
    assert_eq!(dependency.variants[0].groups, ["production"]);
    assert_eq!(dependency.variants[0].platforms, ["jruby"]);
    assert_eq!(
        dependency.variants[0].require,
        Some(Require::Name("rake/task".into()))
    );
    assert_eq!(
        toml::from_str::<Manifest>(&manifest.to_toml().unwrap()).unwrap(),
        manifest
    );
}

#[test]
fn evaluates_environment_scopes_and_conditions_for_each_declaration() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "Gemfile",
        include_str!(
            "fixtures/evaluation/evaluates_environment_scopes_and_conditions_for_each_declaration-1.rb"
        ),
    );
    let deps = evaluator()
        .evaluate(root.path().join("Gemfile"))
        .unwrap()
        .manifest
        .dependencies;
    assert!(deps["present"].enabled);
    assert!(!deps["missing"].enabled);
    assert!(deps["matching"].enabled);
    assert!(deps["first"].enabled);
    assert!(!deps["second"].enabled);
    assert!(!deps["all"].enabled);
}

#[test]
fn supports_ruby_version_file_formats_and_custom_lockfiles() {
    for text in [
        "ruby-3.4.5\n",
        "3.4.5\n",
        "nodejs 20\nruby 3.4.5 # comment\n",
        "[tools]\nruby = \"3.4.5\"\n",
        "ruby = '3.4.5'\n",
    ] {
        let root = tempfile::tempdir().unwrap();
        write(
            root.path(),
            "Gemfile",
            "ruby file: 'versions'\nlockfile 'Gemfile.custom.lock'\n",
        );
        write(root.path(), "versions", text);
        let result = evaluator().evaluate(root.path().join("Gemfile")).unwrap();
        assert_eq!(result.manifest.bundle.ruby, ["3.4.5"], "{text}");
        assert!(
            result
                .manifest
                .bundle
                .lockfile
                .unwrap()
                .ends_with("Gemfile.custom.lock")
        );
    }
}

#[test]
fn expands_git_shortcuts_and_preserves_custom_selector_metadata() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "Gemfile",
        include_str!(
            "fixtures/evaluation/expands_git_shortcuts_and_preserves_custom_selector_metadata-1.rb"
        ),
    );
    let deps = evaluator()
        .evaluate(root.path().join("Gemfile"))
        .unwrap()
        .manifest
        .dependencies;
    assert_eq!(
        deps["github"].git.as_deref(),
        Some("https://github.com/rack/rack.git")
    );
    assert_eq!(
        deps["gitlab"].git.as_deref(),
        Some("https://gitlab.com/group/repo.git")
    );
    assert_eq!(
        deps["gist"].git.as_deref(),
        Some("https://gist.github.com/1234.git")
    );
    assert_eq!(
        deps["bitbucket"].git.as_deref(),
        Some("https://owner@bitbucket.org/owner/owner.git")
    );
    assert_eq!(deps["review"].rev.as_deref(), Some("refs/pull/1/head"));
    assert_eq!(deps["rails"].branch.as_deref(), Some("main"));
}

#[test]
fn records_bundler_read_file_inputs_without_loading_bundler() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "Gemfile",
        "instance_eval(Bundler.read_file('extra.rb'))\n",
    );
    write(
        root.path(),
        "extra.rb",
        "gem 'rake'\nraise 'Bundler gem loaded' if $LOADED_FEATURES.any? { |path| path.include?('/bundler/') }\n",
    );
    let result = evaluator().evaluate(root.path().join("Gemfile")).unwrap();
    assert_eq!(result.inputs.len(), 2);
    assert!(result.inputs.iter().any(|path| path.ends_with("extra.rb")));
    assert!(result.manifest.dependencies.contains_key("rake"));
}

#[test]
fn nested_sources_do_not_inherit_git_selectors() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "Gemfile",
        include_str!("fixtures/evaluation/nested_sources_do_not_inherit_git_selectors-1.rb"),
    );
    let result = evaluator().evaluate(root.path().join("Gemfile")).unwrap();
    let deps = result.manifest.dependencies;
    assert_eq!(
        deps["registry"].source.as_deref(),
        Some("https://example.test")
    );
    assert_eq!(deps["registry"].branch, None);
    assert_eq!(deps["local"].branch, None);
    assert_eq!(deps["local"].submodules, None);
    assert_eq!(deps["inner"].tag.as_deref(), Some("v1"));
    assert_eq!(deps["inner"].branch, None);
    assert_eq!(deps["outer"].branch.as_deref(), Some("main"));
    assert_eq!(deps["outer"].submodules, Some(true));
}

#[test]
fn gemspec_name_uses_package_name_and_retains_package_requirements() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "Gemfile", "gemspec name: 'sample'");
    write(
        root.path(),
        "unrelated-filename.gemspec",
        include_str!(
            "fixtures/evaluation/gemspec_name_uses_package_name_and_retains_package_requirements-1.rb"
        ),
    );
    write(
        root.path(),
        "other.gemspec",
        "Gem::Specification.new do |s|\n s.name = 'other'\n s.version = '0.1.0'\nend",
    );
    let manifest = evaluator()
        .evaluate(root.path().join("Gemfile"))
        .unwrap()
        .manifest;
    assert_eq!(manifest.gemspecs.len(), 1);
    let package = &manifest.gemspecs[0];
    assert_eq!(package.name, "sample");
    assert_eq!(package.version, "1.2.3");
    assert_eq!(package.required_ruby_version, [">= 3.2"]);
    assert_eq!(package.runtime_dependencies["rack"], [">= 3", "< 4"]);
    assert_eq!(package.development_dependencies["rake"], ["~> 13"]);
    let roundtrip: Manifest = toml::from_str(&manifest.to_toml().unwrap()).unwrap();
    assert_eq!(roundtrip, manifest);
    let merged: toml::Value = toml::from_str(&manifest.merge_project("").unwrap()).unwrap();
    assert_eq!(merged["gemspecs"][0]["name"].as_str(), Some("sample"));
}

#[test]
fn gemfile_activation_wins_over_gemspec_development_declaration_in_both_orders() {
    for source in [
        "gemspec; gem 'rake', '>= 13.2', group: :tools, require: false",
        "gem 'rake', '>= 13.2', group: :tools, require: false; gemspec",
    ] {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "Gemfile", source);
        write(
            root.path(),
            "sample.gemspec",
            include_str!(
                "fixtures/evaluation/gemfile_activation_wins_over_gemspec_development_declaration_in_both_orders-1.rb"
            ),
        );
        let result = evaluator().evaluate(root.path().join("Gemfile")).unwrap();
        let rake = &result.manifest.dependencies["rake"];
        assert_eq!(rake.groups, ["tools"]);
        assert_eq!(rake.require, Some(Require::Enabled(false)));
        assert!(rake.version.contains(&"~> 13".to_owned()));
        assert!(rake.version.contains(&">= 13.2".to_owned()));
    }
}

#[test]
fn rejects_invalid_platform_and_boolean_options() {
    for source in [
        "gem 'rake', platform: :linux",
        "gem 'rake', submodules: 'yes', git: 'url'",
        "gem 'rake', force_ruby_platform: 'yes'",
    ] {
        let root = tempfile::tempdir().unwrap();
        write(root.path(), "Gemfile", source);
        assert!(
            evaluator().evaluate(root.path().join("Gemfile")).is_err(),
            "{source}"
        );
    }
}

#[test]
fn explicit_dependency_sources_override_scope_sources() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "Gemfile",
        include_str!("fixtures/evaluation/explicit_dependency_sources_override_scope_sources-1.rb"),
    );
    let deps = evaluator()
        .evaluate(root.path().join("Gemfile"))
        .unwrap()
        .manifest
        .dependencies;
    assert_eq!(
        deps["registry"].source.as_deref(),
        Some("https://example.test")
    );
    assert_eq!(deps["registry"].git, None);
    assert_eq!(deps["registry"].branch, None);
    assert!(deps["local"].path.is_some());
    assert_eq!(deps["local"].branch, None);
    assert_eq!(deps["other"].git.as_deref(), Some("other"));
    assert_eq!(deps["other"].branch, None);
    assert_eq!(deps["other"].tag.as_deref(), Some("v1"));
}

#[test]
fn successful_exit_without_manifest_is_an_evaluation_error() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "Gemfile", "exit 0");
    let error = evaluator()
        .evaluate(root.path().join("Gemfile"))
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("Ruby did not produce a Gemfile manifest")
    );
}

#[test]
fn a_directory_cannot_be_evaluated_as_a_gemfile() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
        evaluator().evaluate(root.path()).unwrap_err().kind(),
        std::io::ErrorKind::InvalidInput
    );
}

#[test]
fn duplicate_gemspec_requirements_are_combined_without_losing_bounds() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "Gemfile",
        "gemspec({glob: 'packages/*.gemspec'}); ruby('>= 3.2', {engine: 'jruby', engine_version: '10.0.0'})",
    );
    write(
        root.path(),
        "sample.gemspec",
        include_str!(
            "fixtures/evaluation/duplicate_gemspec_requirements_are_combined_without_losing_bounds-1.rb"
        ),
    );
    let manifest = evaluator()
        .evaluate(root.path().join("Gemfile"))
        .unwrap()
        .manifest;
    assert_eq!(
        manifest.gemspecs[0].runtime_dependencies["rack"],
        [">= 3", "< 4"]
    );
    assert_eq!(
        manifest.gemspecs[0].development_dependencies["rake"],
        [">= 13", "< 14"]
    );
    assert_eq!(manifest.dependencies["rake"].version, [">= 13", "< 14"]);
    assert_eq!(
        manifest.dependencies["sample"].glob.as_deref(),
        Some("packages/*.gemspec")
    );
    assert_eq!(manifest.bundle.engine.as_deref(), Some("jruby"));
    assert_eq!(manifest.bundle.engine_version.as_deref(), Some("10.0.0"));
}

#[test]
fn selects_the_local_platform_variant_of_a_package_gemspec() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "Gemfile",
        "puts Gem::Platform.local.to_s; gemspec",
    );
    write(
        root.path(),
        "sample.gemspec",
        include_str!(
            "fixtures/evaluation/selects_the_local_platform_variant_of_a_package_gemspec-1.rb"
        ),
    );
    write(
        root.path(),
        "sample-native.gemspec",
        include_str!(
            "fixtures/evaluation/selects_the_local_platform_variant_of_a_package_gemspec-2.rb"
        ),
    );
    let result = evaluator().evaluate(root.path().join("Gemfile")).unwrap();
    assert_eq!(result.manifest.gemspecs.len(), 1);
    assert_eq!(result.manifest.gemspecs[0].platform, result.stdout.trim());
    assert!(
        result.manifest.gemspecs[0]
            .runtime_dependencies
            .contains_key("native")
    );
    assert!(
        !result.manifest.gemspecs[0]
            .runtime_dependencies
            .contains_key("generic")
    );
}

#[test]
fn inherited_rubyopt_cannot_activate_bundler_or_break_evaluation() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "Gemfile",
        "raise 'RUBYOPT retained' if ENV.key?('RUBYOPT'); gem 'rake'",
    );
    let result = evaluator()
        .env("RUBYOPT", "-rrb_gemfile_nonexistent_library")
        .evaluate(root.path().join("Gemfile"))
        .unwrap();
    assert!(result.manifest.dependencies.contains_key("rake"));
}

#[test]
fn source_options_preserve_dependencies_without_exporting_cooldowns() {
    let manifest = evaluator()
        .evaluate(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/ruby/source_options/Gemfile"),
        )
        .unwrap()
        .manifest;
    assert_eq!(
        manifest.bundle.source.as_deref(),
        Some("https://rubygems.org")
    );
    assert_eq!(
        manifest.dependencies["scoped"].source.as_deref(),
        Some("https://example.test")
    );
    let toml = manifest.to_toml().unwrap();
    assert_eq!(toml::from_str::<Manifest>(&toml).unwrap(), manifest);
    assert!(!toml.contains("cooldown"));
    let kdl = manifest.to_kdl().unwrap();
    assert!(kdl.parse::<kdl::KdlDocument>().is_ok());
    assert!(!kdl.contains("cooldown"));
}

#[test]
fn explicit_nil_require_disables_autorequire_in_exports() {
    let manifest = evaluator()
        .evaluate(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/ruby/require_settings/Gemfile"),
        )
        .unwrap()
        .manifest;
    assert_eq!(
        manifest.dependencies["nil-require"].require,
        Some(Require::Enabled(false))
    );
    assert_eq!(
        manifest.dependencies["false-require"].require,
        Some(Require::Enabled(false))
    );
    assert_eq!(manifest.dependencies["default-require"].require, None);
    assert_eq!(
        toml::from_str::<Manifest>(&manifest.to_toml().unwrap()).unwrap(),
        manifest
    );
    let parsed = Manifest::from_kdl(&manifest.to_kdl().unwrap()).unwrap();
    assert_eq!(parsed, manifest);
}

#[test]
fn compatibility_helpers_preserve_manifest_and_input_tracking() {
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ruby/compatibility_helpers");
    let evaluation = evaluator().evaluate(fixture.join("Gemfile")).unwrap();
    assert_eq!(evaluation.manifest.dependencies.len(), 3);
    assert!(!evaluation.manifest.dependencies.contains_key("keep"));
    assert_eq!(
        evaluation.manifest.dependencies["replace"].version,
        ["~> 3"]
    );
    assert_eq!(evaluation.manifest.dependencies["rack"].version, ["~> 3"]);
    assert_eq!(
        evaluation.manifest.dependencies["tzinfo-data"].platforms,
        ["windows"]
    );
    assert!(
        evaluation
            .inputs
            .contains(&dunce::canonicalize(fixture.join("sample.gemspec")).unwrap())
    );
    assert_eq!(
        toml::from_str::<Manifest>(&evaluation.manifest.to_toml().unwrap()).unwrap(),
        evaluation.manifest
    );
}
