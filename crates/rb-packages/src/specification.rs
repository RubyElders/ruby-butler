use anyhow::{Context, Result, ensure};
use rb_gem_archive::GemMetadata;
use serde_json::json;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

#[cfg(test)]
pub(crate) fn write(
    ruby: &Path,
    metadata: &GemMetadata,
    archive_version: &str,
    home: &Path,
) -> Result<PathBuf> {
    write_all(
        ruby,
        &[(metadata.clone(), archive_version.into(), home.into())],
    )?;
    Ok(filename(metadata, archive_version, home))
}

pub(crate) fn filename(metadata: &GemMetadata, archive_version: &str, home: &Path) -> PathBuf {
    home.join(crate::constants::SPECIFICATIONS_DIRECTORY)
        .join(format!("{}-{archive_version}.gemspec", metadata.id.name))
}

#[cfg(test)]
pub(crate) fn write_all(ruby: &Path, records: &[(GemMetadata, String, PathBuf)]) -> Result<()> {
    write_all_with_inputs(
        ruby,
        records,
        &std::env::current_dir()?,
        &std::env::vars_os().collect(),
    )
}

pub(crate) fn write_all_with_inputs(
    ruby: &Path,
    records: &[(GemMetadata, String, PathBuf)],
    directory: &Path,
    environment: &std::collections::BTreeMap<std::ffi::OsString, std::ffi::OsString>,
) -> Result<()> {
    if records.is_empty() {
        return Ok(());
    }
    let records = records
        .iter()
        .map(|(metadata, archive_version, home)| {
            let filename = filename(metadata, archive_version, home);
            std::fs::create_dir_all(filename.parent().unwrap())?;
            Ok((record(metadata, archive_version), filename))
        })
        .collect::<Result<Vec<_>>>()?;
    let data = tempfile::NamedTempFile::new()?;
    std::fs::write(data.path(), serde_json::to_vec(&records)?)?;
    let helper = tempfile::Builder::new().suffix(".rb").tempfile()?;
    std::fs::write(
        helper.path(),
        include_str!("../../../gems/rb-gem-installer/lib/rb_gem_installer/specification.rb"),
    )?;
    let output = Command::new(ruby)
        .current_dir(directory).env_clear().envs(environment)
        .env_remove("RUBYOPT")
        .env_remove("RUBYLIB")
        .args([
            "-rrubygems",
            "-rjson",
            "-e",
            "require ARGV.shift; RubyButler::GemInstaller::Specification.write_all(JSON.parse(File.read(ARGV[0])))",
        ])
        .arg(helper.path())
        .arg(data.path())
        .output()
        .context("Writing installed specification")?;
    ensure!(
        output.status.success(),
        "Writing specification failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(())
}

fn record(metadata: &GemMetadata, archive_version: &str) -> serde_json::Value {
    let (_, platform) = archive_version
        .split_once('-')
        .unwrap_or((archive_version, "ruby"));
    json!({
        "name": metadata.id.name, "version": metadata.id.version.to_string(), "platform": platform,
        "ruby": metadata.ruby.to_string(), "rubygems": metadata.rubygems.to_string(),
        "dependencies": metadata.dependencies.iter().map(|(n,r)| (n.clone(),r.to_string())).collect::<std::collections::BTreeMap<_,_>>(),
        "require_paths": metadata.require_paths, "executables": metadata.executables, "bindir": metadata.bindir,
        "extensions": metadata.extensions, "files": metadata.files, "metadata": metadata.metadata,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        sync::{Arc, Mutex},
    };

    #[test]
    #[ignore = "requires RB_GEM_CORPUS and RubyGems for the archive oracle"]
    fn specifications_match_the_archive_corpus() {
        fn archives(root: &Path, files: &mut Vec<PathBuf>) {
            for entry in fs::read_dir(root).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    archives(&path, files);
                } else if path.extension().is_some_and(|extension| extension == "gem") {
                    files.push(path);
                }
            }
        }
        let root = std::env::var_os("RB_GEM_CORPUS").expect("Set RB_GEM_CORPUS");
        let ruby = std::env::var_os("RB_GEM_BUILD_RUBY").unwrap_or_else(|| "ruby".into());
        let mut paths = Vec::new();
        archives(Path::new(&root), &mut paths);
        paths.sort();
        assert!(!paths.is_empty());
        let paths = Arc::new(Mutex::new(paths.into_iter()));
        let failures = Arc::new(Mutex::new(Vec::new()));
        let count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        std::thread::scope(|scope| {
            for _ in 0..4 {
                let paths = paths.clone();
                let failures = failures.clone();
                let count = count.clone();
                let ruby = ruby.clone();
                scope.spawn(move || loop {
                let Some(archive) = paths.lock().unwrap().next() else { break; };
                let result = (|| -> Result<()> {
                    let metadata = rb_gem_archive::read(fs::File::open(&archive)?)?;
                    let home = tempfile::tempdir()?;
                    let specification = write(Path::new(&ruby), &metadata, &metadata.id.archive_version(), home.path())?;
                    let output = Command::new(&ruby).env_remove("RUBYOPT").env_remove("RUBYLIB")
                        .args(["-rrubygems/package", "-e", "a = Gem::Package.new(ARGV[0]).spec; b = Gem::Specification.load(ARGV[1]); fields = [:name, :version, :platform, :required_ruby_version, :required_rubygems_version, :raw_require_paths, :executables, :bindir, :extensions, :metadata]; fields.each { |field| raise field.to_s + ': ' + [a.public_send(field), b.public_send(field)].inspect unless a.public_send(field) == b.public_send(field) }; raise 'dependencies differ' unless a.runtime_dependencies.sort_by(&:name) == b.runtime_dependencies.sort_by(&:name)"])
                        .arg(&archive).arg(specification).output()?;
                    ensure!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
                    Ok(())
                })();
                count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if let Err(error) = result { failures.lock().unwrap().push(format!("{}: {error:#}", archive.display())); }
            });
            }
        });
        let failures = failures.lock().unwrap();
        println!(
            "Checked {} archive specifications",
            count.load(std::sync::atomic::Ordering::Relaxed)
        );
        assert!(
            failures.is_empty(),
            "{} failures:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }
}
