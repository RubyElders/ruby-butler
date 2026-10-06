# rb-gem-types

Standalone Ruby gem versions, requirements, platforms and package identities.
No Ruby process, registry access, resolver or installer is needed at runtime.

```rust
use rb_gem_types::{Platform, Requirement, Version};

let version: Version = "1.2.3".parse()?;
let requirement: Requirement = "~> 1.2, != 1.3".parse()?;
assert!(requirement.matches(&version));
assert!(Platform::parse("ruby").matches(&Platform::parse("x86_64-linux-gnu")));
# Ok::<(), rb_gem_types::ParseError>(())
```

Versions follow RubyGems ordering, including prereleases and arbitrary-length
numeric segments. Equal versions have equal hashes even if their precision
differs. Requirements support `=`, `!=`, `<`, `<=`, `>`, `>=` and `~>`;
comma or compact-index `&` joins constraints. Prerelease eligibility remains
resolver policy, separate from matching a requirement.

Platform matching is directional: the receiver describes the gem, the argument
the target Ruby platform. Pure Ruby gems match every target. `score` is a local
selection policy: lower is preferred, exact matches first and pure Ruby last;
nonmatching platforms return `None`. `PackageId` names an archive without
including its registry or Git source.

Run `cargo test -p rb-gem-types`. Frozen RubyGems oracle fixtures run without
Ruby. Platform parsing and matching follow the frozen RubyGems 4.0.20 contract;
older releases differ on some platform strings. The live version/requirement
oracle uses installed RubyGems. Set `RB_TEST_RUBY` to select another Ruby.
