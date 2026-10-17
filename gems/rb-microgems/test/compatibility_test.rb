require_relative 'test_helper'

class CompatibilityTest < Minitest::Test
  include RubyProcess

  def setup
    @root = Dir.mktmpdir('microgems-test-')
    FileUtils.mkdir_p(File.join(@root, 'lib'))
    File.write(File.join(@root, 'lib', 'fixture_api.rb'), "FIXTURE_API = 42\n")
    @manifest = File.join(@root, 'manifest.json')
    gem = { name: 'fixture', version: '2.1.3', platform: 'ruby', root: @root,
            require_paths: ['lib'], metadata: { purpose: 'test' }, executables: [], bindir: 'bin' }
    File.write(@manifest, JSON.generate(gems: [gem], arguments: ['--with-extra=a b']))
    @environment = { 'RB_MICROGEMS_MANIFEST' => @manifest, 'FIXTURE_ROOT' => @root }
  end

  def teardown
    FileUtils.remove_entry(@root)
  end

  def test_build_api_contracts
    output, error, status = ruby_fixture('contracts.rb', environment: @environment, compatibility: true)
    assert status.success?, error
    assert_includes output, 'contracts passed'
  end

  def test_gem_directory_matches_rubygems_with_default_and_custom_homes
    [nil, @root, File.join(@root, '.'), File.join(@root, 'missing'), 'relative-gem-home'].each do |home|
      environment = { 'GEM_HOME' => home }
      expected, error, status = Open3.capture3(environment, RbConfig.ruby, '-rrubygems', '-e', 'puts Gem.dir')
      assert status.success?, error
      output, error, status = ruby_fixture('gem_directory.rb', environment: environment, compatibility: true)
      assert status.success?, error
      assert_equal expected, output
    end
  end

  def test_core_does_not_install_the_compatibility_namespace
    output, error, status = ruby_fixture('namespaces.rb', environment: @environment, compatibility: true)
    assert status.success?, error
    assert_equal "namespaces passed\n", output
  end

  def test_each_shim_can_be_the_first_import
    MicroGems::FEATURES.each do |feature|
      output, error, status = ruby_fixture('shims.rb', feature, compatibility: true)
      assert status.success?, "#{feature}: #{error}"
      assert_equal "shims passed\n", output
    end
  end

  def test_loaded_rubygems_is_preserved
    output, error, status = ruby_fixture('existing_rubygems.rb', environment: { 'MICROGEMS_PATH' => RubyProcess::LIB })
    assert status.success?, error
    assert_equal "existing RubyGems preserved\n", output
  end
end
