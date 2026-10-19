mod common;
use rb_packages::ProjectInput;
use std::fs;

#[test]
fn gemfile_snapshot_ignores_generated_bundle_but_tracks_source_edits() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("Gemfile"),
        "source 'https://rubygems.org'\ngem 'rake'\n",
    )
    .unwrap();
    let input = ProjectInput::load(root.path(), common::ruby()).unwrap();
    assert!(!input.reused);
    fs::create_dir_all(root.path().join(".rb/bundle/gems")).unwrap();
    fs::write(root.path().join(".rb/bundle/gems/generated"), "data").unwrap();
    let reused = ProjectInput::load(root.path(), common::ruby()).unwrap();
    assert!(reused.reused);
    fs::write(
        root.path().join("Gemfile"),
        "source 'https://rubygems.org'\ngem 'rack'\n",
    )
    .unwrap();
    let changed = ProjectInput::load(root.path(), common::ruby()).unwrap();
    assert!(!changed.reused);
    assert!(changed.manifest.dependencies.contains_key("rack"));
}

#[test]
fn explicit_project_dependencies_are_authoritative_without_running_gemfile() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("Gemfile"), "raise 'must not execute'\n").unwrap();
    fs::write(
        root.path().join("rbproject.toml"),
        "[project]\nname='example'\n[dependencies]\nrake='>= 0'\n",
    )
    .unwrap();
    let input = ProjectInput::load(root.path(), "missing-ruby").unwrap();
    assert!(input.manifest.dependencies.contains_key("rake"));
    assert!(input.gemfile.is_none());
}

#[test]
fn custom_lockfile_path_cannot_silently_diverge_from_bundler() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("rbproject.toml"),
        "[bundle]\nlockfile='custom.lock'\n[dependencies]\nrake='>= 0'\n",
    )
    .unwrap();
    let error = ProjectInput::load(root.path(), "missing-ruby").unwrap_err();
    assert!(error.to_string().contains("Custom lockfile paths"));
}

#[test]
fn selected_project_file_overrides_both_standard_formats() {
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("rbproject.toml"), "invalid TOML").unwrap();
    fs::write(root.path().join("rbproject.kdl"), "invalid KDL").unwrap();
    let selected = root.path().join("custom.toml");
    fs::write(&selected, "[dependencies]\nrake='>= 0'\n").unwrap();
    let input =
        ProjectInput::load_with_project_file(root.path(), "missing-ruby", Some(&selected)).unwrap();
    assert!(input.manifest.dependencies.contains_key("rake"));
    assert!(input.gemfile.is_none());
    assert_eq!(input.root, dunce::canonicalize(root.path()).unwrap());
}

#[test]
fn selected_missing_project_file_is_an_error() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("rbproject.toml"),
        "[dependencies]\nrake='*'\n",
    )
    .unwrap();
    assert!(
        ProjectInput::load_with_project_file(
            root.path(),
            "missing-ruby",
            Some(&root.path().join("missing.toml"))
        )
        .is_err()
    );
}

#[test]
fn project_scripts_are_not_dependency_declarations() {
    let root = tempfile::tempdir().unwrap();
    let project = root.path().join("settings.toml");
    fs::write(
        &project,
        "[project]\nname='sample'\n[scripts]\nprobe='ruby -v'\n[dependencies]\nrake='>= 0'\n",
    )
    .unwrap();
    let input =
        ProjectInput::load_with_project_file(root.path(), "missing-ruby", Some(&project)).unwrap();
    assert_eq!(input.manifest.dependencies.len(), 1);
    assert!(input.manifest.dependencies.contains_key("rake"));
}

#[test]
fn kdl_project_metadata_and_scripts_are_not_dependency_declarations() {
    let root = tempfile::tempdir().unwrap();
    let project = root.path().join("settings.kdl");
    fs::write(
        &project,
        "project { name \"sample\"; }\nscripts { probe \"ruby -v\"; }\ngem \"rake\" \">= 0\"\n",
    )
    .unwrap();
    let input =
        ProjectInput::load_with_project_file(root.path(), "missing-ruby", Some(&project)).unwrap();
    assert_eq!(input.manifest.dependencies.len(), 1);
    assert!(input.manifest.dependencies.contains_key("rake"));
}

#[test]
fn scripts_only_kdl_project_still_reads_gemfile() {
    let root = tempfile::tempdir().unwrap();
    fs::write(
        root.path().join("Gemfile"),
        "source 'https://rubygems.org'\ngem 'rake'\n",
    )
    .unwrap();
    fs::write(
        root.path().join("rbproject.kdl"),
        "project { name \"sample\"; }\nscripts { probe \"ruby -v\"; }\n",
    )
    .unwrap();
    let input = ProjectInput::load(root.path(), common::ruby()).unwrap();
    assert!(input.manifest.dependencies.contains_key("rake"));
    assert!(input.gemfile.is_some());
}
