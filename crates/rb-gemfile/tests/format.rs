use rb_gemfile::{Evaluator, ExportFormat, Manifest};

fn evaluate() -> Manifest {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("Gemfile"),
        include_str!("fixtures/format/evaluate-1.rb"),
    )
    .unwrap();
    Evaluator::new(std::env::var_os("RB_GEMFILE_RUBY").unwrap_or_else(|| "ruby".into()))
        .evaluate(root.path().join("Gemfile"))
        .unwrap()
        .manifest
}

fn round_trip(manifest: &Manifest) {
    let toml = manifest.to_toml().unwrap();
    let parsed: Manifest = toml::from_str(&toml).unwrap();
    let kdl = manifest.to_kdl().unwrap();
    let from_kdl = Manifest::from_kdl(&kdl).unwrap();
    assert_eq!(parsed, from_kdl);
    assert_eq!(parsed.to_toml().unwrap(), toml);
    assert_eq!(from_kdl.to_kdl().unwrap(), kdl);
    assert!(!toml.contains("variants"));
    assert!(!kdl.contains("variant"));
}

#[test]
fn explicit_sources_preserve_default_registry_and_local_dependencies() {
    let manifest = evaluate();
    round_trip(&manifest);
    let output = manifest.to_toml().unwrap();
    let document: toml::Value = toml::from_str(&output).unwrap();
    let sources = document["sources"].as_array().unwrap();
    assert_eq!(sources.len(), 2);
    assert!(sources.iter().any(
        |source| source["url"].as_str() == Some("https://rubygems.org")
            && source["default"].as_bool() == Some(true)
    ));
    assert!(document["bundle"].get("source").is_none());
    assert!(output.contains("rails = \"~> 8.0\""));
    assert!(output.contains("[sources.dependencies]"));
    assert!(output.contains("[[sources.groups]]"));
    assert!(!output.contains("enabled"));
    assert!(!output.contains("excluded"));
    let parsed: Manifest = toml::from_str(&output).unwrap();
    assert_eq!(parsed.dependencies["debug"].groups, ["development"]);
    assert_eq!(
        parsed.dependencies["internal"].source.as_deref(),
        Some("https://gems.example.com")
    );
    assert_eq!(
        parsed.dependencies["git-gem"],
        manifest.dependencies["git-gem"]
    );
    assert_eq!(parsed.dependencies["rails"], manifest.dependencies["rails"]);
    assert!(
        manifest
            .to_kdl()
            .unwrap()
            .contains("source \"https://rubygems.org\" default=#true")
    );
}

#[test]
fn legacy_input_remains_readable_and_conflicting_sources_are_rejected() {
    let manifest = evaluate();
    let output = manifest.merge_project(include_str!("fixtures/format/reads_mixed_and_expanded_dependencies_and_checks_source_conflicts-1.toml")).unwrap();
    let parsed: Manifest = toml::from_str(&output).unwrap();
    assert_eq!(parsed.dependencies["extra"].version, [">= 1", "< 2"]);
    assert_eq!(
        parsed.dependencies["extra"].source.as_deref(),
        Some("https://gems.example.com")
    );
    for input in [
        "[[sources]]\nurl='https://other.test'\n[sources.dependencies]\nrails='~> 8.0'",
        "[[sources]]\nurl='https://other.test'\n[sources.dependencies]\nx={git='https://github.com/example/x'}",
        "[[sources]]\nurl='https://other.test'\n[sources.dependencies]\nx={source='https://another.test'}",
        "[[sources]]\nurl='https://other.test'\ndefault=true",
    ] {
        assert!(manifest.merge_project(input).is_err(), "{input}");
    }
}

#[test]
fn repeated_declarations_and_escaping_survive_both_formats() {
    for input in [
        include_str!("fixtures/format/complex_variants_and_escaping_survive_both_formats-1.toml"),
        include_str!(
            "fixtures/format/inline_sources_and_variants_with_different_sources_remain_unambiguous-1.toml"
        ),
        include_str!("fixtures/format/explicit_sources.toml"),
    ] {
        let manifest: Manifest = toml::from_str(input).unwrap();
        round_trip(&manifest);
        assert_eq!(
            toml::from_str::<Manifest>(&manifest.to_toml().unwrap()).unwrap(),
            manifest
        );
    }
}

#[test]
fn source_and_group_nesting_is_equivalent() {
    let first = Manifest::from_kdl(include_str!("fixtures/format/source_group.kdl")).unwrap();
    let second = Manifest::from_kdl(include_str!("fixtures/format/group_source.kdl")).unwrap();
    assert_eq!(first, second);
    assert_eq!(
        first.dependencies["internal"].groups,
        ["development", "test"]
    );
    round_trip(&first);
}

#[test]
fn malformed_or_conflicting_blocks_are_rejected() {
    for input in [
        "source \"one\" default=#true; source \"two\" default=#true",
        "source \"one\" { gem \"x\" source=\"two\"; }",
        "source \"one\" { gem \"x\" git=\"two\"; }",
        "group { gem \"x\"; }",
        "gem \"x\" unknown=#true",
        "gem \"x\" require=#false require=#true",
        "source \"one\" unknown=#true",
        "group \"test\" { gem \"x\" { groups \"development\"; }; }",
    ] {
        assert!(Manifest::from_kdl(input).is_err(), "{input}");
    }
}

#[test]
fn kdl_preserves_project_metadata() {
    let output = evaluate()
        .merge_project_format(
            include_str!(
                "fixtures/format/kdl_uses_source_blocks_and_preserves_metadata_and_options-1.toml"
            ),
            ExportFormat::Kdl,
        )
        .unwrap();
    let document: kdl::KdlDocument = output.parse().unwrap();
    assert_eq!(
        document
            .get("project")
            .unwrap()
            .children()
            .unwrap()
            .get("name")
            .unwrap()
            .get(0)
            .unwrap()
            .as_string(),
        Some("tea room")
    );
    assert!(output.starts_with("// Generated by Ruby Butler. Do not edit.\n"));
}

#[test]
fn repeated_gems_in_the_same_context_and_singleton_arrays_round_trip() {
    let input = include_str!("fixtures/format/repeated_declarations.toml");
    let manifest: Manifest = toml::from_str(input).unwrap();
    round_trip(&manifest);
    assert_eq!(manifest.dependencies["rake"].variants.len(), 1);
    assert_eq!(
        manifest.dependencies["rake"].require,
        Some(rb_gemfile::Require::Names(vec!["rake/task".into()]))
    );
    let document: toml::Value = toml::from_str(&manifest.to_toml().unwrap()).unwrap();
    let source = &document["sources"][0];
    assert_eq!(source["groups"].as_array().unwrap().len(), 1);
    assert_eq!(
        source["groups"][0]["dependencies"]["rake"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn invalid_toml_source_and_group_settings_are_rejected() {
    for input in [
        "[[sources]]\nurl='one'\ndefault=true\n[sources.dependencies]\nx={git='two'}",
        "[[sources]]\nurl='one'\ndefault=true\n[sources.dependencies]\nx={source='two'}",
        "[[sources]]\nurl=''",
        "[[sources]]\nurl='one'\ndefault='yes'",
        "[[sources]]\nurl='one'\nunknown=true",
        "[[groups]]\nnames=[]",
        "[dependencies]\nx=[]",
    ] {
        assert!(toml::from_str::<Manifest>(input).is_err(), "{input}");
    }
    let manifest: Manifest = toml::from_str("").unwrap();
    round_trip(&manifest);
}

#[test]
fn explicit_pins_to_the_default_registry_are_preserved() {
    let manifest: Manifest =
        toml::from_str(include_str!("fixtures/format/default_registry_pin.toml")).unwrap();
    assert_eq!(manifest.dependencies["implicit"].source, None);
    assert_eq!(
        manifest.dependencies["pinned"].source.as_deref(),
        Some("https://rubygems.org")
    );
    round_trip(&manifest);
}

#[test]
fn malformed_repeated_declarations_return_errors() {
    assert!(Manifest::from_kdl("gem x variants=wrong; gem x").is_err());
    for input in [
        "[[sources]]\nurl='one'\n[sources.dependencies]\nx=[]",
        "[[groups]]\nnames=['test']\n[groups.dependencies]\nx=[]",
    ] {
        assert!(toml::from_str::<Manifest>(input).is_err(), "{input}");
    }
}

#[test]
fn project_metadata_named_like_dependency_blocks_survives_export() {
    let input = "[project]\ngroups=['staff']\nsources=['manual']";
    let expected: toml::Value = toml::from_str(input).unwrap();
    let output = evaluate().merge_project(input).unwrap();
    let actual: toml::Value = toml::from_str(&output).unwrap();
    assert_eq!(actual["project"], expected["project"]);
}

#[test]
fn gemfile_toml_and_kdl_examples_have_the_same_declarations() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(
        root.path().join("Gemfile"),
        include_str!("fixtures/format/multi_source/Gemfile"),
    )
    .unwrap();
    let evaluated =
        Evaluator::new(std::env::var_os("RB_GEMFILE_RUBY").unwrap_or_else(|| "ruby".into()))
            .evaluate(root.path().join("Gemfile"))
            .unwrap()
            .manifest;
    let exported: Manifest = toml::from_str(&evaluated.to_toml().unwrap()).unwrap();
    let toml: Manifest =
        toml::from_str(include_str!("fixtures/format/multi_source/project.toml")).unwrap();
    let kdl = Manifest::from_kdl(include_str!("fixtures/format/multi_source/project.kdl")).unwrap();
    assert_eq!(exported, toml);
    assert_eq!(toml, kdl);
    assert_eq!(toml.dependencies.len(), 5);
    assert_eq!(toml.dependencies["rake"].variants.len(), 1);
    assert_eq!(toml.dependencies["plugin"].platforms, ["ruby"]);
    round_trip(&toml);
}

#[test]
fn package_dependency_named_item_remains_a_table() {
    let manifest: Manifest =
        toml::from_str(include_str!("fixtures/format/package_named_item.toml")).unwrap();
    assert_eq!(manifest.gemspecs[0].runtime_dependencies["item"], ["~> 1"]);
    round_trip(&manifest);
}
