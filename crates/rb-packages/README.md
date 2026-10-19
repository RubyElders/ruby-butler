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

`rb --package-backend native sync` opts into this pipeline; `RB_PACKAGE_BACKEND=native` also applies to `rb x` and `rb run`. Bundler remains the default backend. Native `x` and `run` reuse a validated environment from `.rb/prepared.json` without preparing again; explicit `sync` always prepares. Library callers enable `PrepareOptions.record_state` to use `prepared`.

The returned environment supports real RubyGems and Bundler. Bundler does not resolve or install; RubyGems still handles runtime inspection and specification serialization. Gemfile conversion is cached under `.rb`. Cached Gemfiles must depend on tracked files and environment; generated `.rb`, `log`, `tmp` directories and lockfiles are excluded. Path gems use snapshots - prepare again after edits. Git/path sources require gemspecs.

Existing lockfile platforms are resolved together when the lock needs refreshing; only the local platform is installed. Git/path declarations may be platform-scoped when they apply to a resolution target and their sources agree across targets. Rakefile builds use the resolved Rake or a bundled Rake from the selected Ruby, without adding a runtime dependency.

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
