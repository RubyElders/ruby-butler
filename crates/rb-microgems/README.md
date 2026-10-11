# rb-microgems

Minimal RubyGems compatibility for isolated build processes running with `--disable=gems`. The caller provides exact installed gem identities and directories; this crate does not resolve, download or install gems.

`Environment::new` creates a temporary Ruby load path and JSON manifest. Preload `microgems/compatibility` and set `RB_MICROGEMS_MANIFEST` to its manifest path. The environment must remain alive while its subprocesses run.

The implementation lives in `MicroGems` under `microgems/`. The `microgems/compatibility` entry point aliases `Gem` to `MicroGems` and installs the `gem` and `require` hooks. Files under `rubygems/` only forward compatibility imports to the implementation. Loading `microgems` directly leaves the `Gem` namespace and Kernel hooks untouched.

The supported surface includes versions, requirements, platforms, explicit gem activation, specification lookup, tar reading and build helpers. Registered dependencies can activate through `gem` or `require`. Remote fetching, dependency resolution and gem installation are outside this compatibility layer. Unsupported RubyGems feature imports fail explicitly. Bundler and full RubyGems are not loaded.

The Ruby gem lives in `gems/rb-microgems`. Its `lib/` files are embedded in the Rust crate; no RubyGems installation is needed to materialize them. Run its independent Ruby tests with `ruby gems/rb-microgems/test/all_test.rb` from the repository root.
