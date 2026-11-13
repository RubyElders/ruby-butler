mod common;
use common::*;
use rb_packages::{
    InstallOptions, MemoryIndex, PackageIndex, PackagePlan, PlanOptions, RubyEnvironment, install,
    plan,
};
use rb_task::TaskEvents;
use std::{
    fs,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

struct CountingIndex {
    index: MemoryIndex,
    archives: AtomicUsize,
}
impl PackageIndex for CountingIndex {
    fn candidates(&self, name: &str) -> anyhow::Result<Vec<rb_packages::PackageRecord>> {
        self.index.candidates(name)
    }
    fn archive(&self, record: &rb_packages::PackageRecord) -> anyhow::Result<std::path::PathBuf> {
        self.archives.fetch_add(1, Ordering::Relaxed);
        self.index.archive(record)
    }
}

fn fixture(
    native: bool,
) -> (
    tempfile::TempDir,
    Arc<CountingIndex>,
    RubyEnvironment,
    PackagePlan,
    InstallOptions,
) {
    let root = tempfile::tempdir().unwrap();
    let mut index = MemoryIndex::default();
    let files: Vec<(&str, &[u8])> = if native {
        vec![
            (
                "ext/extconf.rb",
                include_bytes!("fixtures/shared-cache/extconf.rb"),
            ),
            (
                "ext/package_native.c",
                include_bytes!("fixtures/native/package_native.c"),
            ),
            (
                "lib/teacup.rb",
                b"require 'package_native'\nTEACUP=PackageNative.answer\n",
            ),
        ]
    } else {
        vec![
            ("lib/teacup.rb", b"TEACUP=42\n"),
            ("bin/teacup", b"require 'teacup'; puts TEACUP\n"),
        ]
    };
    add_archive(
        &mut index,
        root.path(),
        "teacup",
        &[],
        if native { &["ext/extconf.rb"] } else { &[] },
        &files,
    );
    let runtime = runtime();
    let plan = plan(
        &manifest("https://rubygems.org", "teacup='>= 0'"),
        &runtime,
        None,
        &index,
        &PlanOptions::default(),
    )
    .unwrap();
    let mut options = InstallOptions::new(&runtime.executable);
    options.cache = Some(root.path().join("cache"));
    options
        .build
        .environment
        .insert("CCACHE_DISABLE".into(), "1".into());
    options.build.environment.insert(
        "BUILD_COUNTER".into(),
        root.path().join("builds").into_os_string(),
    );
    (
        root,
        Arc::new(CountingIndex {
            index,
            archives: AtomicUsize::new(0),
        }),
        runtime,
        plan,
        options,
    )
}

fn check(runtime: &RubyEnvironment, bundle: &rb_packages::InstalledBundle, project: &Path) {
    let result = run(runtime, bundle, project, "require 'teacup'; puts TEACUP");
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(stdout(&result), "42\n");
}

#[test]
fn fresh_worktrees_reuse_installed_gems_without_archives_or_shared_writable_files() {
    let (root, index, runtime, plan, options) = fixture(false);
    let one = root.path().join("one");
    let two = root.path().join("two");
    fs::create_dir_all(&one).unwrap();
    fs::create_dir_all(&two).unwrap();
    let first = install(
        &one,
        &plan,
        &runtime,
        index.clone(),
        options.clone(),
        TaskEvents::default(),
    )
    .unwrap();
    assert_eq!(first.installed, 1);
    let second = install(
        &two,
        &plan,
        &runtime,
        index.clone(),
        options.clone(),
        TaskEvents::default(),
    )
    .unwrap();
    assert_eq!((second.installed, second.reused), (0, 1));
    assert_eq!(index.archives.load(Ordering::Relaxed), 1);
    check(&runtime, &second, &two);
    let script = second.gem_home.join("bin/teacup");
    let result = run(&runtime, &second, &two, &format!("load {:?}", script));
    assert!(result.status.success());
    assert_eq!(stdout(&result), "42\n");
    fs::write(
        first.gem_home.join("gems/teacup-1.0.0/lib/teacup.rb"),
        "TEACUP=0",
    )
    .unwrap();
    check(&runtime, &second, &two);
    fs::remove_dir_all(second.bundle_path).unwrap();
    let third = install(
        &two,
        &plan,
        &runtime,
        index.clone(),
        options,
        TaskEvents::default(),
    )
    .unwrap();
    check(&runtime, &third, &two);
    assert_eq!(index.archives.load(Ordering::Relaxed), 1);
}

#[cfg(unix)]
#[test]
fn fresh_worktrees_reuse_compilation_and_build_flags_invalidate_it() {
    let (root, index, runtime, plan, mut options) = fixture(true);
    for (name, compiled) in [("one", true), ("two", false), ("three", true)] {
        let project = root.path().join(name);
        fs::create_dir_all(&project).unwrap();
        if name == "three" {
            options
                .build
                .arguments
                .push("--enable-cache-fixture".into());
        }
        let bundle = install(
            &project,
            &plan,
            &runtime,
            index.clone(),
            options.clone(),
            TaskEvents::default(),
        )
        .unwrap();
        assert_eq!(bundle.installed, usize::from(compiled));
        check(&runtime, &bundle, &project);
        if name == "one" {
            fs::remove_dir_all(&bundle.bundle_path).unwrap();
        }
    }
    assert_eq!(
        fs::read_to_string(root.path().join("builds"))
            .unwrap()
            .lines()
            .count(),
        2
    );
    assert_eq!(index.archives.load(Ordering::Relaxed), 2);
}

#[cfg(unix)]
#[test]
fn concurrent_worktrees_compile_once() {
    let (root, index, runtime, plan, options) = fixture(true);
    let results = std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for name in ["one", "two"] {
            let project = root.path().join(name);
            fs::create_dir_all(&project).unwrap();
            let index = index.clone();
            let runtime = runtime.clone();
            let plan = plan.clone();
            let options = options.clone();
            handles.push(scope.spawn(move || {
                let bundle = install(
                    &project,
                    &plan,
                    &runtime,
                    index,
                    options,
                    TaskEvents::default(),
                )
                .unwrap();
                check(&runtime, &bundle, &project);
                bundle.installed
            }));
        }
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .sum::<usize>()
    });
    assert_eq!(results, 1);
    assert_eq!(
        fs::read_to_string(root.path().join("builds")).unwrap(),
        "build\n"
    );
}

#[test]
fn completed_cache_reuse_does_not_verify_installed_file_contents() {
    let (root, index, runtime, plan, options) = fixture(false);
    let one = root.path().join("one");
    let two = root.path().join("two");
    fs::create_dir_all(&one).unwrap();
    fs::create_dir_all(&two).unwrap();
    install(
        &one,
        &plan,
        &runtime,
        index.clone(),
        options.clone(),
        TaskEvents::default(),
    )
    .unwrap();
    let cached = fs::read_dir(options.cache.as_ref().unwrap().join("installations-v1"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| path.is_dir())
        .unwrap();
    fs::write(
        cached.join("home/gems/teacup-1.0.0/lib/teacup.rb"),
        "TEACUP=43",
    )
    .unwrap();
    let bundle = install(
        &two,
        &plan,
        &runtime,
        index.clone(),
        options,
        TaskEvents::default(),
    )
    .unwrap();
    assert_eq!((bundle.installed, bundle.reused), (0, 1));
    assert_eq!(index.archives.load(Ordering::Relaxed), 1);
    let result = run(&runtime, &bundle, &two, "require 'teacup'; puts TEACUP");
    assert!(result.status.success());
    assert_eq!(stdout(&result), "43\n");
}

#[test]
fn changed_source_and_incomplete_cache_are_not_reused() {
    let (root, index, runtime, mut plan, options) = fixture(false);
    for name in ["one", "two", "three"] {
        let project = root.path().join(name);
        fs::create_dir_all(&project).unwrap();
        if name == "two" {
            let cached = fs::read_dir(options.cache.as_ref().unwrap().join("installations-v1"))
                .unwrap()
                .map(|e| e.unwrap().path())
                .find(|p| p.is_dir())
                .unwrap();
            fs::remove_file(cached.join("complete.json")).unwrap();
        }
        if name == "three" {
            plan.packages[0].record.source = "https://other.example/".into();
        }
        let bundle = install(
            &project,
            &plan,
            &runtime,
            index.clone(),
            options.clone(),
            TaskEvents::default(),
        )
        .unwrap();
        assert_eq!(bundle.installed, 1);
        check(&runtime, &bundle, &project);
    }
    assert_eq!(index.archives.load(Ordering::Relaxed), 3);
}

#[test]
fn missing_cached_installation_metadata_is_rebuilt() {
    for missing in [
        "home/specifications/teacup-1.0.0.gemspec",
        "home/gems/teacup-1.0.0/installation.json",
    ] {
        let (root, index, runtime, plan, options) = fixture(false);
        for name in ["one", "two"] {
            let project = root.path().join(name);
            fs::create_dir_all(&project).unwrap();
            if name == "two" {
                let entry = fs::read_dir(options.cache.as_ref().unwrap().join("installations-v1"))
                    .unwrap()
                    .map(|entry| entry.unwrap().path())
                    .find(|path| path.is_dir())
                    .unwrap();
                fs::remove_file(entry.join(missing)).unwrap();
            }
            let bundle = install(
                &project,
                &plan,
                &runtime,
                index.clone(),
                options.clone(),
                TaskEvents::default(),
            )
            .unwrap();
            assert_eq!(bundle.installed, 1);
            check(&runtime, &bundle, &project);
        }
        assert_eq!(index.archives.load(Ordering::Relaxed), 2);
    }
}

#[cfg(unix)]
#[test]
fn failed_compilation_is_never_a_completed_cache_entry() {
    let (root, index, runtime, plan, mut options) = fixture(true);
    options.build.arguments.push("--fail-cache-build".into());
    for name in ["one", "two"] {
        let project = root.path().join(name);
        fs::create_dir_all(&project).unwrap();
        assert!(
            install(
                &project,
                &plan,
                &runtime,
                index.clone(),
                options.clone(),
                TaskEvents::default()
            )
            .is_err()
        );
        assert!(!project.join(".rb/bundle/ruby").join(&runtime.abi).exists());
        for entry in fs::read_dir(options.cache.as_ref().unwrap().join("installations-v1")).unwrap()
        {
            assert!(!entry.unwrap().path().join("complete.json").is_file());
        }
    }
    assert_eq!(
        fs::read_to_string(root.path().join("builds"))
            .unwrap()
            .lines()
            .count(),
        2
    );
}

#[test]
fn preparation_uses_global_cache_in_an_offline_second_worktree() {
    let (root, index, runtime, _, _) = fixture(false);
    let record = &index.index.packages["teacup"][0];
    let archive = fs::read(index.index.archives.values().next().unwrap()).unwrap();
    let server = common::server::Server::new(std::collections::BTreeMap::from([
        (
            "/info/teacup".into(),
            format!("---\n1.0.0 |checksum:{}\n", record.checksum).into_bytes(),
        ),
        ("/gems/teacup-1.0.0.gem".into(), archive),
    ]));
    let cache = root.path().join("global");
    let one = root.path().join("one");
    let two = root.path().join("two");
    for project in [&one, &two] {
        fs::create_dir_all(project).unwrap();
        fs::write(
            project.join("Gemfile"),
            format!("source '{}'\ngem 'teacup'\n", server.url),
        )
        .unwrap();
    }
    let first = rb_packages::prepare(
        rb_packages::PrepareOptions::new(&one, &runtime.executable, &cache),
        TaskEvents::default(),
    )
    .unwrap();
    assert_eq!(first.installed.installed, 1);
    fs::copy(one.join("Gemfile.lock"), two.join("Gemfile.lock")).unwrap();
    let requests = server.requests.lock().unwrap().len();
    drop(server);
    let mut options = rb_packages::PrepareOptions::new(&two, &runtime.executable, &cache);
    options.offline = true;
    options.plan.mode = rb_packages::LockMode::Locked;
    let second = rb_packages::prepare(options, TaskEvents::default()).unwrap();
    assert!(second.changed);
    assert_eq!(
        (second.installed.installed, second.installed.reused),
        (0, 1)
    );
    assert_eq!(requests, 3);
    check(&runtime, &second.installed, &two);
    assert_eq!(
        fs::read(one.join("Gemfile.lock")).unwrap(),
        fs::read(two.join("Gemfile.lock")).unwrap()
    );
}

#[test]
fn equivalent_path_sources_share_installations_and_payload_edits_invalidate_them() {
    let root = tempfile::tempdir().unwrap();
    let runtime = runtime();
    let cache = root.path().join("cache");
    for (name, value, installed) in [("one", 42, 1), ("two", 42, 0), ("three", 43, 1)] {
        let project = root.path().join(name);
        fs::create_dir_all(project.join("local/lib")).unwrap();
        fs::write(project.join("local/teacup.gemspec"), "Gem::Specification.new { |s| s.name='teacup'; s.version='1.0.0'; s.summary='Fixture'; s.authors=['Fixture']; s.files=['lib/teacup.rb'] }\n").unwrap();
        fs::write(
            project.join("local/lib/teacup.rb"),
            format!("TEACUP={value}\n"),
        )
        .unwrap();
        fs::write(project.join("Gemfile"), "gem 'teacup', path: 'local'\n").unwrap();
        let options = rb_packages::PrepareOptions::new(&project, &runtime.executable, &cache);
        let prepared = rb_packages::prepare(options, TaskEvents::default()).unwrap();
        assert_eq!(prepared.installed.installed, installed);
        for entry in fs::read_dir(cache.join("installations-v1")).unwrap() {
            let receipt = entry
                .unwrap()
                .path()
                .join("home/gems/teacup-1.0.0/installation.json");
            if receipt.is_file() {
                let text = fs::read_to_string(receipt).unwrap();
                assert!(!text.contains(project.to_str().unwrap()), "{text}");
            }
        }

        let output = run(
            &runtime,
            &prepared.installed,
            &project,
            "require 'teacup'; puts TEACUP",
        );
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(stdout(&output), format!("{value}\n"));
    }
}

#[cfg(unix)]
#[test]
fn extensions_build_against_stable_cached_dependencies_and_repair_after_cache_removal() {
    let root = tempfile::tempdir().unwrap();
    let mut index = MemoryIndex::default();
    add_archive(
        &mut index,
        root.path(),
        "leaf",
        &[],
        &[],
        &[("lib/leaf.rb", b"LEAF=42\n")],
    );
    add_archive(
        &mut index,
        root.path(),
        "other",
        &[],
        &[],
        &[("lib/other.rb", b"OTHER=42\n")],
    );
    add_archive(
        &mut index,
        root.path(),
        "teacup",
        &[("leaf", ">= 0"), ("other", ">= 0")],
        &["ext/extconf.rb"],
        &[
            (
                "ext/extconf.rb",
                include_bytes!("fixtures/shared-cache/extconf-with-dependency.rb"),
            ),
            (
                "ext/package_native.c",
                include_bytes!("fixtures/native/package_native.c"),
            ),
            (
                "lib/teacup.rb",
                b"require 'package_native'\nTEACUP=PackageNative.answer\n",
            ),
        ],
    );
    let runtime = runtime();
    let plan = plan(
        &manifest("https://rubygems.org", "teacup='>= 0'"),
        &runtime,
        None,
        &index,
        &PlanOptions::default(),
    )
    .unwrap();
    let mut options = InstallOptions::new(&runtime.executable);
    options.cache = Some(root.path().join("cache"));
    options.build.environment.insert(
        "BUILD_COUNTER".into(),
        root.path().join("builds").into_os_string(),
    );
    options
        .build
        .environment
        .insert("CCACHE_DISABLE".into(), "1".into());
    let index = Arc::new(index);
    let one = root.path().join("one");
    let two = root.path().join("two");
    for (project, expected) in [(&one, 3), (&two, 0)] {
        fs::create_dir_all(project).unwrap();
        let bundle = install(
            project,
            &plan,
            &runtime,
            index.clone(),
            options.clone(),
            TaskEvents::default(),
        )
        .unwrap();
        assert_eq!(bundle.installed, expected);
        check(&runtime, &bundle, project);
        let path = fs::read_to_string(
            bundle
                .gem_home
                .join("gems/teacup-1.0.0/ext/dependency-location"),
        )
        .unwrap();
        assert_eq!(path.lines().count(), 2);
        assert!(
            path.lines()
                .all(|path| Path::new(path).starts_with(options.cache.as_ref().unwrap()))
        );
        if project == &one {
            fs::remove_dir_all(bundle.bundle_path).unwrap();
        }
    }
    assert_eq!(
        fs::read_to_string(root.path().join("builds")).unwrap(),
        "build\n"
    );
    let leaf_entry = fs::read_dir(options.cache.as_ref().unwrap().join("installations-v1"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            fs::read(path.join("complete.json"))
                .ok()
                .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
                .is_some_and(|value| value["metadata"]["id"]["name"] == "leaf")
        })
        .unwrap();
    fs::remove_dir_all(leaf_entry).unwrap();
    let repaired = install(&two, &plan, &runtime, index, options, TaskEvents::default()).unwrap();
    assert_eq!(repaired.installed, 1);
    check(&runtime, &repaired, &two);
    let locations = fs::read_to_string(
        repaired
            .gem_home
            .join("gems/teacup-1.0.0/ext/dependency-location"),
    )
    .unwrap();
    assert_eq!(locations.lines().count(), 2);
    assert!(
        locations
            .lines()
            .all(|path| Path::new(path).starts_with(&two))
    );
    assert_eq!(
        fs::read_to_string(root.path().join("builds")).unwrap(),
        "build\nbuild\n"
    );
}

#[test]
fn parallel_restores_publish_colliding_executables_in_stable_order() {
    let root = tempfile::tempdir().unwrap();
    let runtime = runtime();
    let mut index = MemoryIndex::default();
    for name in ["alpha", "zulu"] {
        add_archive(
            &mut index,
            root.path(),
            name,
            &[],
            &[],
            &[("bin/shared", b"puts 'fixture'\n")],
        );
    }
    let plan = plan(
        &manifest("https://rubygems.org", "alpha='>= 0'\nzulu='>= 0'"),
        &runtime,
        None,
        &index,
        &PlanOptions::default(),
    )
    .unwrap();
    let index = Arc::new(index);
    let mut options = InstallOptions::new(&runtime.executable);
    options.cache = Some(root.path().join("cache"));
    for (name, workers) in [("one", 1), ("two", 4)] {
        options.workers = workers;
        let project = root.path().join(name);
        fs::create_dir_all(&project).unwrap();
        let bundle = install(
            &project,
            &plan,
            &runtime,
            index.clone(),
            options.clone(),
            TaskEvents::default(),
        )
        .unwrap();
        let wrapper = fs::read_to_string(bundle.gem_home.join("bin/shared")).unwrap();
        assert!(
            wrapper.contains("Gem.activate_bin_path(\"zulu\""),
            "{wrapper}"
        );
        if name == "two" {
            assert_eq!(bundle.installed, 0);
        }
    }
}
