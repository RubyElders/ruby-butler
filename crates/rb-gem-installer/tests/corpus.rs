use std::{fs, path::Path};

fn archives(root: &Path, found: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(root).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            archives(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "gem") {
            found.push(path);
        }
    }
}

#[test]
#[ignore = "requires RB_GEM_CORPUS pointing to the public archive corpus"]
fn installs_the_archive_corpus() {
    let root = std::env::var_os("RB_GEM_CORPUS").expect("Set RB_GEM_CORPUS");
    let mut found = Vec::new();
    archives(Path::new(&root), &mut found);
    assert!(!found.is_empty(), "Corpus contains no archives");
    found.sort();
    let mut failures = Vec::new();
    for path in &found {
        let root = tempfile::tempdir().unwrap();
        if let Err(error) =
            rb_gem_installer::install(fs::File::open(path).unwrap(), root.path().join("gem"))
        {
            failures.push(format!("{}: {error:#}", path.display()));
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} failed:\n{}",
        failures.len(),
        found.len(),
        failures.join("\n")
    );
    println!("Installed {} archives", found.len());
}
