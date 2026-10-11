use flate2::{Compression, write::GzEncoder};
use rb_gem_installer::{BuildEvent, BuildOptions, NativeBuilder};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

fn ruby() -> PathBuf {
    std::env::var_os("RB_GEM_BUILD_RUBY")
        .unwrap_or_else(|| "ruby".into())
        .into()
}
fn builder() -> NativeBuilder {
    let mut options = BuildOptions::new(ruby());
    options
        .environment
        .insert("CCACHE_DISABLE".into(), "1".into());
    NativeBuilder::new(options).unwrap()
}
fn metadata(extension: &str) -> rb_gem_archive::GemMetadata {
    let root = tempfile::tempdir().unwrap();
    let archive = archive(extension, &[]);
    rb_gem_installer::install(archive.as_slice(), root.path().join("gem")).unwrap()
}
fn gzip(bytes: &[u8]) -> Vec<u8> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(bytes).unwrap();
    encoder.finish().unwrap()
}
fn archive(extension: &str, files: &[(&str, &[u8])]) -> Vec<u8> {
    archive_extensions(&[extension], files)
}
fn archive_extensions(extensions: &[&str], files: &[(&str, &[u8])]) -> Vec<u8> {
    fn tar(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
        let mut builder = tar::Builder::new(Vec::new());
        for (path, bytes) in files {
            let mut header = tar::Header::new_gnu();
            header.set_mode(0o644);
            header.set_size(bytes.len() as u64);
            header.set_cksum();
            builder
                .append_data(&mut header, path, bytes.as_slice())
                .unwrap();
        }
        builder.into_inner().unwrap()
    }
    let data = tar(&files
        .iter()
        .map(|(path, bytes)| (*path, bytes.to_vec()))
        .collect::<Vec<_>>());
    let extensions = serde_json::to_string(extensions).unwrap();
    tar(&[("metadata.gz", gzip(format!("name: dinner\nversion: 1.2.3\nplatform: ruby\nextensions: {extensions}\nrequire_paths: [lib]\n").as_bytes())), ("data.tar.gz", gzip(&data))])
}
fn copy_fixture(kind: &str, root: &Path) {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/native")
        .join(kind);
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        fs::copy(entry.path(), root.join(entry.file_name())).unwrap();
    }
}
fn rake_options() -> BuildOptions {
    let output = Command::new(ruby()).args(["--disable=gems", "-rrbconfig", "-e", "puts Dir.glob(File.join(RbConfig::CONFIG.fetch('rubylibprefix'), 'gems', RbConfig::CONFIG.fetch('ruby_version'), 'gems', 'rake-*', 'lib')).last"]).output().unwrap();
    assert!(output.status.success());
    let path = PathBuf::from(String::from_utf8(output.stdout).unwrap().trim());
    let version = Command::new(ruby())
        .args(["--disable=gems", "-I"])
        .arg(&path)
        .args(["-rrake/version", "-e", "puts Rake::VERSION"])
        .output()
        .unwrap();
    assert!(version.status.success());
    let mut options = BuildOptions::new(ruby());
    options.dependencies.push(rb_microgems::InstalledGem {
        name: "rake".into(),
        version: String::from_utf8(version.stdout).unwrap().trim().into(),
        platform: "ruby".into(),
        root: path.parent().unwrap().into(),
        require_paths: vec!["lib".into()],
        metadata: Default::default(),
        executables: vec!["rake".into()],
        bindir: "exe".into(),
    });
    options
}

fn require(root: &Path, program: &str) {
    let output = Command::new(ruby())
        .args(["--disable=gems", "-I"])
        .arg(root.join("lib"))
        .args(["-e", program])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "42");
}

#[test]
fn failed_build_rolls_back_and_streams_diagnostics() {
    let root = tempfile::tempdir().unwrap();
    let archive = archive(
        "extconf.rb",
        &[("extconf.rb", b"warn 'missing dependency'; exit 1\n")],
    );
    let mut output = Vec::new();
    let error = rb_gem_installer::install_with_builder(
        archive.as_slice(),
        root.path().join("gem"),
        &builder(),
        |event| {
            if let BuildEvent::Output { bytes, .. } = event {
                output.extend(bytes);
            }
        },
    )
    .unwrap_err();
    assert!(format!("{error:#}").contains("missing dependency"));
    assert!(String::from_utf8_lossy(&output).contains("missing dependency"));
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn timed_out_build_rolls_back() {
    let root = tempfile::tempdir().unwrap();
    let archive = archive(
        "extconf.rb",
        &[("extconf.rb", b"puts 'started'; STDOUT.flush; sleep 10\n")],
    );
    let mut options = BuildOptions::new(ruby());
    options.timeout = Duration::from_secs(1);
    let builder = NativeBuilder::new(options).unwrap();
    let error = rb_gem_installer::install_with_builder(
        archive.as_slice(),
        root.path().join("gem"),
        &builder,
        |_| {},
    )
    .unwrap_err();
    assert!(format!("{error:#}").contains("timed out"));
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn unsupported_builders_and_escaping_declarations_are_rejected() {
    for extension in ["script.py", "../extconf.rb"] {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("script.py"), "").unwrap();
        assert!(
            builder()
                .build(&metadata(extension), root.path(), |_| {})
                .is_err()
        );
    }
}

#[test]
fn configure_builder_installs_outputs() {
    let root = tempfile::tempdir().unwrap();
    copy_fixture("configure", root.path());
    let mut configure_arguments = Vec::new();
    let report = builder()
        .build(&metadata("configure"), root.path(), |event| {
            if let BuildEvent::Started {
                stage: rb_gem_installer::BuildStage::Configure,
                arguments,
                ..
            } = event
            {
                configure_arguments = arguments;
            }
        })
        .unwrap();
    assert!(
        configure_arguments[1]
            .to_string_lossy()
            .starts_with("--prefix=./")
    );
    assert_eq!(report.extensions, [PathBuf::from("configure")]);
    assert!(!report.artifacts.is_empty());
    require(
        root.path(),
        "require 'configure_output'; puts CONFIGURE_BUILD_ANSWER",
    );
}

#[cfg(unix)]
#[test]
fn compiles_and_requires_a_c_extension_with_spaces_in_the_path() {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("gem with spaces");
    fs::create_dir(&directory).unwrap();
    copy_fixture("c", &directory);
    let mut started = Vec::new();
    let report = builder()
        .build(&metadata("extconf.rb"), &directory, |event| {
            if let BuildEvent::Started { stage, .. } = event {
                started.push(stage);
            }
        })
        .unwrap();
    assert_eq!(
        started,
        [
            rb_gem_installer::BuildStage::Configure,
            rb_gem_installer::BuildStage::Compile,
            rb_gem_installer::BuildStage::Install
        ]
    );
    assert!(!report.artifacts.is_empty());
    assert!(
        report
            .artifacts
            .iter()
            .any(|path| path.parent() == Some(Path::new("")))
    );
    assert!(
        report
            .artifacts
            .iter()
            .all(|path| directory.join(path).is_file())
    );
    require(
        &directory,
        "require 'butler_native'; puts ButlerNative.answer",
    );
    let library = fs::read_dir(directory.join("lib"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert!(
        report
            .artifacts
            .contains(&library.strip_prefix(&directory).unwrap().to_path_buf())
    );
}

#[cfg(unix)]
#[test]
fn compiles_and_requires_a_rust_extension() {
    let root = tempfile::tempdir().unwrap();
    copy_fixture("cargo", root.path());
    let report = builder()
        .build(&metadata("Cargo.toml"), root.path(), |_| {})
        .unwrap();
    assert!(!report.artifacts.is_empty());
    require(
        root.path(),
        "require 'butler_rust'; abort unless defined?(ButlerRust); puts 42",
    );
}

#[test]
fn rake_builder_uses_explicit_dependency_load_paths() {
    let root = tempfile::tempdir().unwrap();
    copy_fixture("rake", root.path());
    let output = Command::new(ruby()).args(["--disable=gems", "-rrbconfig", "-e", "puts Dir.glob(File.join(RbConfig::CONFIG.fetch('rubylibprefix'), 'gems', RbConfig::CONFIG.fetch('ruby_version'), 'gems', 'rake-*', 'lib')).last"]).output().unwrap();
    assert!(output.status.success());
    let path = String::from_utf8(output.stdout).unwrap();
    assert!(
        !path.trim().is_empty(),
        "Test Ruby must include its bundled rake gem"
    );
    let mut options = BuildOptions::new(ruby());
    options.load_paths.push(path.trim().into());
    let builder = NativeBuilder::new(options).unwrap();
    let report = builder
        .build(&metadata("Rakefile"), root.path(), |_| {})
        .unwrap();
    assert!(!report.artifacts.is_empty());
    require(root.path(), "require 'rake_output'; puts RAKE_BUILD_ANSWER");
}

#[cfg(unix)]
#[test]
fn cmake_builder_compiles_an_extension() {
    let root = tempfile::tempdir().unwrap();
    fs::create_dir(root.path().join("ext")).unwrap();
    copy_fixture("cmake", &root.path().join("ext"));
    fs::write(
        root.path().join("butler_native.c"),
        include_bytes!("fixtures/native/c/butler_native.c"),
    )
    .unwrap();
    let report = builder()
        .build(&metadata("ext/CMakeLists.txt"), root.path(), |_| {})
        .unwrap();
    assert!(!report.artifacts.is_empty());
    require(
        root.path(),
        "require 'butler_native'; puts ButlerNative.answer",
    );
}

#[cfg(unix)]
#[test]
fn panicking_reporters_do_not_leave_build_processes_or_partial_installs() {
    let root = tempfile::tempdir().unwrap();
    let archive = archive(
        "extconf.rb",
        &[("extconf.rb", b"puts Process.pid; STDOUT.flush; sleep 10\n")],
    );
    let mut pid = None;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        rb_gem_installer::install_with_builder(
            archive.as_slice(),
            root.path().join("gem"),
            &builder(),
            |event| {
                if let BuildEvent::Output { bytes, .. } = event {
                    pid = String::from_utf8_lossy(&bytes).trim().parse::<i32>().ok();
                    panic!("reporter failed");
                }
            },
        )
    }));
    assert!(result.is_err());
    let pid = rustix::process::Pid::from_raw(pid.unwrap()).unwrap();
    assert!(rustix::process::test_kill_process(pid).is_err());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn large_stdout_and_stderr_do_not_deadlock_or_lose_final_errors() {
    let root = tempfile::tempdir().unwrap();
    let archive = archive("extconf.rb", &[("extconf.rb", b"STDOUT.write('x' * 2000000); STDERR.write('y' * 2000000); STDERR.puts('final failure'); exit 1\n")]);
    let mut bytes = 0;
    let error = rb_gem_installer::install_with_builder(
        archive.as_slice(),
        root.path().join("gem"),
        &builder(),
        |event| {
            if let BuildEvent::Output { bytes: output, .. } = event {
                bytes += output.len();
            }
        },
    )
    .unwrap_err();
    assert!(format!("{error:#}").contains("final failure"));
    assert!(bytes >= 4_000_000);
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn failed_builds_preserve_mkmf_diagnostics_before_rollback() {
    let root = tempfile::tempdir().unwrap();
    let archive = archive(
        "extconf.rb",
        &[(
            "extconf.rb",
            b"File.write('mkmf.log', 'missing compiler header'); exit 1\n",
        )],
    );
    let error = rb_gem_installer::install_with_builder(
        archive.as_slice(),
        root.path().join("gem"),
        &builder(),
        |_| {},
    )
    .unwrap_err();
    assert!(format!("{error:#}").contains("missing compiler header"));
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn missing_build_tools_roll_back() {
    let root = tempfile::tempdir().unwrap();
    let archive = archive(
        "extconf.rb",
        &[("extconf.rb", b"File.write('Makefile', '')\n")],
    );
    let mut options = BuildOptions::new(ruby());
    options.make = Some(root.path().join("missing-make").into_os_string());
    let error = rb_gem_installer::install_with_builder(
        archive.as_slice(),
        root.path().join("gem"),
        &NativeBuilder::new(options).unwrap(),
        |_| {},
    )
    .unwrap_err();
    assert!(format!("{error:#}").contains("Cannot start build tool"));
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn empty_require_paths_install_into_the_gem_root() {
    let root = tempfile::tempdir().unwrap();
    copy_fixture("c", root.path());
    let mut metadata = metadata("extconf.rb");
    metadata.require_paths = vec![String::new()];
    let report = builder().build(&metadata, root.path(), |_| {}).unwrap();
    assert!(!report.artifacts.is_empty());
    let output = Command::new(ruby())
        .args(["--disable=gems", "-I"])
        .arg(root.path())
        .args(["-e", "require 'butler_native'; puts ButlerNative.answer"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "42");
}

#[cfg(unix)]
#[test]
fn dot_slash_require_paths_are_the_gem_root() {
    let root = tempfile::tempdir().unwrap();
    copy_fixture("c", root.path());
    let mut metadata = metadata("extconf.rb");
    metadata.require_paths = vec!["./".to_owned()];
    let report = builder().build(&metadata, root.path(), |_| {}).unwrap();
    assert!(!report.artifacts.is_empty());
    let output = Command::new(ruby())
        .args(["--disable=gems", "-I"])
        .arg(root.path())
        .args(["-e", "require 'butler_native'; puts ButlerNative.answer"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "42");
}

#[test]
fn make_environment_override_is_used() {
    let root = tempfile::tempdir().unwrap();
    let archive = archive(
        "extconf.rb",
        &[("extconf.rb", b"File.write('Makefile', '')\n")],
    );
    let mut options = BuildOptions::new(ruby());
    options.environment.insert(
        "MAKE".into(),
        root.path().join("missing-make").into_os_string(),
    );
    let error = rb_gem_installer::install_with_builder(
        archive.as_slice(),
        root.path().join("gem"),
        &NativeBuilder::new(options).unwrap(),
        |_| {},
    )
    .unwrap_err();
    assert!(format!("{error:#}").contains("Cannot start build tool"));
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

#[cfg(unix)]
#[test]
fn explicit_build_dependency_is_activated_without_external_rubygems() {
    let root = tempfile::tempdir().unwrap();
    let dependency = tempfile::tempdir().unwrap();
    fs::create_dir(dependency.path().join("lib")).unwrap();
    fs::write(
        dependency.path().join("lib/helper_api.rb"),
        "HELPER_ANSWER = 42\n",
    )
    .unwrap();
    copy_fixture("c", root.path());
    copy_fixture("compatibility", root.path());
    let mut options = BuildOptions::new(ruby());
    options
        .environment
        .insert("CCACHE_DISABLE".into(), "1".into());
    options.arguments.push("--with-extra=a b".into());
    options.environment.insert("MAKEFLAGS".into(), "-s".into());
    options.dependencies.push(rb_microgems::InstalledGem {
        name: "build-helper".into(),
        version: "2.1.0".into(),
        platform: "ruby".into(),
        root: dependency.path().into(),
        require_paths: vec!["lib".into()],
        metadata: Default::default(),
        executables: vec![],
        bindir: "bin".into(),
    });
    let builder = NativeBuilder::new(options).unwrap();
    let report = builder
        .build(&metadata("extconf.rb"), root.path(), |_| {})
        .unwrap();
    assert!(!report.artifacts.is_empty());
    require(
        root.path(),
        "require 'butler_native'; puts ButlerNative.answer",
    );
}

#[cfg(unix)]
#[test]
fn native_build_can_load_a_completed_native_dependency() {
    let dependency = tempfile::tempdir().unwrap();
    copy_fixture("c", dependency.path());
    let dependency_metadata = metadata("extconf.rb");
    builder()
        .build(&dependency_metadata, dependency.path(), |_| {})
        .unwrap();
    let root = tempfile::tempdir().unwrap();
    copy_fixture("c", root.path());
    copy_fixture("dependent", root.path());
    let mut consumer = metadata("extconf.rb");
    consumer.id.name = "guest".into();
    let mut options = BuildOptions::new(ruby());
    options
        .environment
        .insert("CCACHE_DISABLE".into(), "1".into());
    options.environment.insert(
        "EXPECTED_DEPENDENCY_ROOT".into(),
        dependency.path().as_os_str().into(),
    );
    options.dependencies.push(rb_microgems::InstalledGem {
        name: "dinner".into(),
        version: "1.2.3".into(),
        platform: "ruby".into(),
        root: dependency.path().into(),
        require_paths: vec!["lib".into()],
        metadata: Default::default(),
        executables: vec![],
        bindir: "bin".into(),
    });
    let report = NativeBuilder::new(options)
        .unwrap()
        .build(&consumer, root.path(), |_| {})
        .unwrap();
    assert!(!report.artifacts.is_empty());
    require(
        root.path(),
        "require 'butler_native'; puts ButlerNative.answer",
    );
}

#[cfg(unix)]
#[test]
fn rake_builder_activates_an_explicit_dependency_without_load_path_overrides() {
    let root = tempfile::tempdir().unwrap();
    copy_fixture("rake", root.path());
    let options = rake_options();
    let report = NativeBuilder::new(options)
        .unwrap()
        .build(&metadata("Rakefile"), root.path(), |_| {})
        .unwrap();
    assert!(!report.artifacts.is_empty());
    require(root.path(), "require 'rake_output'; puts RAKE_BUILD_ANSWER");
}

#[cfg(unix)]
#[test]
fn cpp_extension_uses_the_callers_compiler_flags() {
    let root = tempfile::tempdir().unwrap();
    copy_fixture("cpp", root.path());
    let mut options = BuildOptions::new(ruby());
    options
        .environment
        .insert("CCACHE_DISABLE".into(), "1".into());
    options.arguments.push("--with-cxxflags=-std=c++17".into());
    let report = NativeBuilder::new(options)
        .unwrap()
        .build(&metadata("extconf.rb"), root.path(), |_| {})
        .unwrap();
    assert!(!report.artifacts.is_empty());
    require(root.path(), "require 'butler_cpp'; puts ButlerCpp.answer");
}

#[cfg(unix)]
#[test]
fn staged_native_installation_loads_through_manifest_activation() {
    let root = tempfile::tempdir().unwrap();
    let destination = root.path().join("installed gem");
    let archive = archive(
        "ext/native/extconf.rb",
        &[
            (
                "ext/native/extconf.rb",
                include_bytes!("fixtures/native/c/extconf.rb"),
            ),
            (
                "ext/native/butler_native.c",
                include_bytes!("fixtures/native/c/butler_native.c"),
            ),
            (
                "lib/dinner.rb",
                include_bytes!("fixtures/native/runtime/dinner.rb"),
            ),
        ],
    );
    let (metadata, report) = rb_gem_installer::install_with_builder(
        archive.as_slice(),
        &destination,
        &builder(),
        |_| {},
    )
    .unwrap();
    assert_eq!(report.extensions, [PathBuf::from("ext/native/extconf.rb")]);
    assert!(
        report
            .artifacts
            .iter()
            .all(|path| destination.join(path).is_file())
    );
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    let gem = rb_microgems::InstalledGem {
        name: metadata.id.name,
        version: metadata.id.version.to_string(),
        platform: metadata.id.platform.to_string(),
        root: destination.clone(),
        require_paths: metadata.require_paths,
        metadata: metadata.metadata,
        executables: metadata.executables,
        bindir: metadata.bindir,
    };
    let environment = rb_microgems::Environment::new(&[gem], &[]).unwrap();
    let output = Command::new(ruby())
        .args(["--disable=gems", "-I"])
        .arg(environment.load_path())
        .args(["-rmicrogems/compatibility", "-e", "require 'dinner'"])
        .env("RB_MICROGEMS_MANIFEST", environment.manifest_path())
        .env("EXPECTED_GEM_ROOT", &destination)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "42");
}

#[cfg(unix)]
#[test]
fn a_later_extension_failure_rolls_back_completed_extensions() {
    let root = tempfile::tempdir().unwrap();
    let archive = archive_extensions(
        &["ext/first/extconf.rb", "ext/second/extconf.rb"],
        &[
            (
                "ext/first/extconf.rb",
                include_bytes!("fixtures/native/c/extconf.rb"),
            ),
            (
                "ext/first/butler_native.c",
                include_bytes!("fixtures/native/c/butler_native.c"),
            ),
            (
                "ext/second/extconf.rb",
                include_bytes!("fixtures/native/failing/extconf.rb"),
            ),
        ],
    );
    let mut completed = Vec::new();
    let error = rb_gem_installer::install_with_builder(
        archive.as_slice(),
        root.path().join("gem"),
        &builder(),
        |event| {
            if let BuildEvent::Finished {
                extension,
                stage: rb_gem_installer::BuildStage::Install,
                ..
            } = event
            {
                completed.push(extension);
            }
        },
    )
    .unwrap_err();
    assert_eq!(completed, [PathBuf::from("ext/first/extconf.rb")]);
    assert!(format!("{error:#}").contains("second extension failed"));
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn mkrf_builder_generates_and_runs_its_rakefile() {
    let root = tempfile::tempdir().unwrap();
    copy_fixture("rake", root.path());
    fs::rename(root.path().join("Rakefile"), root.path().join("build.rake")).unwrap();
    copy_fixture("mkrf", root.path());
    let mut stages = Vec::new();
    let report = NativeBuilder::new(rake_options())
        .unwrap()
        .build(&metadata("mkrf_conf.rb"), root.path(), |event| {
            if let BuildEvent::Finished { stage, .. } = event {
                stages.push(stage);
            }
        })
        .unwrap();
    assert_eq!(
        stages,
        [
            rb_gem_installer::BuildStage::Configure,
            rb_gem_installer::BuildStage::Compile
        ]
    );
    assert_eq!(report.extensions, [PathBuf::from("mkrf_conf.rb")]);
    assert!(!report.artifacts.is_empty());
    assert!(root.path().join("Rakefile").is_file());
    require(root.path(), "require 'rake_output'; puts RAKE_BUILD_ANSWER");
}
