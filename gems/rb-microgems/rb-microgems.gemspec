require_relative 'lib/microgems'

Gem::Specification.new do |spec|
  spec.name = 'rb-microgems'
  spec.version = MicroGems::VERSION
  spec.authors = ['RubyElders']
  spec.summary = 'Minimal RubyGems compatibility for isolated native builds'
  spec.homepage = 'https://github.com/RubyElders/ruby-butler'
  spec.licenses = ['MIT', 'Apache-2.0']
  spec.required_ruby_version = '>= 3.3'
  spec.files = Dir.chdir(__dir__) { Dir['lib/**/*.rb', 'README.md'] }
  spec.require_paths = ['lib']
  spec.add_development_dependency 'minitest', '>= 5.20', '< 7'
  spec.add_development_dependency 'rake', '>= 13', '< 14'
end
