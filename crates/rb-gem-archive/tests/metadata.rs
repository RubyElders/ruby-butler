mod common;
use rb_gem_archive::read;

#[test]
fn tagged_gemspec_retains_runtime_metadata_and_combines_duplicate_requirements() {
    let bytes = common::container(&common::members(include_bytes!("fixtures/metadata.yml")));
    let gem = read(bytes.as_slice()).unwrap();
    assert_eq!(gem.id.full_name(), "dinner-1.2.3");
    assert_eq!(gem.dependencies.len(), 1);
    assert_eq!(gem.dependencies["rack"].to_string(), "~> 3.0, >= 3.1");
    assert!(!gem.dependencies["rack"].matches(&"3.0".parse().unwrap()));
    assert!(gem.dependencies["rack"].matches(&"3.2".parse().unwrap()));
    assert_eq!(gem.ruby.to_string(), ">= 3.1");
    assert_eq!(gem.rubygems.to_string(), ">= 3.3");
    assert_eq!(gem.require_paths, ["lib"]);
    assert_eq!(gem.executables, ["dinner"]);
    assert_eq!(gem.bindir, "exe");
    assert_eq!(gem.extensions, ["ext/dinner/extconf.rb"]);
    assert_eq!(gem.files, ["lib/dinner.rb"]);
    assert_eq!(
        gem.metadata["source_code_uri"],
        "https://example.com/dinner"
    );
}

#[test]
fn native_platform_objects_are_read_without_ruby_execution() {
    let text = include_str!("fixtures/metadata.yml").replace(
        "platform: ruby",
        "platform: !ruby/object:Gem::Platform\n  cpu: x86_64\n  os: linux\n  version: musl",
    );
    let bytes = common::container(&common::members(text.as_bytes()));
    assert_eq!(
        read(bytes.as_slice()).unwrap().id.platform.to_string(),
        "x86_64-linux-musl"
    );
}

#[test]
fn absent_optional_metadata_uses_gemspec_defaults() {
    let bytes = common::container(&common::members(b"name: minimal\nversion: '1'\n"));
    let gem = read(bytes.as_slice()).unwrap();
    assert_eq!(gem.id.full_name(), "minimal-1");
    assert_eq!(gem.bindir, "bin");
    assert_eq!(gem.require_paths, ["lib"]);
    assert_eq!(gem.ruby.to_string(), ">= 0");
    assert!(gem.dependencies.is_empty());
    assert!(gem.extensions.is_empty());
}

#[test]
fn malformed_platform_components_cannot_silently_change_package_identity() {
    for key in ["cpu", "os", "version"] {
        let text = include_str!("fixtures/metadata.yml")
            .replace(
                "platform: ruby",
                "platform: {cpu: x86_64, os: linux, version: musl}\n",
            )
            .replace(
                &format!(
                    "{key}: {}",
                    match key {
                        "cpu" => "x86_64",
                        "os" => "linux",
                        _ => "musl",
                    }
                ),
                &format!("{key}: []"),
            );
        let bytes = common::container(&common::members(text.as_bytes()));
        assert!(read(bytes.as_slice()).is_err(), "{key}");
    }
}
