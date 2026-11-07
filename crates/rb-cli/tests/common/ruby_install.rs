use rb_ruby_installer::CATALOG_URL;
use rb_tests::RubyArchive;
use sha2::{Digest, Sha256};
use std::{
    fs,
    process::{Command, Output},
    time::SystemTime,
};

pub struct Fixture {
    pub root: tempfile::TempDir,
}
impl Fixture {
    pub fn new() -> Self {
        let fixture = Self {
            root: tempfile::tempdir().unwrap(),
        };
        fs::write(fixture.root.path().join("config.toml"), "").unwrap();
        fixture
    }
    pub fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_rb"));
        command
            .current_dir(self.root.path())
            .args(["-c", "config.toml", "-R"])
            .arg(self.root.path().join("rubies"))
            .env("XDG_CACHE_HOME", self.root.path().join("cache"))
            .env_remove("RB_AUTO_INSTALL_RUBY")
            .env_remove("RB_RUBY_VERSION")
            .env_remove("RB_PROJECT")
            .env_remove("RB_WORK_DIR")
            .env_remove("BUNDLE_GEMFILE")
            .env_remove("RB_NO_BUNDLER")
            .env_remove("RB_PACKAGE_BACKEND");
        command
    }
    pub fn seed(&self) {
        let mut archive = RubyArchive::new("3.4.10");
        for (path, contents, mode) in [
            (
                "bin/ruby",
                include_str!("../fixtures/ruby_install/ruby.sh"),
                0o755,
            ),
            ("bin/bundle", "#!/bin/sh\nexit 0\n", 0o755),
            ("lib/ruby/fixture.rb", "", 0o644),
        ] {
            archive.file(path, contents, mode).unwrap();
        }
        let bytes = archive.finish().unwrap();
        let digest = format!("{:x}", Sha256::digest(&bytes));
        let cache = self.root.path().join("cache/rb/rubies");
        fs::create_dir_all(cache.join("archives")).unwrap();
        let suffix = match rb_ruby_installer::Platform::current().unwrap() {
            rb_ruby_installer::Platform::LinuxX86_64 => "x86_64_linux",
            rb_ruby_installer::Platform::LinuxAarch64 => "arm64_linux",
            rb_ruby_installer::Platform::LinuxMuslX86_64 => "x86_64_linux_musl",
            rb_ruby_installer::Platform::LinuxMuslAarch64 => "arm64_linux_musl",
            rb_ruby_installer::Platform::MacosX86_64 => "ventura",
            rb_ruby_installer::Platform::MacosAarch64 => "arm64_sonoma",
        };
        let catalog = serde_json::json!({"url":CATALOG_URL,"etag":null,"fetched":SystemTime::now(),"release":{
            "tag_name":"fixture","assets":[{"name":format!("ruby-3.4.10.{suffix}.tar.gz"),"size":bytes.len(),"digest":format!("sha256:{digest}"),"browser_download_url":"https://example.invalid/ruby.tar.gz"}]
        }});
        fs::write(
            cache.join("catalog.json"),
            serde_json::to_vec(&catalog).unwrap(),
        )
        .unwrap();
        fs::write(cache.join(format!("archives/{digest}.tar.gz")), bytes).unwrap();
    }
    pub fn installed(&self) -> bool {
        self.root
            .path()
            .join("rubies/ruby-3.4.10/bin/ruby")
            .is_file()
    }
}
pub fn success(output: Output) -> Output {
    assert!(output.status.success(), "{:?}", output);
    output
}
