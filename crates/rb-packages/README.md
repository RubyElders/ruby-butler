# rb-packages

Standalone Ruby bundle preparation: read `rbproject.toml`/KDL or a trusted Gemfile, resolve dependencies, and install registry, Git and path gems into `.rb`, including native extensions. Work reports through `rb-task` events. The CLI is not connected yet.

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

The returned environment supports real RubyGems and Bundler. Bundler does not resolve or install; RubyGems still handles runtime inspection and specification serialization. Gemfile conversion is cached under `.rb`; Gemfiles must not read generated bundle files. Path gems use snapshots - prepare again after edits. Git/path sources require gemspecs.

Private registry authentication, cross-registry transitive searching, changed multi-target locks, platform-specific Git/path sources, custom lock paths, plugins and MicroGems application runtime are not supported yet.

Run `cargo test -p rb-packages`, or try `cargo run -p rb-packages --example prepare -- /path/to/app /path/to/ruby /path/to/cache`.
