use rb_gemfile::{Evaluator, SnapshotCache, write_project};
use std::{fs, path::Path};

fn evaluator() -> Evaluator {
    Evaluator::new(std::env::var_os("RB_GEMFILE_RUBY").unwrap_or_else(|| "ruby".into()))
}

fn write(root: &Path, file: &str, text: &str) {
    let path = root.join(file);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

#[test]
fn reuses_snapshot_and_invalidates_file_contents_and_directory_membership() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "Gemfile",
        "gem 'rack', File.read('version').strip\nDir['Gemfile.*'].each { |f| eval_gemfile f }",
    );
    write(root.path(), "version", "3.1.0");
    let cache = SnapshotCache::new(root.path().join(".rb/gemfile.json"));
    let evaluator = evaluator();
    let gemfile = root.path().join("Gemfile");
    assert!(!cache.evaluate(&evaluator, &gemfile).unwrap().reused);
    assert!(cache.evaluate(&evaluator, &gemfile).unwrap().reused);
    write(root.path(), "version", "3.2.0");
    let result = cache.evaluate(&evaluator, &gemfile).unwrap();
    assert!(!result.reused);
    assert_eq!(
        result.evaluation.manifest.dependencies["rack"].version,
        ["3.2.0"]
    );
    write(root.path(), "Gemfile.local", "gem 'rake'");
    let result = cache.evaluate(&evaluator, &gemfile).unwrap();
    assert!(!result.reused);
    assert!(result.evaluation.manifest.dependencies.contains_key("rake"));
    fs::remove_file(root.path().join("Gemfile.local")).unwrap();
    let result = cache.evaluate(&evaluator, &gemfile).unwrap();
    assert!(!result.evaluation.manifest.dependencies.contains_key("rake"));
    assert!(cache.evaluate(&evaluator, &gemfile).unwrap().reused);
}

#[test]
fn corruption_is_a_miss_and_failed_evaluation_preserves_existing_snapshot() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "Gemfile",
        "warn 'diagnostic'; puts 'hello'; gem 'rake'",
    );
    let path = root.path().join("snapshot.json");
    let cache = SnapshotCache::new(&path);
    let evaluator = evaluator();
    let gemfile = root.path().join("Gemfile");
    cache.evaluate(&evaluator, &gemfile).unwrap();
    let reused = cache.evaluate(&evaluator, &gemfile).unwrap();
    assert!(reused.reused);
    assert_eq!(
        reused.evaluation.stdout.lines().collect::<Vec<_>>(),
        ["hello"]
    );
    assert_eq!(
        reused.evaluation.stderr.lines().collect::<Vec<_>>(),
        ["diagnostic"]
    );
    fs::write(&path, "broken JSON").unwrap();
    assert!(!cache.evaluate(&evaluator, &gemfile).unwrap().reused);
    let original = fs::read(&path).unwrap();
    write(root.path(), "Gemfile", "raise 'evaluation failed'");
    assert!(cache.evaluate(&evaluator, &gemfile).is_err());
    assert_eq!(fs::read(&path).unwrap(), original);
    write(
        root.path(),
        "Gemfile",
        "warn 'diagnostic'; puts 'hello'; gem 'rake'",
    );
    assert!(cache.evaluate(&evaluator, &gemfile).unwrap().reused);
}

#[test]
fn external_includes_require_declared_cache_inputs() {
    let root = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    write(external.path(), "Gemfile", "gem 'rake'");
    let external_file = external.path().join("Gemfile");
    write(
        root.path(),
        "Gemfile",
        &format!(
            "eval_gemfile {}",
            serde_json::to_string(&external_file.to_str().unwrap()).unwrap()
        ),
    );
    let path = root.path().join(".rb/snapshot.json");
    let cache = SnapshotCache::new(&path);
    let evaluator = evaluator();
    let gemfile = root.path().join("Gemfile");
    assert!(!cache.evaluate(&evaluator, &gemfile).unwrap().reused);
    assert!(!path.exists());
    let cache = cache.input(external.path());
    assert!(!cache.evaluate(&evaluator, &gemfile).unwrap().reused);
    assert!(cache.evaluate(&evaluator, &gemfile).unwrap().reused);
    write(external.path(), "Gemfile", "gem 'rack'");
    let result = cache.evaluate(&evaluator, &gemfile).unwrap();
    assert!(!result.reused);
    assert!(result.evaluation.manifest.dependencies.contains_key("rack"));
}

#[test]
#[cfg(unix)]
fn external_includes_through_symlinked_directories_reuse_snapshot() {
    let root = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    write(external.path(), "Gemfile", "gem 'rake'");
    let alias = root.path().join("external");
    std::os::unix::fs::symlink(external.path(), &alias).unwrap();
    write(root.path(), "Gemfile", "eval_gemfile 'external/Gemfile'");
    let gemfile = root.path().join("Gemfile");
    let cache = SnapshotCache::new(root.path().join(".rb/snapshot.json")).input(external.path());
    let evaluator = evaluator();
    let result = cache.evaluate(&evaluator, &gemfile).unwrap();
    assert!(!result.reused);
    assert!(
        result
            .evaluation
            .inputs
            .contains(&dunce::canonicalize(external.path().join("Gemfile")).unwrap())
    );
    assert!(cache.evaluate(&evaluator, &gemfile).unwrap().reused);
    write(external.path(), "Gemfile", "gem 'rack'");
    let result = cache.evaluate(&evaluator, &gemfile).unwrap();
    assert!(!result.reused);
    assert!(result.evaluation.manifest.dependencies.contains_key("rack"));
}

#[test]
fn generated_project_is_replaced_and_keeps_project_configuration() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "Gemfile",
        "source 'https://rubygems.org'; gem 'rack'",
    );
    let evaluation = evaluator().evaluate(root.path().join("Gemfile")).unwrap();
    let project = evaluation
        .manifest
        .merge_project("[project]\nname = 'test'\n[dependencies]\nrake = '>= 13'")
        .unwrap();
    let path = root.path().join(".rb/rbproject.toml");
    write_project(&path, &project).unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), project);
    assert!(project.starts_with("# Generated by Ruby Butler. Do not edit.\n"));
    let parsed: toml::Value = toml::from_str(&project).unwrap();
    assert_eq!(parsed["project"]["name"].as_str(), Some("test"));
    assert!(parsed["sources"].as_array().unwrap().iter().any(|source| {
        source
            .get("dependencies")
            .is_some_and(|deps| deps.get("rack").is_some())
    }));
    write_project(&path, "replacement\n").unwrap();
    assert_eq!(fs::read_to_string(path).unwrap(), "replacement\n");
}

#[cfg(unix)]
#[test]
fn a_cache_hit_does_not_start_ruby() {
    use std::os::unix::fs::PermissionsExt;
    let root = tempfile::tempdir().unwrap();
    let executable = tempfile::tempdir().unwrap();
    let count = executable.path().join("starts");
    let wrapper = executable.path().join("ruby");
    let ruby =
        which::which(std::env::var_os("RB_GEMFILE_RUBY").unwrap_or_else(|| "ruby".into())).unwrap();
    let quote = |path: &Path| format!("'{}'", path.to_str().unwrap().replace('\'', "'\\''"));
    fs::write(
        &wrapper,
        format!(
            "#!/bin/sh\nprintf 'start\\n' >> {}\nexec {} \"$@\"\n",
            quote(&count),
            quote(&ruby)
        ),
    )
    .unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
    write(root.path(), "Gemfile", "gem 'rake'");
    let evaluator = Evaluator::new(&wrapper);
    let cache = SnapshotCache::new(root.path().join(".rb/snapshot.json"));
    let gemfile = root.path().join("Gemfile");
    assert!(!cache.evaluate(&evaluator, &gemfile).unwrap().reused);
    assert!(cache.evaluate(&evaluator, &gemfile).unwrap().reused);
    assert_eq!(fs::read_to_string(&count).unwrap(), "start\n");
    fs::write(
        &wrapper,
        format!(
            "#!/bin/sh\nprintf 'changed\\n' >> {}\nexec {} \"$@\"\n",
            quote(&count),
            quote(&ruby)
        ),
    )
    .unwrap();
    assert!(!cache.evaluate(&evaluator, &gemfile).unwrap().reused);
    assert_eq!(fs::read_to_string(count).unwrap(), "start\nchanged\n");
}

#[test]
fn effective_environment_changes_invalidate_without_storing_values() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "Gemfile",
        "gem 'rack', ENV.fetch('TEST_RACK_VERSION')",
    );
    let path = root.path().join(".rb/snapshot.json");
    let cache = SnapshotCache::new(&path);
    let gemfile = root.path().join("Gemfile");
    let first = evaluator()
        .env("TEST_RACK_VERSION", "3.1.0")
        .env("TEST_SECRET", "private-value");
    cache.evaluate(&first, &gemfile).unwrap();
    assert!(cache.evaluate(&first, &gemfile).unwrap().reused);
    assert!(!fs::read_to_string(&path).unwrap().contains("private-value"));
    let second = evaluator()
        .env("TEST_RACK_VERSION", "3.2.0")
        .env("TEST_SECRET", "private-value");
    let result = cache.evaluate(&second, &gemfile).unwrap();
    assert!(!result.reused);
    assert_eq!(
        result.evaluation.manifest.dependencies["rack"].version,
        ["3.2.0"]
    );
    assert!(cache.evaluate(&second, &gemfile).unwrap().reused);
}

#[test]
fn input_changes_during_evaluation_are_not_cached() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "Gemfile",
        "gem 'rack', File.read('version').strip; File.write('version', '3.2.0')",
    );
    write(root.path(), "version", "3.1.0");
    let path = root.path().join(".rb/snapshot.json");
    let result = SnapshotCache::new(&path)
        .evaluate(&evaluator(), root.path().join("Gemfile"))
        .unwrap();
    assert!(!result.reused);
    assert_eq!(
        result.evaluation.manifest.dependencies["rack"].version,
        ["3.1.0"]
    );
    assert!(!path.exists());
}

#[test]
fn cannot_overwrite_gemfile_or_ruby_inputs_with_a_snapshot() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "Gemfile", "gem 'rake'");
    let gemfile = root.path().join("Gemfile");
    let error = SnapshotCache::new(&gemfile)
        .evaluate(&evaluator(), &gemfile)
        .err()
        .unwrap();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    assert_eq!(fs::read_to_string(&gemfile).unwrap(), "gem 'rake'");
    let ruby =
        which::which(std::env::var_os("RB_GEMFILE_RUBY").unwrap_or_else(|| "ruby".into())).unwrap();
    let error = SnapshotCache::new(&ruby)
        .evaluate(&evaluator(), &gemfile)
        .unwrap_err();
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn valid_json_with_damaged_manifest_is_rebuilt() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "Gemfile", "gem 'rake'");
    let path = root.path().join(".rb/snapshot.json");
    let cache = SnapshotCache::new(&path);
    let evaluator = evaluator();
    let gemfile = root.path().join("Gemfile");
    cache.evaluate(&evaluator, &gemfile).unwrap();
    let mut snapshot: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    snapshot["evaluation"]["manifest"]["dependencies"]
        .as_object_mut()
        .unwrap()
        .remove("rake");
    fs::write(&path, serde_json::to_vec(&snapshot).unwrap()).unwrap();
    let result = cache.evaluate(&evaluator, &gemfile).unwrap();
    assert!(!result.reused);
    assert!(result.evaluation.manifest.dependencies.contains_key("rake"));
    assert!(cache.evaluate(&evaluator, &gemfile).unwrap().reused);
}

#[cfg(unix)]
#[test]
fn symlinked_inputs_are_fingerprinted_without_directory_cycles() {
    use std::os::unix::fs::symlink;
    let root = tempfile::tempdir().unwrap();
    let external = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "Gemfile",
        "gem 'rack', File.read('version').strip",
    );
    write(external.path(), "version", "3.1.0");
    symlink(external.path().join("version"), root.path().join("version")).unwrap();
    symlink(root.path(), root.path().join("loop")).unwrap();
    let cache = SnapshotCache::new(root.path().join(".rb/snapshot.json"));
    let evaluator = evaluator();
    let gemfile = root.path().join("Gemfile");
    cache.evaluate(&evaluator, &gemfile).unwrap();
    assert!(cache.evaluate(&evaluator, &gemfile).unwrap().reused);
    write(external.path(), "version", "3.2.0");
    let result = cache.evaluate(&evaluator, &gemfile).unwrap();
    assert!(!result.reused);
    assert_eq!(
        result.evaluation.manifest.dependencies["rack"].version,
        ["3.2.0"]
    );
}

#[test]
fn invalid_file_paths_return_errors() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "Gemfile", "gem 'rake'");
    let evaluator = evaluator();
    let cache = SnapshotCache::new(root.path().join("snapshot.json"));
    assert_eq!(
        cache.evaluate(&evaluator, root.path()).unwrap_err().kind(),
        std::io::ErrorKind::InvalidInput
    );
    let filesystem_root = root.path().ancestors().last().unwrap();
    assert_eq!(
        SnapshotCache::new(filesystem_root)
            .evaluate(&evaluator, root.path().join("Gemfile"))
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::InvalidInput
    );
}

#[test]
fn exporting_the_generated_project_does_not_invalidate_its_snapshot() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "Gemfile", "gem 'rake'");
    let project = root.path().join(".rb/rbproject.toml");
    let cache = SnapshotCache::new(root.path().join(".rb/snapshot.json")).output(&project);
    let evaluator = evaluator();
    let gemfile = root.path().join("Gemfile");
    let initial = cache.evaluate(&evaluator, &gemfile).unwrap();
    assert!(!initial.reused);
    write_project(&project, &initial.evaluation.manifest.to_toml().unwrap()).unwrap();
    assert!(cache.evaluate(&evaluator, &gemfile).unwrap().reused);
    write_project(
        &project,
        &initial
            .evaluation
            .manifest
            .merge_project("[project]\nname='updated'")
            .unwrap(),
    )
    .unwrap();
    assert!(cache.evaluate(&evaluator, &gemfile).unwrap().reused);
}

#[test]
fn concurrent_callers_share_one_atomic_snapshot() {
    let root = tempfile::tempdir().unwrap();
    write(root.path(), "Gemfile", "gem 'rake'");
    let cache = SnapshotCache::new(root.path().join(".rb/snapshot.json"));
    let evaluator = evaluator();
    let gemfile = root.path().join("Gemfile");
    let mut results = std::thread::scope(|scope| {
        let first = scope.spawn(|| cache.evaluate(&evaluator, &gemfile).unwrap());
        let second = scope.spawn(|| cache.evaluate(&evaluator, &gemfile).unwrap());
        [first.join().unwrap(), second.join().unwrap()]
    });
    assert_eq!(
        results[0].evaluation.manifest,
        results[1].evaluation.manifest
    );
    results.sort_by_key(|result| result.reused);
    assert!(!results[0].reused);
    assert!(results[1].reused);
    assert!(cache.evaluate(&evaluator, &gemfile).unwrap().reused);
}

#[test]
fn changing_condition_inputs_updates_exported_dependencies() {
    let root = tempfile::tempdir().unwrap();
    write(
        root.path(),
        "Gemfile",
        include_str!(
            "fixtures/snapshot/changing_condition_inputs_updates_exported_dependencies-1.rb"
        ),
    );
    let cache = SnapshotCache::new(root.path().join(".rb/snapshot.json"));
    let gemfile = root.path().join("Gemfile");
    for (setting, expected) in [("0", false), ("1", true), ("0", false)] {
        let evaluator = evaluator().env("WITH_DEBUG", setting);
        let fresh = cache.evaluate(&evaluator, &gemfile).unwrap();
        assert!(!fresh.reused);
        let reused = cache.evaluate(&evaluator, &gemfile).unwrap();
        assert!(reused.reused);
        for result in [fresh, reused] {
            let output = result.evaluation.manifest.to_toml().unwrap();
            let parsed: toml::Value = toml::from_str(&output).unwrap();
            assert_eq!(parsed["dependencies"].get("debug").is_some(), expected);
            assert!(parsed["dependencies"].get("rake").is_some());
            assert!(!output.contains("enabled ="));
            if expected {
                assert_eq!(
                    parsed["dependencies"]["debug"]["require"].as_bool(),
                    Some(false)
                );
            }
        }
    }
}
