# rb-microgems

Minimal gem APIs under `MicroGems`. Load `microgems` for the core or preload `microgems/compatibility` with RubyGems disabled for `Gem`, `gem` and `require` compatibility. The caller supplies installed dependencies through `RB_MICROGEMS_MANIFEST`.

Run `ruby test/all_test.rb` or `rake test`. Minitest and rake are development dependencies only. Build with `gem build rb-microgems.gemspec`; native builds consume the embedded library files directly and do not install this gem.
