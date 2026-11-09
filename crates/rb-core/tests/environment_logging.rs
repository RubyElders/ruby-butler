use rb_core::{ButlerRuntime, process_context::ProcessContext};
use std::{collections::BTreeMap, ffi::OsStr, sync::Mutex};

struct Recorder(Mutex<Vec<String>>);

impl log::Log for Recorder {
    fn enabled(&self, metadata: &log::Metadata<'_>) -> bool {
        metadata.level() <= log::Level::Debug
    }

    fn log(&self, record: &log::Record<'_>) {
        if self.enabled(record.metadata()) {
            self.0.lock().unwrap().push(record.args().to_string());
        }
    }

    fn flush(&self) {}
}

static LOG: Recorder = Recorder(Mutex::new(Vec::new()));

#[test]
fn preserves_environment_messages_once_without_dumping_inherited_secrets() {
    log::set_logger(&LOG).unwrap();
    log::set_max_level(log::LevelFilter::Debug);
    rb_core::butler::DetectorComposer::version_detector_for_bundler();
    rb_core::butler::DetectorComposer::gem_path_detector_for_bundler();
    rb_core::butler::DetectorComposer::gem_path_detector_standard();
    let root = tempfile::tempdir().unwrap();
    let bin = root.path().join("prepared/bin");
    let home = root.path().join("prepared/gems");
    let inputs = ProcessContext::new(
        root.path().into(),
        BTreeMap::from([
            ("PATH".into(), "inherited-path-sentinel".into()),
            ("CREDENTIAL_SECRET".into(), "secret-sentinel".into()),
        ]),
    );
    let runtime = ButlerRuntime::empty_in(root.path().into(), inputs).with_package_environment_os(
        BTreeMap::from([
            ("GEM_HOME".into(), home.clone().into_os_string()),
            ("GEM_PATH".into(), home.clone().into_os_string()),
            (
                "BUNDLE_PATH".into(),
                root.path().join("prepared").into_os_string(),
            ),
        ]),
        bin.clone(),
    );
    let same_state = runtime.clone();
    assert_eq!(
        runtime.execution_environment()[OsStr::new("GEM_HOME")],
        home.as_os_str()
    );
    same_state.execution_environment();
    runtime.execution_environment();
    let messages = LOG.0.lock().unwrap();
    assert!(!messages.iter().any(|message| message.contains("0.0.0")));
    assert!(
        !messages
            .iter()
            .any(|message| message.contains("Total gem directories:"))
    );
    let addition = format!("Adding to PATH: {}", bin.display());
    assert_eq!(
        messages
            .iter()
            .filter(|message| *message == &addition)
            .count(),
        1
    );
    assert_eq!(
        messages
            .iter()
            .filter(|message| message.starts_with("Setting GEM_HOME:"))
            .count(),
        1
    );
    assert!(messages.iter().any(|message| message
        == &format!(
            "Setting BUNDLE_PATH: {}",
            root.path().join("prepared").display()
        )));
    for prefix in [
        "Composing environment variables",
        "Building PATH environment variable",
        "Appending existing PATH:",
        "Final PATH:",
        "Environment composition complete:",
    ] {
        assert_eq!(
            messages
                .iter()
                .filter(|message| message.starts_with(prefix))
                .count(),
            1,
            "{prefix}"
        );
    }
    assert!(
        messages
            .iter()
            .any(|message| message == "Appending existing PATH: inherited-path-sentinel")
    );
    assert!(
        !messages
            .iter()
            .any(|message| message.contains("secret-sentinel"))
    );
}
