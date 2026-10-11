# rb-gem-installer

The Ruby helper for the Rust installer. `RubyButler::GemInstaller::RubyConfig.read` returns the selected interpreter's build configuration. `ruby --disable=gems -Ilib exe/rb-ruby-config` emits it as JSON without RubyGems or Bundler.

Run `ruby test/all_test.rb` or `rake test`. Minitest and rake are development dependencies only. Build with `gem build rb-gem-installer.gemspec`; the Rust installer embeds and materializes these files directly.
