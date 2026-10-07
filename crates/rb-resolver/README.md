# rb-resolver

Standalone Ruby gem dependency resolution using PubGrub and `rb-gem-types`.
Providers supply candidate metadata; this crate does no HTTP, Git, installation,
Ruby execution or terminal reporting.

```rust
use rb_gem_types::{PackageId, Platform};
use rb_resolver::{InMemoryIndex, Package, ResolveRequest, resolve};
use std::collections::BTreeMap;

let rack = Package::new(PackageId {
    name: "rack".into(),
    version: "3.1.0".parse()?,
    platform: Platform::parse("ruby"),
});
let index = InMemoryIndex::new([rack]);
let request = ResolveRequest::new(
    BTreeMap::from([("rack".into(), "~> 3.0".parse()?)]),
    "3.4.0".parse()?,
    Platform::parse("x86_64-linux-gnu"),
);
let packages = resolve(&index, &request)?;
assert_eq!(packages[0].id.full_name(), "rack-3.1.0");
# Ok::<(), Box<dyn std::error::Error>>(())
```

Candidates are filtered by Ruby and target platform. Selection prefers an exact
locked identity, then a locked version, then the newest version with the best
platform score. Locks are preferences, so conflicts can move to another version.
Results are sorted by gem name, independently of provider order.

Prereleases are eligible when requested explicitly, locked, enabled through
`allow_prereleases`, or when all Ruby/platform-compatible candidates are
prereleases. Provider results are fetched once per gem per resolution; identical
records are deduplicated and conflicting identities are rejected.

`ResolveError` separates conflicts from provider failures and invalid metadata.
`InMemoryIndex` can resolve from a fixed lockfile candidate set; it never fetches
missing packages. Source selection belongs to the provider.

Run `cargo test -p rb-resolver`. All tests are offline. Run the small example with
`cargo run -p rb-resolver --example resolve`.

The [frozen corpus](tests/fixtures/corpus/README.md) covers eleven exported
Gemfile projections with Bundler reference locks and a small captured index.
It checks locked results, fresh-solution validity and real conflict witnesses
without registry access. The adapters are test-only.
