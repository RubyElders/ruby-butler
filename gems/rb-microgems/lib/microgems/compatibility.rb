require_relative '../microgems'

raise ::LoadError, 'MicroGems requires RubyGems to be disabled' if defined?(Gem) && !Gem.equal?(MicroGems)

Gem = MicroGems unless defined?(Gem)

module Kernel
  def gem(name, *requirements) = MicroGems.activate(name, *requirements)
  private :gem
  prepend MicroGems::Requires
end

if ENV['RB_MICROGEMS_MANIFEST']
  MicroGems.configure(JSON.parse(File.read(ENV.fetch('RB_MICROGEMS_MANIFEST'))))
end

if ENV['RB_MICROGEMS_LOAD_PATHS']
  JSON.parse(ENV.fetch('RB_MICROGEMS_LOAD_PATHS')).reverse_each do |path|
    $LOAD_PATH.unshift(path) unless $LOAD_PATH.include?(path)
  end
end

if ENV['RB_MICROGEMS_CURRENT']
  name, version = JSON.parse(ENV.fetch('RB_MICROGEMS_CURRENT'))
  MicroGems.activate(name, "= #{version}")
end
