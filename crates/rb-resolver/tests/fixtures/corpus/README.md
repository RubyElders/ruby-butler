# Frozen Gemfile corpus

Captured on 2026-10-07 from the public
[gemfile-corpus](https://github.com/RubyElders/gemfile-corpus). `cases.json` records
each repository, immutable commit, corpus path and original Gemfile SHA-256.
The original files' hashes were checked during capture. Original Gemfiles remain
subject to their upstream licenses; these fixtures contain exported dependency
metadata, not application code.

Eleven successful cases cover Hanami 1.x, a Hanami router prerelease, Roda,
Sinatra/Sequel, Rails 7, Grape/Padrino, Jekyll, async networking, native/ML gems,
WhatWeb and Rails tooling. Each `rbproject.toml` is the actual `rb-gemfile` export.
Only enabled default-group registry roots without platform qualifiers enter the
request. Original Ruby declarations, other groups, activation settings and
platform-qualified roots are outside this projection. These are dependency
resolution cases, not claims that the original applications install or run.

## Reference resolution

The exports were projected into simple Gemfiles with the same selected names
and requirement strings. Bundler 2.6.9, Ruby 3.4.10 and RubyGems 3.6.9 resolved
these against RubyGems.org using `bundle lock --add-platform x86_64-linux-gnu`.
No application gems were installed. `Gemfile.lock` preserves each reference.
`expected.json` records the target closure obtained with
`Bundler::SpecSet.new(parser.specs).for(parser.dependencies.values, [target])`,
where `parser` is `Bundler::LockfileParser` and `target` is the recorded platform.

Bundler treats its running version as an implicit constraint and omits that
package from the target closure. Fixture requests expose `bundler = 2.6.9`
explicitly and reference sets include that package. This is oracle context,
not a special rule in the production resolver.

## Bounded index

`index/<gem>.info` retains the original header and complete records needed by
the union of the reference lockfiles, plus Bundler 2.6.9 and the old Padrino
failure witness. Records came from `https://rubygems.org/info/<gem>` with
`Accept-Encoding: identity`; supplied SHA-256 representation digests were
verified before reduction. No record values or archive checksums were changed.
There are 263 records for 193 gems, approximately 44 KB. This is a bounded index,
not a copy of the full registry. No archives or credentials are included.

Locked tests reproduce the target reference exactly. Fresh tests resolve over
this bounded union, validate every root, dependency, Ruby and platform constraint,
reject orphan packages, and repeat with reversed provider order. Fresh solutions
need not equal a full-registry Bundler resolution or prove global optimality.
All corpus tests use local files and make no HTTP requests or Ruby subprocesses.

## Conflicts

Combining the actual locked Hanami and Sinatra framework versions produces a
Rack conflict over the captured index.

`padrino-conflict` preserves another source export and Bundler's failure:
Padrino 0.11.4 requires padrino-gen 0.11.4, which requires Bundler `~> 1.0`.
The captured metadata verifies that chain. Its minimized resolver witness
intersects that requirement with the oracle's `= 2.6.9` constraint. The full
failed project's candidate graph was not captured or claimed to be resolved.

Regeneration is opt-in: retrieve the source files at the recorded commits,
export with `rb-gemfile`, repeat the selected-root Bundler locks with the recorded
Ruby/tool versions, extract the target closures, and retain the corresponding
compact-index records. Inspect changed declarations and tool semantics before
replacing expected results. CI never updates fixtures from live services.
