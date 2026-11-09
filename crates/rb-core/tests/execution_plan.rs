use rb_core::{ButlerCommand, ButlerRuntime, process_context::ProcessContext};
use std::{
    collections::{BTreeMap, HashMap},
    ffi::{OsStr, OsString},
};

#[test]
fn plans_reuse_environment_and_package_changes_replace_it() {
    let directory = tempfile::tempdir().unwrap();
    let inputs = ProcessContext::new(
        directory.path().into(),
        BTreeMap::from([
            ("PATH".into(), OsString::from("baseline")),
            ("INVOCATION_VALUE".into(), "first".into()),
        ]),
    );
    let runtime = ButlerRuntime::empty(directory.path().into(), directory.path().into())
        .with_process_context(inputs);
    let executable = std::env::current_exe().unwrap();
    let command = ButlerCommand::new(executable.to_str().unwrap());
    let same_state = runtime.clone();
    let first = command.execution_plan(&runtime).unwrap();
    let cached = runtime.execution_environment();
    let second = command.execution_plan(&runtime).unwrap();
    assert!(std::ptr::eq(cached, runtime.execution_environment()));
    assert!(std::ptr::eq(cached, same_state.execution_environment()));
    assert_eq!(first.environment(), second.environment());
    let changed = runtime.clone().with_package_environment(
        HashMap::from([("INVOCATION_VALUE".into(), "second".into())]),
        directory.path().join("new-bin"),
    );
    assert_eq!(
        changed.execution_environment()[OsStr::new("INVOCATION_VALUE")],
        "second"
    );
    assert_eq!(first.environment()[OsStr::new("INVOCATION_VALUE")], "first");
    assert_ne!(
        changed.execution_environment()[OsStr::new("PATH")],
        first.environment()[OsStr::new("PATH")]
    );
}

#[test]
fn invocation_inputs_and_debug_output_are_isolated() {
    let first = ProcessContext::new(
        "/first".into(),
        BTreeMap::from([("TOKEN".into(), "secret-value".into())]),
    );
    let second = ProcessContext::new("/second".into(), BTreeMap::new());
    assert_eq!(first.var("TOKEN"), Some(OsStr::new("secret-value")));
    assert_eq!(second.var("TOKEN"), None);
    assert!(!format!("{first:?}").contains("secret-value"));
    assert_eq!(
        first.at("nested").directory(),
        std::path::Path::new("/first/nested")
    );
}

#[test]
fn runtime_composition_uses_captured_home() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("explicit-home");
    #[cfg(windows)]
    let key = "USERPROFILE";
    #[cfg(not(windows))]
    let key = "HOME";
    let inputs = ProcessContext::new(
        root.path().into(),
        BTreeMap::from([(key.into(), home.clone().into_os_string())]),
    );
    let ruby = rb_core::RubyRuntime::new(
        rb_core::ruby::RubyType::CRuby,
        semver::Version::new(3, 4, 10),
        root.path().join("ruby-3.4.10"),
    );
    let runtime = ButlerRuntime::compose_selected(
        inputs,
        "rubies".into(),
        None,
        None,
        vec![ruby.clone()],
        ruby,
        None,
    );
    assert_eq!(runtime.gem_home().unwrap(), home.join(".gem/ruby/3.4.10"));
    assert_eq!(runtime.rubies_dir(), &root.path().join("rubies"));
}

#[test]
fn composition_retains_the_detected_custom_gem_paths() {
    let root = tempfile::tempdir().unwrap();
    let base = root.path().join("custom-gems");
    let ruby = rb_core::RubyRuntime::new(
        rb_core::ruby::RubyType::CRuby,
        semver::Version::new(3, 4, 10),
        root.path().join("ruby-3.4.10"),
    );
    let runtime = ButlerRuntime::compose_selected(
        ProcessContext::new(root.path().into(), BTreeMap::new()),
        root.path().into(),
        None,
        Some(base.clone()),
        vec![ruby.clone()],
        ruby,
        None,
    );
    let home = base.join("ruby").join("3.4.10");
    assert_eq!(runtime.gem_home(), Some(home.clone()));
    assert!(runtime.bin_dirs().contains(&home.join("bin")));
    assert_eq!(
        runtime.execution_environment()[OsStr::new("GEM_HOME")],
        home.as_os_str()
    );
}

#[cfg(windows)]
#[test]
fn windows_resolution_uses_captured_case_insensitive_environment() {
    let root = tempfile::tempdir().unwrap();
    let executable = root.path().join("local-tool.custom");
    std::fs::write(&executable, "fixture").unwrap();
    let inputs = ProcessContext::new(
        root.path().into(),
        BTreeMap::from([
            ("Path".into(), root.path().as_os_str().to_owned()),
            ("PathExt".into(), ".CUSTOM".into()),
        ]),
    );
    let runtime = ButlerRuntime::empty_in(root.path().into(), inputs);
    let plan = ButlerCommand::new("local-tool")
        .execution_plan(&runtime)
        .unwrap();
    assert_eq!(
        plan.executable().canonicalize().unwrap(),
        executable.canonicalize().unwrap()
    );
    let isolated = ButlerRuntime::empty_in(
        root.path().into(),
        ProcessContext::new(
            root.path().into(),
            BTreeMap::from([
                ("PATH".into(), root.path().as_os_str().to_owned()),
                ("PATHEXT".into(), ".EXE".into()),
            ]),
        ),
    );
    assert!(
        ButlerCommand::new("local-tool")
            .execution_plan(&isolated)
            .is_err()
    );
}

#[cfg(unix)]
#[test]
fn plans_resolve_relative_to_child_directory_and_preserve_non_unicode_inputs() {
    use std::{
        fs,
        os::unix::{ffi::OsStringExt, fs::PermissionsExt},
    };
    let directory = tempfile::tempdir().unwrap();
    let executable = directory.path().join("local-tool");
    fs::write(
        &executable,
        "#!/bin/sh\nprintf '%s\\n' \"$INVOCATION_VALUE\" \"$PWD\"\n",
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o755)).unwrap();
    let non_unicode = OsString::from_vec(vec![b'x', 0xff]);
    let inputs = ProcessContext::new(
        directory.path().into(),
        BTreeMap::from([
            ("PATH".into(), OsString::from(".")),
            ("INVOCATION_VALUE".into(), "captured".into()),
            ("OS_VALUE".into(), non_unicode.clone()),
        ]),
    );
    let runtime = ButlerRuntime::empty(directory.path().into(), directory.path().into())
        .with_process_context(inputs);
    let plan = ButlerCommand::new("local-tool")
        .execution_plan(&runtime)
        .unwrap();
    assert_eq!(plan.executable(), executable);
    assert_eq!(plan.environment()[OsStr::new("OS_VALUE")], non_unicode);
    let gem_home = directory.path().join(&non_unicode);
    let packaged = runtime.clone().with_package_environment_os(
        BTreeMap::from([("GEM_HOME".into(), gem_home.clone().into_os_string())]),
        gem_home.join("bin"),
    );
    assert_eq!(
        packaged.execution_environment()[OsStr::new("GEM_HOME")],
        gem_home.as_os_str()
    );
    assert!(
        std::env::split_paths(&packaged.execution_environment()[OsStr::new("PATH")])
            .any(|path| path == gem_home.join("bin"))
    );
    let mut overridden = ButlerCommand::new("local-tool");
    overridden.env("PATH", "/not-present");
    assert!(overridden.execution_plan(&runtime).is_err());
    let output = plan.command().output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!(
            "captured\n{}\n",
            directory.path().canonicalize().unwrap().display()
        )
    );
    fs::remove_file(executable).unwrap();
    assert_eq!(
        plan.command().spawn().unwrap_err().kind(),
        std::io::ErrorKind::NotFound
    );
}
