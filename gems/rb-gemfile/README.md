# rb-gemfile

The Ruby evaluator embedded by the Rust `rb-gemfile` crate. Executes trusted Gemfiles and returns dependency declarations as JSON without loading Bundler. Allows one Git selector (`branch`, `tag` or `ref`/`rev`) per dependency.

Run `ruby test/all_test.rb` or `rake test`. Build with `gem build rb-gemfile.gemspec`.
