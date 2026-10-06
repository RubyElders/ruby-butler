# rb-compact-index

A standalone blocking client for RubyGems metadata and gem archives. It handles HTTP, caching and checksums; version comparison, platform selection, resolution and installation belong to the caller.

```rust,no_run
use rb_compact_index::Client;
use std::path::Path;

let client = Client::new("https://rubygems.org", Path::new("gem-cache"), false)?;
for gem in client.info("rack")? {
    println!("{}: {:?}", gem.version, gem.dependencies);
}
# Ok::<(), anyhow::Error>(())
```

| API | Purpose |
| --- | --- |
| `Client::info` | Fetch a gem's versions, dependency requirements and metadata. |
| `Client::download` | Fetch an archive with an expected SHA-256 checksum. |
| `parse` | Parse a compact-index response without HTTP or cache access. |

Metadata is cached per source and revalidated with ETags. Archives are shared by checksum and verified before reuse. File locks and atomic writes support concurrent callers. Offline mode reads cached metadata and archives without contacting the registry.

`GemInfo` keeps version strings, including platform suffixes, and metadata fields. Compact-index records retain server order. Repeated dependency requirements are combined with commas.

HTTP(S) base URLs cannot contain credentials, query strings or fragments. SHA-256 `Repr-Digest` values are checked when present. Servers without compact-index endpoints fall back to `specs.4.8.gz` and optional prerelease metadata, parsed in Rust; this downloads matching archives to obtain their metadata. No source substitution occurs. Authentication, `/names`, `/versions` parsing and incremental Range requests are not implemented.

Run `cargo test -p rb-compact-index`. HTTP tests use loopback fixture servers and require no public registry or Ruby installation.

For an optional live check of metadata, archive verification and offline reuse:

```sh
cargo run -p rb-compact-index --example check_source -- https://gem.coop/@kaspth oaken 1.0.0
```
