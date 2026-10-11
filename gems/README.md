# Ruby gems

`rb-microgems` provides the native build compatibility layer. `rb-gem-installer` provides the selected Ruby's configuration helper. Rust embeds their runtime files into Butler.

Install test tools with `gem install rake minitest`. From this directory, run `rake test` for both suites. Install lint tools with `bundle install`, then run `bundle exec rake lint`; `Gemfile.lock` pins the tooling versions. The same tasks are available inside each gem. Lint checks Ruby 3.3 syntax, correctness, layout and security rules. CI runs lint separately from the Ruby test matrix.
