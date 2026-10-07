# Lockfile fixtures

The eleven framework locks were captured by the `rb-resolver` corpus on 2026-10-07 using Bundler 2.6.9, Ruby 3.4.10 and RubyGems 3.6.9. Their source Gemfile commits and hashes are recorded in `../../../rb-resolver/tests/fixtures/corpus/cases.json`; that corpus README explains the default-group projections and limitations. These copies keep this crate's tests self-contained. Regenerate them together with the resolver fixtures.

`mixed.lock` is a synthetic fixture covering the source, platform, checksum and Ruby environment records absent from those registry-only captures. No private source is contacted. Fixture dependencies are metadata, not installed code.

`../bundler_oracle.rb` compares the parsed semantics of originals and rendered files with Bundler. Run it manually with a Ruby containing Bundler 2.6.9; the ordinary Rust suite does not need Ruby or network access.

The three small compatibility fixtures reproduce patterns from the collected lockfile corpus: repeated polyglot declarations in Epictetus/padrino-mongoid-delayed_job (2021), an implicit Bundler checksum in basecamp/fizzy (2026), and remote-less registry sections in denny/ShinyCMS-ruby (2021) and lewagon/setup (2021). They are minimized records rather than full project locks.
