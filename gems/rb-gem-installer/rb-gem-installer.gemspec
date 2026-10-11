require_relative 'lib/rb_gem_installer'

Gem::Specification.new do |spec|
  spec.name = 'rb-gem-installer'
  spec.version = RubyButler::GemInstaller::VERSION
  spec.authors = ['RubyElders']
  spec.summary = 'Ruby configuration helper for native gem builds'
  spec.homepage = 'https://github.com/RubyElders/ruby-butler'
  spec.licenses = ['MIT', 'Apache-2.0']
  spec.required_ruby_version = '>= 3.3'
  spec.files = Dir.chdir(__dir__) { Dir['lib/**/*.rb', 'exe/*', 'README.md'] }
  spec.require_paths = ['lib']
  spec.bindir = 'exe'
  spec.executables = ['rb-ruby-config']
  spec.add_development_dependency 'minitest', '>= 5.20', '< 7'
  spec.add_development_dependency 'rake', '>= 13', '< 14'
end
