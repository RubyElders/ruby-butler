require 'minitest/autorun'
require 'tmpdir'
require_relative '../lib/rb_gem_installer'

class SpecificationTest < Minitest::Test
  def test_installed_specification_preserves_runtime_contract
    record = { 'name' => 'fixture', 'version' => '1.2.3', 'platform' => 'ruby',
               'ruby' => '>= 3.3', 'rubygems' => '>= 0', 'require_paths' => ['lib'],
               'executables' => ['fixture'], 'bindir' => 'bin', 'extensions' => ['ext/extconf.rb'],
               'files' => ['lib/fixture.rb'], 'metadata' => { 'purpose' => 'test' },
               'dependencies' => { 'rake' => '>= 13, < 14' } }
    Dir.mktmpdir do |directory|
      path = File.join(directory, 'fixture.gemspec')
      RubyButler::GemInstaller::Specification.write(record, path)
      specification = Gem::Specification.load(path)
      assert_equal 'fixture', specification.name
      assert_equal '1.2.3', specification.version.to_s
      assert_equal ['lib'], specification.raw_require_paths
      assert_equal ['ext/extconf.rb'], specification.extensions
      assert_equal ['fixture'], specification.executables
      assert_equal 'bin', specification.bindir
      assert_equal({ 'purpose' => 'test' }, specification.metadata)
      assert_equal Gem::Requirement.new('>= 13', '< 14'), specification.dependencies.first.requirement
    end
  end
end
