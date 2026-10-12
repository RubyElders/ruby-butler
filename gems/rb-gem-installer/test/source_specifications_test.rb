require 'minitest/autorun'
require 'tmpdir'
require 'fileutils'
require_relative '../lib/rb_gem_installer'

class SourceSpecificationsTest < Minitest::Test
  def test_reads_gemspec_with_relative_project_helpers
    Dir.mktmpdir do |directory|
      fixtures = File.expand_path('../../../crates/rb-packages/tests/fixtures/sources', __dir__)
      FileUtils.cp_r(File.join(fixtures, '.'), directory)
      records = RubyButler::GemInstaller::SourceSpecifications.read(directory, 'fixture.gemspec')
      assert_equal 1, records.size
      assert_equal 'fixture.gemspec', records.first.fetch(:gemspec)
      specification = Gem::Specification.from_yaml(records.first.fetch(:metadata))
      assert_equal 'fixture', specification.name
      assert_equal '1.0.0', specification.version.to_s
      assert_includes records.first.fetch(:specification), 'Gem::Specification.new'
    end
  end

  def test_rejects_gemspec_glob_outside_the_source
    Dir.mktmpdir do |directory|
      FileUtils.mkdir_p(File.join(directory, 'source'))
      File.write(File.join(directory, 'outside.gemspec'), "raise 'must not execute'\n")
      error = assert_raises(RuntimeError) do
        RubyButler::GemInstaller::SourceSpecifications.read(File.join(directory, 'source'), '../outside.gemspec')
      end
      assert_equal 'Gemspec escapes source directory', error.message
    end
  end
end
