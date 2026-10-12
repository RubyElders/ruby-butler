use anyhow::{Result, ensure};
use rb_gemfile::Evaluator;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

fn files(root: &Path, output: &mut Vec<PathBuf>) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            files(&path, output)?;
        } else {
            output.push(path);
        }
    }
    Ok(())
}

fn classify(error: &str) -> &'static str {
    if error.contains("Git options require") || error.contains("Conflicting Git selectors") {
        "unsupported Git option combination"
    } else if error.contains("No such file")
        || error.contains("gemspec")
        || error.contains("cannot load such file")
    {
        "missing companion or gemspec"
    } else if error.contains("Ruby versions") {
        "older Ruby required"
    } else if error.contains("exists?") {
        "removed Ruby API"
    } else if error.contains("database.yml") || error.contains("ENV") {
        "application configuration required"
    } else if error.contains("Bundler::Plugin") {
        "plugin DSL unsupported"
    } else {
        "evaluation failure"
    }
}

fn canonical(mut lock: rb_lockfile::Lockfile) -> rb_lockfile::Lockfile {
    lock.platforms.sort();
    for source in &mut lock.sources {
        source.remotes.sort();
        source.packages.sort_by_key(|package| package.identity());
    }
    lock.sources
        .sort_by(|a, b| (a.kind, &a.remotes, &a.options).cmp(&(b.kind, &b.remotes, &b.options)));
    lock
}

fn main() -> Result<()> {
    ensure!(
        std::env::var_os("BUNDLE_GEMFILE").is_none(),
        "Unset BUNDLE_GEMFILE for a corpus run"
    );
    let mut arguments = std::env::args_os().skip(1);
    let root = PathBuf::from(arguments.next().expect("Pass the gemfile-corpus directory"));
    let ruby = PathBuf::from(arguments.next().unwrap_or_else(|| "ruby".into()));
    let mut gemfiles = Vec::new();
    files(&root.join("gemfiles"), &mut gemfiles)?;
    gemfiles.retain(|p| {
        p.file_name()
            .is_some_and(|n| n == "Gemfile" || n == "gems.rb")
    });
    gemfiles.sort();
    let total = gemfiles.len();
    let input = Arc::new(Mutex::new(gemfiles.into_iter()));
    let results = Arc::new(Mutex::new(Vec::new()));
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let input = input.clone();
            let results = results.clone();
            let ruby = ruby.clone();
            let root = root.clone();
            scope.spawn(move || loop {
            let Some(path) = input.lock().unwrap().next() else { break; };
            let result = (|| -> Result<Value> {
                let project = tempfile::tempdir()?;
                fs::copy(&path, project.path().join(path.file_name().unwrap()))?;
                let input = Evaluator::new(&ruby).evaluate(project.path().join(path.file_name().unwrap()))?;
                let exported = input.manifest.to_toml()?;
                let roundtrip: rb_gemfile::Manifest = toml::from_str(&exported)?;
                ensure!(roundtrip.to_toml()? == exported, "Manifest roundtrip changed");
                let git = input.manifest.dependencies.values().any(|d| d.git.is_some() || d.variants.iter().any(|d| d.git.is_some()));
                let local = !input.manifest.gemspecs.is_empty() || input.manifest.dependencies.values().any(|d| d.path.is_some() || d.variants.iter().any(|d| d.path.is_some()));
                Ok(json!({"status":"loaded", "dependencies":input.manifest.dependencies.len(), "git":git, "path":local}))
            })();
            let row = match result {
                Ok(value) => json!({"path":path.strip_prefix(&root).unwrap(), "result":value}),
                Err(error) => { let error = format!("{error:#}"); json!({"path":path.strip_prefix(&root).unwrap(), "result":{"status":"failed", "category":classify(&error), "error":error}}) }
            };
            results.lock().unwrap().push(row);
        });
        }
    });
    let mut results = Arc::try_unwrap(results).unwrap().into_inner().unwrap();
    results.sort_by_key(|r| r["path"].as_str().unwrap().to_owned());
    let loaded = results
        .iter()
        .filter(|r| r["result"]["status"] == "loaded")
        .count();
    let mut locks = Vec::new();
    files(&root.join("lockfiles"), &mut locks)?;
    locks.sort();
    let mut lock_failures = Vec::new();
    for path in &locks {
        let result = (|| -> Result<()> {
            let lock = rb_lockfile::Lockfile::parse(&fs::read_to_string(path)?)?;
            let rendered = lock.render()?;
            ensure!(
                canonical(rb_lockfile::Lockfile::parse(&rendered)?) == canonical(lock),
                "Lockfile roundtrip changed"
            );
            Ok(())
        })();
        if let Err(error) = result {
            lock_failures
                .push(json!({"path":path.strip_prefix(&root)?, "error":format!("{error:#}")}));
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &json!({"gemfiles":{"total":total,"loaded":loaded,"failed":total-loaded,"results":results},"lockfiles":{"total":locks.len(),"passed":locks.len()-lock_failures.len(),"failures":lock_failures}})
        )?
    );
    Ok(())
}
