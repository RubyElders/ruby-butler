require 'minitest/autorun'
require 'tmpdir'
require 'rubygems/package'
require 'open3'
require 'json'
require 'rbconfig'
require_relative '../lib/rb_gemfile/version'

class PackageTest < Minitest::Test
  def test_packaged_evaluator_runs_without_repository_files
    root = File.expand_path('..', __dir__)
    spec = Gem::Specification.load(File.join(root, 'rb-gemfile.gemspec'))
    assert_equal RubyButler::Gemfile::VERSION, spec.version.to_s
    assert_empty spec.runtime_dependencies
    assert_equal ['rb-evaluate-gemfile'], spec.executables
    Dir.mktmpdir do |directory|
      archive = File.join(directory, 'rb-gemfile.gem')
      capture_io { Dir.chdir(root) { Gem::Package.build(spec, false, true, archive) } }
      destination = File.join(directory, 'extracted')
      Gem::Package.new(archive).extract_files(destination)
      files = Dir.chdir(root) { Dir['lib/**/*.rb', 'exe/*'].sort }
      assert_equal files, Dir.chdir(destination) { Dir['lib/**/*.rb', 'exe/*'].sort }
      files.each do |path|
        assert_equal File.binread(File.join(root, path)), File.binread(File.join(destination, path)), path
      end
      gemfile = File.join(directory, 'Gemfile')
      result = File.join(directory, 'manifest.json')
      File.write(gemfile, "gem 'rake', '~> 13.0'\n")
      stdout, stderr, status = Open3.capture3(RbConfig.ruby, File.join(destination, 'exe/rb-evaluate-gemfile'), gemfile, result)
      assert status.success?, stderr
      assert_empty stdout
      assert_equal ['~> 13.0'], JSON.parse(File.read(result)).fetch('manifest').fetch('dependencies').fetch('rake').fetch('version')
    end
  end
end
