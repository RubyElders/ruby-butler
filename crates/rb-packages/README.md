# rb-packages

Standalone Ruby bundle preparation: read `rbproject.toml`/KDL or a trusted Gemfile, resolve dependencies, and install registry, Git and path gems into `.rb`, including native extensions. Work reports through `rb-task` events.

```rust,no_run
use rb_packages::{PrepareOptions, prepare};
use rb_task::TaskEvents;

let bundle = prepare(
    PrepareOptions::new("my-app", "ruby", "gem-cache"),
    TaskEvents::default(),
)?;
let environment = bundle.installed.environment();
# Ok::<(), anyhow::Error>(())
```

`rb --package-backend native sync` opts into this pipeline; `RB_PACKAGE_BACKEND=native` also applies to `rb x` and `rb run`. Bundler remains the default backend. Native `sync`, `x` and `run` reuse a successful installation with matching dependency inputs from `.rb/prepared.json` without preparing again. Unrelated launcher rebuilds do not invalidate prepared state; environment variables are tracked when dependencies read them. Library callers enable `PrepareOptions.record_state` to use `prepared`. `PreparedBundle.fingerprint` identifies resolved packages, provenance, activation and Ruby/build compatibility independently of input format; `changed` reports installation changes.

The returned environment supports real RubyGems and Bundler. Bundler does not resolve or install; RubyGems still handles runtime inspection and specification serialization. Gemfile conversion is cached under `.rb`. Cache inputs are dependency manifests, loaded dependency files and declared path dependencies. The selected Ruby installation is identified by its executable path; application and Ruby installation contents are not scanned. Other Gemfile filesystem inputs must be declared to `SnapshotCache::input`. Path gems use snapshots - prepare again after edits. Git/path sources require gemspecs.

Valid locked graphs are reused, with selected source metadata fetched concurrently and checked for dependency, checksum and Ruby compatibility. Changed graphs are resolved and written as compatible lockfiles. Matching resolved bundles reuse their installation without staging a copy; missing Git/path snapshots still go through repair. Existing lockfile platforms are resolved together when the lock needs refreshing; only the local platform is installed. Git/path declarations may be platform-scoped when they apply to a resolution target and their sources agree across targets. Rakefile builds use the resolved Rake or a bundled Rake from the selected Ruby, without adding a runtime dependency.

Transitive registry dependencies prefer the parent source, then fall back to the global source when absent. Competing scoped sources require an explicit declaration.

Registry authentication shims Bundler's local config, environment and global config lookup, using source URL or hostname keys; Bundler is not loaded. Native command credentials take priority for an exact source URL. Configure them in `$XDG_CONFIG_HOME/rb/auth.toml` (default `~/.config/rb/auth.toml`; `%APPDATA%/rb/auth.toml` on Windows):

```toml
[auth."https://rubygems.pkg.github.com/OWNER/"]
command = ["gh", "auth", "token", "--hostname", "github.com", "--user", "USERNAME"]
username = "USERNAME"
```

Commands run directly, without a shell, from the config directory. Stdout supplies the token for HTTP Basic authentication; credentials stay in memory for the invocation and out of source URLs and lockfiles. Offline preparation does not run credential commands.

Credential stores, different sources for the same gem across targets, custom lock paths, plugins and MicroGems application runtime are not supported yet.

Run `cargo test -p rb-packages`, or try `cargo run -p rb-packages --example prepare -- /path/to/app /path/to/ruby /path/to/cache`.

Native CLI preparation shares downloads under `$XDG_CACHE_HOME/rb` (normally `~/.cache/rb` on Linux). Archives are scoped by normalized registry URL and verified SHA-256. Installed bundles remain project-local. Prepared-state checks inspect dependency inputs, lockfiles and recorded installation state; installed gem payloads are not scanned.
