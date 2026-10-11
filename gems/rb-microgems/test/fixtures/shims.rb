require ARGV.fetch(0)
raise 'Gem is not the compatibility alias' unless Gem.equal?(MicroGems)
raise 'Version is not owned by MicroGems' unless Gem::Version.name == 'MicroGems::Version'

MicroGems::FEATURES.each { |feature| require feature }
raise 'Package is not owned by MicroGems' unless Gem::Package.name == 'MicroGems::Package'
raise 'Builder is not owned by MicroGems' unless Gem::Ext::Builder.name == 'MicroGems::Ext::Builder'
raise 'Full RubyGems loaded' if $LOADED_FEATURES.any? { |file| file.include?('rubygems/core_ext') }

puts 'shims passed'
