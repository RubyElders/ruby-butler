use rb_lockfile::Lockfile;

#[test]
fn malformed_records_and_ambiguous_metadata_are_rejected() {
    let fixture = include_str!("fixtures/mixed.lock");
    let cases = [
        fixture.replace(
            "revision: 0123456789abcdef0123456789abcdef01234567",
            "revision: broken",
        ),
        fixture.replace("  specs:\n    widget", "    widget"),
        fixture.replace("  ffi\n", "  ffi\n  ffi\n"),
        fixture.replace("  ruby\n", "  ruby\n  ruby\n"),
        fixture.replace("    rack (3.1.0)", "    rack (3.1.0)\n    rack (3.1.0)"),
        fixture.replace("    rack (3.1.0)", "    rack (bad version)"),
        fixture.replace("    rack (3.1.0)", "    ../rack (3.1.0)"),
        fixture.replace("      rack (>= 2)", "      rack (?? 2)"),
        fixture.replace("      rack (>= 2)", "       rack (>= 2)"),
        fixture.replace("  submodules: true", "  submodules: perhaps"),
        fixture.replace("  branch: stable", "  branch: stable\n  branch: other"),
        fixture.replace("PLATFORMS", "PLUGIN SOURCE"),
        fixture.replace("  rack (3.1.0) sha256=", "  missing (3.1.0) sha256="),
        fixture.replace(
            "sha256=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "sha256=xyz",
        ),
        fixture.replace(
            "sha256=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "sha512=xyz",
        ),
        format!("{fixture}\nBUNDLED WITH\n   2.6.9\n"),
        fixture.replace("   2.6.9\n", ""),
        fixture.replace("   ruby 3.1.2p20 (jruby 9.4.0.0)\n", ""),
        fixture.replace("   2.6.9", "   invalid"),
        fixture.replace("GIT", "<<<<<<< HEAD"),
    ];
    for (index, text) in cases.into_iter().enumerate() {
        assert!(Lockfile::parse(&text).is_err(), "case {index}");
    }
}

#[test]
fn dependencies_do_not_leak_across_source_sections() {
    let text = "GEM\n  remote: https://rubygems.org\n  specs:\n    rack (1)\nGEM\n  remote: https://other.example\n  specs:\n      missing (>= 0)\nPLATFORMS\n  ruby\nDEPENDENCIES\n  rack\n";
    let error = Lockfile::parse(text).unwrap_err();
    assert_eq!(error.line, 8);
    assert!(error.message.contains("without a package"));
}

#[test]
fn incomplete_lockfiles_are_rejected() {
    for text in [
        "",
        "GEM\n",
        "GEM\n  remote: https://rubygems.org\n",
        "PLATFORMS\n  ruby\nDEPENDENCIES\n",
    ] {
        assert!(Lockfile::parse(text).is_err());
    }
}

#[test]
fn invalid_public_fields_cannot_inject_sections_or_disappear_on_render() {
    let lock = Lockfile::parse(include_str!("fixtures/whatweb.lock")).unwrap();
    let mut injected = lock.clone();
    injected.sources[0].remotes[0].push_str("\n  remote: https://unexpected.example/");
    assert!(injected.render().is_err());
    let mut unknown = lock.clone();
    unknown.sources[0]
        .options
        .insert("unknown".into(), "discarded".into());
    assert!(unknown.render().is_err());
    let mut duplicate = lock;
    let package = duplicate.sources[0].packages[0].clone();
    duplicate.sources[0].packages.push(package);
    assert!(duplicate.render().is_err());
}
