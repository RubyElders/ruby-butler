require 'microgems'
require 'microgems/package'
require 'microgems/ext/builder'
require 'microgems/command'

raise 'Gem alias installed by the core' if defined?(Gem)
raise 'Kernel hook installed by the core' if Kernel.ancestors.include?(MicroGems::Requires)

%w[Version Requirement Platform Specification Command Package Ext::Builder LoadError MissingSpecError].each do |name|
  raise "Wrong implementation namespace: #{name}" unless MicroGems.const_get(name).name == "MicroGems::#{name}"
end

MicroGems.configure(JSON.parse(File.read(ENV.fetch('RB_MICROGEMS_MANIFEST'))))
Kernel.prepend(MicroGems::Requires)
require 'fixture_api'
raise 'Core activation failed' unless FIXTURE_API == 42 && MicroGems.loaded_specs.fetch('fixture').version.to_s == '2.1.3'
raise 'Core require installed Gem' if defined?(Gem)

require 'microgems/compatibility'
raise 'Compatibility namespace is not an alias' unless Gem.equal?(MicroGems)
raise 'Specification registries differ' unless Gem.loaded_specs.equal?(MicroGems.loaded_specs)
raise 'Exception classes differ' unless Gem::LoadError.equal?(MicroGems::LoadError)
raise 'Version classes differ' unless Gem::Version.equal?(MicroGems::Version)

puts 'namespaces passed'
