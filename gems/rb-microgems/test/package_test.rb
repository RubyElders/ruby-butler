require_relative 'test_helper'
require 'rubygems/package'

class PackageTest < Minitest::Test
  def test_built_gem_contains_the_runtime_library
    root = File.expand_path('..', __dir__)
    spec = Gem::Specification.load(File.join(root, 'rb-microgems.gemspec'))
    assert_equal MicroGems::VERSION, spec.version.to_s
    assert_empty spec.runtime_dependencies
    Dir.mktmpdir('microgems-package-') do |directory|
      archive = File.join(directory, 'rb-microgems.gem')
      capture_io { Dir.chdir(root) { Gem::Package.build(spec, false, true, archive) } }
      destination = File.join(directory, 'extracted')
      Gem::Package.new(archive).extract_files(destination)
      files = Dir.chdir(root) { Dir['lib/**/*.rb'].sort }
      assert_equal files, Dir.chdir(destination) { Dir['lib/**/*.rb'].sort }
      files.each { |path|
        assert_equal File.binread(File.join(root, path)), File.binread(File.join(destination, path)), path
      }
    end
  end
end
