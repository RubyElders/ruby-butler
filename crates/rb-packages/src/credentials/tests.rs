use super::*;

fn setup(code: &str) -> (tempfile::TempDir, BTreeMap<String, String>) {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("rb");
    fs::create_dir(&directory).unwrap();
    fs::write(directory.join("credential.rb"), code).unwrap();
    let ruby = std::env::var_os("RB_GEM_BUILD_RUBY").unwrap_or_else(|| "ruby".into());
    let text = toml::to_string(&serde_json::json!({
        "auth": { "https://gems.example/team": {
            "command": [ruby.to_str().unwrap(), "credential.rb", "token; literal argument"],
            "username": "account"
        }}
    }))
    .unwrap();
    fs::write(directory.join(AUTH_CONFIG_FILE), text).unwrap();
    let env = BTreeMap::from([
        (
            "XDG_CONFIG_HOME".into(),
            root.path().to_str().unwrap().into(),
        ),
        ("BUNDLE_GEMS__EXAMPLE".into(), "bundler:token".into()),
    ]);
    (root, env)
}

#[test]
fn command_credentials_override_bundler_only_for_the_configured_source() {
    let (root, env) = setup("print ARGV.fetch(0), \"\\r\\n\"\n");
    let credentials = Credentials::from_environment(root.path(), env).unwrap();
    assert_eq!(
        credentials
            .credentials("https://gems.example/team/")
            .unwrap()
            .as_deref(),
        Some("account:token; literal argument")
    );
    assert_eq!(
        credentials
            .credentials("https://gems.example/other/")
            .unwrap()
            .as_deref(),
        Some("bundler:token")
    );
    assert_eq!(
        credentials
            .credentials("https://unrelated.example/")
            .unwrap(),
        None
    );
}

#[test]
fn commands_run_only_for_matched_sources_and_fail_without_disclosing_output() {
    for (code, message) in [
        (
            "warn 'secret'; print 'secret'; exit 1",
            "Credential command failed",
        ),
        (
            "print ''",
            "Credential command must return one nonempty token line",
        ),
        (
            "print \"secret\\nsecond\\n\"",
            "Credential command must return one nonempty token line",
        ),
        (
            "STDOUT.binmode; print [255].pack('C')",
            "Credential command returned invalid UTF-8",
        ),
    ] {
        let (root, env) = setup(code);
        let credentials = Credentials::from_environment(root.path(), env).unwrap();
        assert_eq!(
            credentials
                .credentials("https://unrelated.example/")
                .unwrap(),
            None
        );
        assert_eq!(
            format!(
                "{:#}",
                credentials
                    .credentials("https://gems.example/team/")
                    .unwrap_err()
            ),
            message
        );
    }
}

#[test]
fn config_uses_rb_user_directory_and_rejects_invalid_entries() {
    let home = BTreeMap::from([("HOME".into(), "/home/example".into())]);
    if !cfg!(windows) {
        assert_eq!(
            config_path(&home),
            Some(PathBuf::from("/home/example/.config/rb/auth.toml"))
        );
    }
    assert!(config_path(&BTreeMap::new()).is_none());
    for text in [
        "secret = [",
        "[auth.\"https://gems.example/\"]\ncommand = []\nusername = 'user'",
        "[auth.\"https://gems.example/\"]\ncommand = ['missing']\nusername = 'bad:user'",
        "[auth.\"https://user:secret@gems.example/\"]\ncommand = ['missing']\nusername = 'user'",
    ] {
        let (root, env) = setup("");
        fs::write(root.path().join("rb/auth.toml"), text).unwrap();
        let error = Credentials::from_environment(root.path(), env)
            .err()
            .unwrap();
        assert!(!format!("{error:#}").contains("secret"));
    }
}

#[test]
fn commands_are_lazy_and_offline_registry_setup_needs_no_credentials() {
    let (root, env) = setup("File.write('ran', 'yes'); puts 'token'");
    let credentials = Credentials::from_environment(root.path(), env).unwrap();
    assert!(!root.path().join("rb/ran").exists());
    assert_eq!(
        credentials
            .credentials("https://gems.example/team/")
            .unwrap()
            .as_deref(),
        Some("account:token")
    );
    assert!(root.path().join("rb/ran").exists());
    let manifest: rb_gemfile::Manifest =
        toml::from_str("[bundle]\nsource = 'https://gems.example/'").unwrap();
    let mut index = crate::RegistryIndex::new(&manifest, root.path(), true).unwrap();
    fs::create_dir(root.path().join(".bundle")).unwrap();
    fs::write(root.path().join(".bundle/config"), "[invalid").unwrap();
    index.load_credentials(root.path(), true).unwrap();
}
