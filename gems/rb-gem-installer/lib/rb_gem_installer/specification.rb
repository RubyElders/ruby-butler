module RubyButler
  module GemInstaller
    module Specification
      def self.write(record, path)
        spec = Gem::Specification.new do |gem|
          gem.name = record.fetch('name')
          gem.version = record.fetch('version')
          gem.platform = record.fetch('platform')
          gem.required_ruby_version = record.fetch('ruby').split(', ')
          gem.required_rubygems_version = record.fetch('rubygems').split(', ')
          gem.require_paths = record.fetch('require_paths')
          gem.executables = record.fetch('executables')
          gem.bindir = record.fetch('bindir')
          gem.extensions = record.fetch('extensions')
          gem.files = record.fetch('files')
          gem.metadata = record.fetch('metadata')
          record.fetch('dependencies').each { |name, requirement| gem.add_runtime_dependency(name, *requirement.split(', ')) }
        end
        File.write(path, spec.to_ruby)
      end
    end
  end
end
