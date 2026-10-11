# rb-gem-installer

Verifies a `.gem` archive and extracts its payload into a new directory. The parent directory must exist and be controlled by the caller; installation does not replace existing gems. Extraction happens in a private staging directory and failed installs are removed.

```rust,no_run
let archive = std::fs::File::open("rake-13.2.1.gem")?;
let metadata = rb_gem_installer::install(archive, "gems/rake-13.2.1")?;
println!("Installed {}", metadata.id);
# Ok::<(), anyhow::Error>(())
```

Regular files and directories are supported. Relative symlinks to payload files and directories are supported, including chains. Links outside the installation, cycles, hard links and special files are rejected. Identical repeated files are deduplicated; conflicting paths are rejected. Archive size, expanded size and entry count are bounded. File permissions are preserved on Unix subject to the process umask, without special permission bits. Signature trust follows `rb-gem-archive` and is not verified.

Native compilation is opt-in. `install_with_builder` builds in staging and publishes only after every declared extension succeeds. `NativeBuilder::build` can also build an already extracted directory; callers manage staging when using it directly.

```rust,no_run
use rb_gem_installer::{BuildOptions, NativeBuilder, install_with_builder};

let builder = NativeBuilder::new(BuildOptions::new("ruby"))?;
let archive = std::fs::File::open("json.gem")?;
let (metadata, report) = install_with_builder(archive, "gems/json", &builder, |_| {})?;
println!("Built {} outputs for {}", report.artifacts.len(), metadata.id);
# Ok::<(), anyhow::Error>(())
```

Builders support extconf/make, rake and mkrf, configure, CMake and Cargo. Ruby configuration comes from the selected interpreter; its development headers and the required build tools must be available. Build arguments, dependency load paths, environment, tool paths, job count and per-command timeout are caller-controlled. Structured events stream stdout, stderr and command stages without a reporting dependency. Build logs retained in errors are bounded; callbacks receive the full stream.

Collected outputs are stored under `.rb-native/<architecture>/<Ruby ABI>` and copied into the first require path. Reports also include native libraries written directly into gem require paths or extension directories. This crate does not resolve build dependencies, generate executable wrappers or activate gems for application runtime. Build subprocesses preload `rb-microgems` for minimal RubyGems API compatibility. Supply exact installed build dependencies through `BuildOptions::dependencies`; missing gems and unsupported APIs fail explicitly. Bundler and full RubyGems are not loaded.

The regular tests include a public gem checked against RubyGems extraction. To run the larger local corpus, set `RB_GEM_CORPUS` to its absolute `gems` directory and run `cargo test -p rb-gem-installer --test corpus -- --ignored --nocapture`.

The Ruby configuration helper is a standalone gem in `gems/rb-gem-installer`. Its files are embedded in this crate and materialized for the selected interpreter without a gem installation. Run its Ruby tests with `ruby gems/rb-gem-installer/test/all_test.rb` from the repository root.
