require 'minitest/autorun'
require 'tmpdir'
require 'rubygems/package'
require_relative '../lib/rb_gem_installer'

class PackageTest < Minitest::Test
  def test_built_gem_contains_the_configuration_helper
    root = File.expand_path('..', __dir__)
    spec = Gem::Specification.load(File.join(root, 'rb-gem-installer.gemspec'))
    assert_equal RubyButler::GemInstaller::VERSION, spec.version.to_s
    assert_empty spec.runtime_dependencies
    assert_equal ['rb-ruby-config'], spec.executables
    Dir.mktmpdir('installer-package-') do |directory|
      archive = File.join(directory, 'rb-gem-installer.gem')
      capture_io { Dir.chdir(root) { Gem::Package.build(spec, false, true, archive) } }
      destination = File.join(directory, 'extracted')
      Gem::Package.new(archive).extract_files(destination)
      files = Dir.chdir(root) { Dir['lib/**/*.rb', 'exe/*'].sort }
      assert_equal files, Dir.chdir(destination) { Dir['lib/**/*.rb', 'exe/*'].sort }
      files.each { |path|
        assert_equal File.binread(File.join(root, path)), File.binread(File.join(destination, path)), path
      }
    end
  end
end
