#![allow(dead_code)]
use flate2::{Compression, write::GzEncoder};
use rb_gem_types::{PackageId, Platform};
use rb_packages::{MemoryIndex, PackageRecord, RubyEnvironment};
use rb_resolver::Package;
use sha2::{Digest, Sha256};
use std::{
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

pub fn ruby() -> PathBuf {
    std::env::var_os("RB_GEM_BUILD_RUBY")
        .unwrap_or_else(|| "ruby".into())
        .into()
}
pub fn runtime() -> RubyEnvironment {
    RubyEnvironment::inspect(ruby()).unwrap()
}
pub fn manifest(source: &str, dependencies: &str) -> rb_gemfile::Manifest {
    toml::from_str(&format!(
        "[bundle]\nsource = {source:?}\n[dependencies]\n{dependencies}\n"
    ))
    .unwrap()
}
pub fn record(name: &str, version: &str, dependencies: &[(&str, &str)]) -> PackageRecord {
    PackageRecord {
        package: Package {
            id: PackageId {
                name: name.into(),
                version: version.parse().unwrap(),
                platform: Platform::parse("ruby"),
            },
            dependencies: dependencies
                .iter()
                .map(|(n, r)| (n.to_string(), r.parse().unwrap()))
                .collect(),
            ruby: Default::default(),
        },
        source: "https://rubygems.org/".into(),
        archive_version: version.into(),
        checksum: "a".repeat(64),
        rubygems: None,
    }
}
pub fn add(index: &mut MemoryIndex, record: PackageRecord) {
    index
        .packages
        .entry(record.package.id.name.clone())
        .or_default()
        .push(record);
}
pub fn add_archive(
    index: &mut MemoryIndex,
    directory: &Path,
    name: &str,
    dependencies: &[(&str, &str)],
    extensions: &[&str],
    files: &[(&str, &[u8])],
) {
    let metadata = serde_json::json!({ "name": name, "version": "1.0.0", "platform": "ruby", "dependencies": dependencies.iter().map(|(n,r)| serde_json::json!({"name":n,"requirement": {"requirements": [[">=", {"version":r.trim_start_matches(">= ")}]]}, "type":"runtime"})).collect::<Vec<_>>(), "require_paths": ["lib"], "extensions": extensions, "files": files.iter().map(|(p,_)| *p).collect::<Vec<_>>(), "executables": if files.iter().any(|(p,_)| p.starts_with("bin/")) { vec![name] } else { vec![] }, "bindir":"bin" });
    let payload = tar(files);
    let archive = tar(&[
        (
            "metadata.gz",
            &gzip(&serde_json::to_vec(&metadata).unwrap()),
        ),
        ("data.tar.gz", &gzip(&payload)),
    ]);
    let checksum = format!("{:x}", Sha256::digest(&archive));
    let path = directory.join(format!("{name}.gem"));
    std::fs::write(&path, archive).unwrap();
    let mut record = record(name, "1.0.0", dependencies);
    record.checksum = checksum.clone();
    index.archives.insert(checksum, path);
    add(index, record);
}
fn tar(files: &[(&str, &[u8])]) -> Vec<u8> {
    let mut archive = tar::Builder::new(Vec::new());
    for (name, data) in files {
        let mut header = tar::Header::new_gnu();
        header.set_mode(0o644);
        header.set_size(data.len() as u64);
        header.set_cksum();
        archive.append_data(&mut header, name, *data).unwrap();
    }
    archive.into_inner().unwrap()
}
fn gzip(data: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}
pub fn run(
    runtime: &RubyEnvironment,
    bundle: &rb_packages::InstalledBundle,
    project: &Path,
    code: &str,
) -> std::process::Output {
    Command::new(&runtime.executable)
        .envs(bundle.environment())
        .env_remove("RUBYOPT")
        .env_remove("RUBYLIB")
        .env_remove("BUNDLE_GEMFILE")
        .env("BUNDLE_IGNORE_CONFIG", "true")
        .current_dir(project)
        .args(["-e", code])
        .output()
        .unwrap()
}

pub fn stdout(output: &std::process::Output) -> String {
    String::from_utf8(output.stdout.clone())
        .unwrap()
        .replace("\r\n", "\n")
}
