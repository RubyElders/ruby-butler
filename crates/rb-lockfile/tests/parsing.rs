use rb_lockfile::Lockfile;

#[test]
fn windows_line_endings_are_accepted() {
    let text = include_str!("fixtures/whatweb.lock");
    assert_eq!(
        Lockfile::parse(text).unwrap(),
        Lockfile::parse(&text.replace('\n', "\r\n")).unwrap()
    );
}

#[test]
fn legacy_environment_indentation_is_accepted() {
    let text = include_str!("fixtures/mixed.lock");
    let legacy = text
        .replace("   ruby ", "  ruby ")
        .replace("   2.6.9", "  2.6.9");
    assert_eq!(
        Lockfile::parse(text).unwrap(),
        Lockfile::parse(&legacy).unwrap()
    );
}
