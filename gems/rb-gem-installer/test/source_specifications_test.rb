require 'minitest/autorun'
require 'tmpdir'
require 'fileutils'
require 'json'
require 'open3'
require_relative '../lib/rb_gem_installer'

class SourceSpecificationsTest < Minitest::Test
  def test_exports_gemspec_and_loaded_helper_inputs
    Dir.mktmpdir do |directory|
      fixtures = File.expand_path('../../../crates/rb-packages/tests/fixtures/sources', __dir__)
      FileUtils.cp_r(File.join(fixtures, '.'), directory)
      request = File.join(directory, 'request.json')
      output = File.join(directory, 'specifications.json')
      File.write(request, JSON.generate(directory: directory, glob: 'fixture.gemspec', output: output))
      tracker = File.expand_path('../../rb-gemfile/lib/rb_gemfile/file_inputs.rb', __dir__)
      helper = File.expand_path('../lib/rb_gem_installer/source_specifications.rb', __dir__)
      _stdout, stderr, status = Open3.capture3(
        RbConfig.ruby, '-r', tracker, '-r', helper, '-e',
        'RubyButler::GemInstaller::SourceSpecifications.export(ARGV.fetch(0), RubyButler::FileInputs.new)', request
      )
      assert status.success?, stderr
      specifications = JSON.parse(File.read(output))
      expected = ['fixture.gemspec', 'lib/fixture/version.rb'].map { |path| File.realpath(File.join(directory, path)) }.sort
      assert_equal expected, specifications.fetch(0).fetch('inputs').sort
    end
  end

  def test_exports_environment_names_read_by_the_gemspec
    Dir.mktmpdir do |directory|
      File.write(File.join(directory, 'fixture.gemspec'), "ENV['RAILS_ENV']\nGem::Specification.new { |s| s.name='fixture'; s.version='1.0'; s.summary='Fixture'; s.authors=['Fixture'] }\n")
      request = File.join(directory, 'request.json')
      output = File.join(directory, 'specifications.json')
      File.write(request, JSON.generate(directory: directory, glob: 'fixture.gemspec', output: output))
      helpers = File.expand_path('../../rb-gemfile/lib/rb_gemfile', __dir__)
      helper = File.expand_path('../lib/rb_gem_installer/source_specifications.rb', __dir__)
      _stdout, stderr, status = Open3.capture3(
        RbConfig.ruby, '-r', File.join(helpers, 'file_inputs.rb'), '-r', File.join(helpers, 'environment_inputs.rb'), '-r', helper, '-e',
        'RubyButler::GemInstaller::SourceSpecifications.export(ARGV.fetch(0), RubyButler::FileInputs.new, RubyButler::EnvironmentInputs.new)', request
      )
      assert status.success?, stderr
      assert_equal ['RAILS_ENV'], JSON.parse(File.read(output)).fetch(0).fetch('environment')
    end
  end

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
