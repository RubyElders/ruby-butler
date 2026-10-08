# rb-gem-archive

Reads gem identity, runtime dependencies and gemspec metadata from `.gem` archives without Ruby or RubyGems. It verifies declared SHA-1, SHA-256 and SHA-512 member checksums when present and bounds compressed archive and decoded metadata sizes. Older archives without a checksum member remain readable.

```rust,no_run
use rb_gem_archive::read;
use std::fs::File;

let metadata = read(File::open("rake-13.2.1.gem")?)?;
println!("{}", metadata.id);
# Ok::<(), anyhow::Error>(())
```

This slice reads declared metadata. It streams the compressed data member for hashing, but does not inspect or extract its payload, validate install paths, compile extensions or activate gems. Ruby YAML object tags are treated as data. Bounded signature members are accepted, but certificate trust and signature authenticity are not verified. Installation and payload validation belong to a later phase.
