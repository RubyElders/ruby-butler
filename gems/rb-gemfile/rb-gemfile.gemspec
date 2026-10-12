require_relative 'lib/rb_gemfile/version'

Gem::Specification.new do |spec|
  spec.name = 'rb-gemfile'
  spec.version = RubyButler::Gemfile::VERSION
  spec.authors = ['RubyElders']
  spec.summary = 'Gemfile evaluation without Bundler'
  spec.homepage = 'https://github.com/RubyElders/ruby-butler'
  spec.licenses = ['MIT', 'Apache-2.0']
  spec.required_ruby_version = '>= 3.3'
  spec.files = Dir.chdir(__dir__) { Dir['lib/**/*.rb', 'exe/*', 'README.md'] }
  spec.require_paths = ['lib']
  spec.bindir = 'exe'
  spec.executables = ['rb-evaluate-gemfile']
  spec.add_development_dependency 'minitest', '>= 5.20', '< 7'
  spec.add_development_dependency 'rake', '>= 13', '< 14'
end
