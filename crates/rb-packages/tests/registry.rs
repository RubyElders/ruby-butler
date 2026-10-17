mod common;
use common::server::Server;
use rb_task::{TaskEvent, TaskEventSink, TaskEvents};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

#[derive(Default)]
struct Events(Mutex<Vec<TaskEvent>>);
impl TaskEventSink for Events {
    fn event(&self, event: TaskEvent) {
        self.0.lock().unwrap().push(event);
    }
}

#[test]
fn prepares_from_gemfile_then_reuses_offline_without_contacting_registry() {
    let project = tempfile::tempdir().unwrap();
    let archives = tempfile::tempdir().unwrap();
    let mut index = rb_packages::MemoryIndex::default();
    common::add_archive(
        &mut index,
        archives.path(),
        "teacup",
        &[],
        &[],
        &[("lib/teacup.rb", b"TEACUP=42\n")],
    );
    let record = &index.packages["teacup"][0];
    let server = Server::new(BTreeMap::from([
        (
            "/info/teacup".into(),
            format!("---\n1.0.0 |checksum:{}\n", record.checksum).into_bytes(),
        ),
        (
            "/gems/teacup-1.0.0.gem".into(),
            fs::read(index.archives.values().next().unwrap()).unwrap(),
        ),
    ]));
    fs::write(
        project.path().join("Gemfile"),
        format!("source '{}'\ngem 'teacup'\n", server.url),
    )
    .unwrap();
    let events = TaskEvents::default();
    let collected = Arc::new(Events::default());
    events.subscribe(collected.clone());
    let mut options = rb_packages::PrepareOptions::new(
        project.path(),
        common::ruby(),
        project.path().join(".rb/cache"),
    );
    let result = rb_packages::prepare(options.clone(), events.clone()).unwrap();
    assert_eq!(result.installed.installed, 1);
    assert!(result.input.bundler_gemfile.is_file());
    let count = server.requests.lock().unwrap().len();
    assert_eq!(count, 2);
    options.offline = true;
    let reused = rb_packages::prepare(options, events).unwrap();
    assert_eq!(reused.installed.reused, 1);
    assert!(!reused.plan.lock_changed);
    assert_eq!(server.requests.lock().unwrap().len(), count);
    let labels: Vec<_> = collected
        .0
        .lock()
        .unwrap()
        .iter()
        .filter_map(|event| {
            if let TaskEvent::Started { label, worker, .. } = event {
                assert!(*worker < 4);
                Some(label.clone())
            } else {
                None
            }
        })
        .collect();
    for label in [
        "lock project",
        "inspect Ruby",
        "load dependencies",
        "resolve dependencies",
        "prepare installation",
        "download teacup-1.0.0",
        "extract teacup-1.0.0",
        "install teacup-1.0.0",
        "publish bundle",
        "write lockfile",
    ] {
        assert!(
            labels.iter().any(|value| value == label),
            "Missing worker task {label}"
        );
    }
}

#[test]
fn explicit_registry_blocks_are_usable_by_frozen_bundler() {
    let project = tempfile::tempdir().unwrap();
    let archives = tempfile::tempdir().unwrap();
    let mut index = rb_packages::MemoryIndex::default();
    common::add_archive(
        &mut index,
        archives.path(),
        "teacup",
        &[],
        &[],
        &[("lib/teacup.rb", b"TEACUP=42\n")],
    );
    common::add_archive(
        &mut index,
        archives.path(),
        "saucer",
        &[],
        &[],
        &[("lib/saucer.rb", b"SAUCER=1\n")],
    );
    let make_server = |name: &str| {
        let record = &index.packages[name][0];
        Server::new(BTreeMap::from([
            (
                format!("/info/{name}"),
                format!("---\n1.0.0 |checksum:{}\n", record.checksum).into_bytes(),
            ),
            (
                format!("/gems/{name}-1.0.0.gem"),
                fs::read(&index.archives[&record.checksum]).unwrap(),
            ),
        ]))
    };
    let one = make_server("teacup");
    let two = make_server("saucer");
    let mut manifest = common::manifest(&one.url, "teacup='>= 0'\nsaucer='>= 0'");
    manifest.bundle.source = None;
    manifest.dependencies.get_mut("teacup").unwrap().source = Some(one.url.clone());
    manifest.dependencies.get_mut("saucer").unwrap().source = Some(two.url.clone());
    fs::write(
        project.path().join("rbproject.toml"),
        manifest.to_toml().unwrap(),
    )
    .unwrap();
    let options = rb_packages::PrepareOptions::new(
        project.path(),
        common::ruby(),
        project.path().join(".rb/cache"),
    );
    let result = rb_packages::prepare(options, TaskEvents::default()).unwrap();
    let output = std::process::Command::new(common::ruby())
        .envs(result.installed.environment())
        .env_remove("RUBYOPT")
        .env("BUNDLE_FROZEN", "true")
        .env("BUNDLE_IGNORE_CONFIG", "true")
        .current_dir(project.path())
        .args([
            "-rbundler/setup",
            "-rteacup",
            "-rsaucer",
            "-e",
            "puts TEACUP + SAUCER",
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(common::stdout(&output), "43\n");
}

#[test]
fn registry_fixture_accepts_a_request_after_connection_establishment() {
    let server = Server::new(BTreeMap::from([("/probe".into(), b"ready".to_vec())]));
    let address = server
        .url
        .trim_start_matches("http://")
        .trim_end_matches('/');
    let mut socket = std::net::TcpStream::connect(address).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    thread::sleep(Duration::from_millis(100));
    socket
        .write_all(b"GET /probe HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    let mut response = String::new();
    socket.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(response.ends_with("\r\n\r\nready"));
    assert_eq!(*server.requests.lock().unwrap(), ["/probe"]);
}
