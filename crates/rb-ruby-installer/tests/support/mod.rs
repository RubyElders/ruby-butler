use rb_ruby_installer::{Installer, Platform};
use rb_tests::{HttpServer as Server, RubyArchive};
use std::path::Path;

pub fn archive(link: Option<(&str, &str)>, version: &str) -> Vec<u8> {
    let mut archive = RubyArchive::new("3.4.10");
    archive
        .file(
            "bin/ruby",
            format!("#!/bin/sh\nprintf '%s' '{version}'\n"),
            0o755,
        )
        .unwrap();
    archive.file("lib/ruby/fixture.rb", [], 0o644).unwrap();
    if let Some((name, target)) = link {
        archive.symlink(name, target).unwrap();
    }
    archive.finish().unwrap()
}

pub fn configure(server: &Server, bytes: Vec<u8>, digest: Option<String>) {
    let size = bytes.len();
    server.route("/ruby.tar.gz", 200, bytes);
    server.route("/catalog",200,serde_json::to_vec(&serde_json::json!({"tag_name":"fixture", "assets":[
        {"name":"ruby-3.4.10.x86_64_linux.tar.gz","size":size,"digest":digest,"browser_download_url":format!("{}/ruby.tar.gz",server.url)},
        {"name":"SHA256SUMS","size":100,"browser_download_url":format!("{}/sums",server.url)}
    ]})).unwrap());
}
pub fn install(
    installer: &Installer,
    destination: &Path,
) -> anyhow::Result<rb_ruby_installer::Installation> {
    installer.install(
        &"3.4.10".parse().unwrap(),
        Platform::LinuxX86_64,
        destination,
        |_| {},
    )
}

pub fn installer(root: &Path, server: &Server) -> Installer {
    Installer::new(root.join("cache"))
        .unwrap()
        .with_catalog_url(format!("{}/catalog", server.url))
}
