# rb-lockfile

Standalone parsing and deterministic rendering of Bundler-compatible lockfiles. No Ruby, network, filesystem or resolver calls are made by the library.

```rust
use rb_lockfile::Lockfile;

let text = "GEM\n  remote: https://rubygems.org/\n  specs:\n    rack (3.1.0)\n\nPLATFORMS\n  ruby\n\nDEPENDENCIES\n  rack\n\nBUNDLED WITH\n   2.6.9\n";
let lock = Lockfile::parse(text)?;
let rendered = lock.render()?;
assert_eq!(rendered, text);
# Ok::<(), rb_lockfile::ParseError>(())
```

Source sections own their package variants and dependencies. Registry remotes, Git options and revisions, path sources, pinned roots, optional SHA-256 checksums, Ruby version text and Bundler version are preserved. Repeated package dependency declarations and registry sections without remotes are retained. Checksums for the implicit Bundler version do not require a package spec. Absent checksums remain absent. Platform spelling stays as recorded; target matching belongs to `rb-gem-types`.

Malformed, duplicate or unsupported sections and checksum algorithms return `ParseError` rather than being silently discarded. Rendering validates the result and refuses fields that would change meaning. Source, package, platform and map ordering is deterministic; this is a semantic round trip, not a text editor.

Run `cargo test -p rb-lockfile`. The offline suite includes frozen Bundler locks from the resolver corpus and mixed registry, Git, path and checksum fixtures.
