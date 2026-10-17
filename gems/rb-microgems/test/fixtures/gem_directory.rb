require 'microgems/compatibility'

raise 'Full RubyGems loaded' if $LOADED_FEATURES.any? { |file| file.include?('rubygems/core_ext') }

puts Gem.dir
