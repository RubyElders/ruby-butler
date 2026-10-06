# rb-gemfile

Evaluate trusted Gemfiles into dependency manifests without loading Bundler. An embedded Ruby evaluator collects declarations; Rust provides typed data, TOML/KDL export and optional caching. Ruby and RubyGems are still required. This crate does not resolve versions, install gems or generate lockfiles.

```rust,no_run
use rb_gemfile::{Evaluator, write_project};

let evaluation = Evaluator::new("ruby").evaluate("Gemfile")?;
let project = evaluation.manifest.merge_project("[project]\nname = 'example'\n")?;
write_project(".rb/rbproject.toml", &project)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

| API | Purpose |
| --- | --- |
| `Evaluator::evaluate` | Run a Gemfile and return its manifest, input paths and captured output. |
| `Manifest::to_toml` / `to_kdl` | Export active declarations with explicit source and group blocks. |
| `Manifest::merge_project` / `merge_project_format` | Merge with TOML project settings, rejecting explicit conflicts. |
| `Manifest::from_kdl` | Read KDL declarations; TOML uses Serde deserialization. |
| `SnapshotCache::evaluate` | Reuse deterministic evaluations when fingerprinted inputs are unchanged. |
| `write_project` | Write a generated file atomically. |

Supports groups, platforms, conditions, require settings, registry/Git/path sources, included Gemfiles and local gemspecs. Repeated declarations remain separate in `Dependency::variants`; exports omit inactive declarations. Bundler compatibility helpers are limited: plugins are recorded, not installed or executed, and source `cooldown` is accepted but not enforced.

Gemfiles execute arbitrary Ruby: evaluate only trusted inputs. For caching, declare external files/directories with `input` and unread generated files with `output`. The project directory is fingerprinted automatically. Use uncached evaluation for time, network, subprocess results or side effects.

The CLI exposes this through `rb export-gemfile`, with `--format kdl` for KDL output. Exported dependencies are not yet consumed by `rb-core`.

Tested against 1,477 snapshots from the public [Gemfile corpus](https://github.com/RubyElders/gemfile-corpus), using Ruby 3.4.1 and Bundler 2.6.2 as a comparison: 1,318 evaluated and exported to TOML successfully; 159 failed, of which 157 also failed Bundler. Most failures involved missing companion files. Two Git-option combinations accepted by Bundler remain unsupported. This checks evaluation/export, not installation or complete manifest equivalence.

Run `cargo test -p rb-gemfile`. Direct Ruby tests use `ruby crates/rb-gemfile/tests/ruby/evaluator_test.rb` with Minitest 5. `RB_GEMFILE_RUBY` selects Ruby for Rust tests and the `evaluate` example.
