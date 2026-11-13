mod common;
use common::{add, manifest, record, runtime, server::Server};
use rb_packages::{MemoryIndex, PlanOptions, RegistryIndex, plan};
use std::collections::BTreeMap;

#[test]
fn locked_sources_do_not_discover_dependencies_of_unselected_releases() {
    let checksum = "a".repeat(64);
    let public = Server::new(BTreeMap::from([(
        "/info/water".into(),
        format!("---\n1.0 |checksum:{checksum}\n").into_bytes(),
    )]));
    let private = Server::new(BTreeMap::from([("/info/tea".into(), format!("---\n1.0 water:>= 1|checksum:{checksum}\n99.0 historical-only:>= 1|checksum:{checksum}\n").into_bytes())]));
    let mut manifest = manifest(&public.url, "tea='>= 0'");
    manifest.dependencies.get_mut("tea").unwrap().source = Some(private.url.clone());
    let mut records = MemoryIndex::default();
    let mut tea = record("tea", "1.0", &[("water", ">= 1")]);
    tea.source = private.url.clone().into();
    let mut water = record("water", "1.0", &[]);
    water.source = public.url.clone().into();
    add(&mut records, tea);
    add(&mut records, water);
    let runtime = runtime();
    let first = plan(&manifest, &runtime, None, &records, &PlanOptions::default()).unwrap();
    let cache = tempfile::tempdir().unwrap();
    let index = RegistryIndex::new(&manifest, cache.path(), false).unwrap();
    let result = plan(
        &manifest,
        &runtime,
        Some(&first.lockfile),
        &index,
        &PlanOptions::default(),
    )
    .unwrap();
    assert_eq!(result.lockfile, first.lockfile);
    assert!(!result.lock_changed);
    assert_eq!(
        *private.requests.lock().unwrap(),
        ["/versions", "/info/tea"]
    );
    assert_eq!(
        *public.requests.lock().unwrap(),
        ["/versions", "/info/water"]
    );
}

#[test]
fn locked_registry_metadata_requests_overlap() {
    use std::{
        io::{Read, Write},
        net::TcpListener,
        sync::{Arc, Condvar, Mutex},
        thread,
        time::Duration,
    };
    let arrived = Arc::new((Mutex::new(0), Condvar::new()));
    let mut urls = Vec::new();
    let mut servers = Vec::new();
    for _ in 0..2 {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        urls.push(format!("http://{}/", listener.local_addr().unwrap()));
        let arrived = arrived.clone();
        servers.push(thread::spawn(move || {
            let mut socket = loop {
                let (mut socket, _) = listener.accept().unwrap();
                socket
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut request = Vec::new();
                let mut buffer = [0; 1024];
                while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                    let count = socket.read(&mut buffer).unwrap();
                    assert_ne!(count, 0);
                    request.extend_from_slice(&buffer[..count]);
                }
                if request.starts_with(b"GET /versions ") {
                    socket.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").unwrap();
                    continue;
                }
                assert!(request.starts_with(b"GET /info/"));
                break socket;
            };
            let (count, ready) = &*arrived;
            let mut count = count.lock().unwrap();
            *count += 1;
            ready.notify_all();
            let (_count, wait) = ready
                .wait_timeout_while(count, Duration::from_secs(2), |count| *count < 2)
                .unwrap();
            let overlapped = !wait.timed_out();
            let body = format!("---\n1.0 |checksum:{}\n", "a".repeat(64));
            write!(
                socket,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .unwrap();
            overlapped
        }));
    }
    let mut manifest = manifest(&urls[0], "tea='>= 0'\nwater='>= 0'");
    manifest.dependencies.get_mut("water").unwrap().source = Some(urls[1].clone());
    let mut records = MemoryIndex::default();
    for (name, url) in ["tea", "water"].into_iter().zip(&urls) {
        let mut package = record(name, "1.0", &[]);
        package.source = url.clone().into();
        add(&mut records, package);
    }
    let runtime = runtime();
    let first = plan(&manifest, &runtime, None, &records, &PlanOptions::default()).unwrap();
    let cache = tempfile::tempdir().unwrap();
    let index = RegistryIndex::new(&manifest, cache.path(), false).unwrap();
    let result = plan(
        &manifest,
        &runtime,
        Some(&first.lockfile),
        &index,
        &PlanOptions::default(),
    )
    .unwrap();
    assert_eq!(result.lockfile, first.lockfile);
    for server in servers {
        assert!(server.join().unwrap(), "Metadata was fetched serially");
    }
}

#[test]
fn locked_graph_reuses_source_metadata_across_projects_without_freezing_group_settings() {
    let server = Server::new(BTreeMap::from([(
        "/info/water".into(),
        format!("---\n1.0 |checksum:{}\n", "a".repeat(64)).into_bytes(),
    )]));
    let manifest = manifest(&server.url, "water={version='>= 0',groups=['test']}");
    let runtime = runtime();
    let mut records = MemoryIndex::default();
    let mut water = record("water", "1.0", &[]);
    water.source = server.url.clone().into();
    add(&mut records, water);
    let initial = plan(&manifest, &runtime, None, &records, &PlanOptions::default()).unwrap();
    let cache = tempfile::tempdir().unwrap();
    let index = RegistryIndex::new(&manifest, cache.path(), false).unwrap();
    plan(
        &manifest,
        &runtime,
        Some(&initial.lockfile),
        &index,
        &PlanOptions::default(),
    )
    .unwrap();
    let requests = server.requests.lock().unwrap().clone();
    drop(server);
    let index = RegistryIndex::new(&manifest, cache.path(), false).unwrap();
    let without = PlanOptions {
        without: ["test".into()].into(),
        ..Default::default()
    };
    let next = plan(
        &manifest,
        &runtime,
        Some(&initial.lockfile),
        &index,
        &without,
    )
    .unwrap();
    assert_eq!(next.lockfile, initial.lockfile);
    assert!(!next.packages[0].install);
    let normal = plan(
        &manifest,
        &runtime,
        Some(&initial.lockfile),
        &index,
        &PlanOptions::default(),
    )
    .unwrap();
    assert!(normal.packages[0].install);
    assert_eq!(requests, ["/versions", "/info/water"]);
    let source = cache.path().join("index");
    let directory = std::fs::read_dir(source)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let cached: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("water.json")).unwrap()).unwrap();
    assert_eq!(cached.as_object().unwrap().len(), 2);
    assert!(cached.get("body").is_some());
    assert!(cached.get("etag").is_some());
}

#[test]
fn locked_metadata_mismatch_fetches_replacement_and_explicit_updates_refresh_sources() {
    let server = Server::new(BTreeMap::from([(
        "/info/water".into(),
        format!("---\n1.0 |checksum:{}\n", "a".repeat(64)).into_bytes(),
    )]));
    let manifest = manifest(&server.url, "water='>= 0'");
    let runtime = runtime();
    let cache = tempfile::tempdir().unwrap();
    let index = RegistryIndex::new(&manifest, cache.path(), false).unwrap();
    let first = plan(&manifest, &runtime, None, &index, &PlanOptions::default()).unwrap();
    let before = server.requests.lock().unwrap().len();
    let index = RegistryIndex::new(&manifest, cache.path(), false).unwrap();
    let mut altered = first.lockfile.clone();
    let sums = altered.checksums.as_mut().unwrap();
    *sums.values_mut().next().unwrap() = Some("b".repeat(64));
    let locked = PlanOptions {
        mode: rb_packages::LockMode::Locked,
        ..Default::default()
    };
    assert!(plan(&manifest, &runtime, Some(&altered), &index, &locked).is_err());
    assert!(server.requests.lock().unwrap().len() > before);
    let before = server.requests.lock().unwrap().len();
    let index = RegistryIndex::new(&manifest, cache.path(), false).unwrap();
    let update = PlanOptions {
        mode: rb_packages::LockMode::Update,
        ..Default::default()
    };
    plan(&manifest, &runtime, Some(&first.lockfile), &index, &update).unwrap();
    assert!(server.requests.lock().unwrap().len() > before);
}
