original = Gem
version = Gem::Version
$LOAD_PATH.unshift(ENV.fetch('MICROGEMS_PATH'))
begin
  require 'microgems/compatibility'
  raise 'Full RubyGems was silently replaced'
rescue LoadError => error
  raise unless error.message.include?('RubyGems to be disabled')
end
raise 'Gem namespace changed' unless Gem.equal?(original)
raise 'RubyGems version class changed' unless Gem::Version.equal?(version)
raise 'Implementation was defined under Gem' if MicroGems::Version.equal?(version)

puts 'existing RubyGems preserved'
