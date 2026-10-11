spec = Gem.loaded_specs.fetch('dinner')
raise 'Wrong runtime gem root' unless spec.full_gem_path == ENV.fetch('EXPECTED_GEM_ROOT')
raise 'Wrong runtime require paths' unless spec.require_paths == ['lib']
raise 'Bundler was loaded' if defined?(Bundler)
raise 'Full RubyGems was loaded' if $LOADED_FEATURES.any? { |path| path.include?('rubygems/core_ext') }
require 'butler_native'
puts ButlerNative.answer
