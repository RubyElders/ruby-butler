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
