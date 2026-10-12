require_relative 'lib/fixture/version'

Gem::Specification.new do |spec|
  spec.name = 'fixture'
  spec.version = Fixture::VERSION
  spec.summary = 'Source installation fixture'
  spec.authors = ['Fixture']
  spec.files = Dir.glob('{lib,ext}/**/*').select { |path| File.file?(path) }
  spec.require_paths = ['lib']
  spec.extensions = Dir.glob('ext/**/extconf.rb')
end
