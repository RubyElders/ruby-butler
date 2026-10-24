#![cfg(unix)]
mod common;
use common::*;
use rb_packages::{InstallOptions, MemoryIndex, PlanOptions, install, plan};
use rb_task::TaskEvents;
use std::{fs, os::unix::fs::PermissionsExt, path::Path, sync::Arc};

#[test]
fn installation_serializes_all_specifications_in_one_ruby_process_and_reuses_them() {
    let root = tempfile::tempdir().unwrap();
    let mut index = MemoryIndex::default();
    for name in ["teacup", "water"] {
        add_archive(
            &mut index,
            root.path(),
            name,
            &[],
            &[],
            &[("lib/fixture.rb", b"VALUE=42\n")],
        );
    }
    let mut runtime = runtime();
    let quote = |path: &Path| format!("'{}'", path.to_str().unwrap().replace('\'', "'\\''"));
    let calls = root.path().join("calls");
    let wrapper = root.path().join("ruby-wrapper");
    fs::write(
        &wrapper,
        format!(
            "#!/bin/sh\nprintf '%s\\n' \"$*\" >> {}\nexec {} \"$@\"\n",
            quote(&calls),
            quote(&runtime.executable)
        ),
    )
    .unwrap();
    fs::set_permissions(&wrapper, fs::Permissions::from_mode(0o755)).unwrap();
    runtime.executable = wrapper;
    let plan = plan(
        &manifest("https://rubygems.org", "teacup='>= 0'\nwater='>= 0'"),
        &runtime,
        None,
        &index,
        &PlanOptions::default(),
    )
    .unwrap();
    let index = Arc::new(index);
    let bundle = install(
        root.path(),
        &plan,
        &runtime,
        index.clone(),
        InstallOptions::new(&runtime.executable),
        TaskEvents::default(),
    )
    .unwrap();
    let log = fs::read_to_string(&calls).unwrap();
    assert_eq!(
        log.lines()
            .filter(|line| line.contains("Specification.write_all"))
            .count(),
        1,
        "{log}"
    );
    let code = "require 'rubygems'; %w[teacup water].each { |name| spec = Gem::Specification.find_by_name(name); raise name unless spec.version.to_s == '1.0.0' && spec.raw_require_paths == ['lib'] }; puts 'ready'";
    let output = run(&runtime, &bundle, root.path(), code);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(stdout(&output), "ready\n");
    fs::write(&calls, "").unwrap();
    let reused = install(
        root.path(),
        &plan,
        &runtime,
        index,
        InstallOptions::new(&runtime.executable),
        TaskEvents::default(),
    )
    .unwrap();
    assert_eq!(reused.reused, 2);
    assert!(
        !fs::read_to_string(calls)
            .unwrap()
            .contains("Specification.write_all")
    );
}
