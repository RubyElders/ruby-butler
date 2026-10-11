require 'minitest/autorun'
require 'open3'
require 'json'
require_relative '../lib/rb_gem_installer'

class RubyConfigTest < Minitest::Test
  def test_selected_interpreter_configuration
    result = RubyButler::GemInstaller::RubyConfig.read
    assert_equal RbConfig.ruby, result.fetch(:executable)
    assert_equal RUBY_VERSION, result.fetch(:version)
    assert_equal RbConfig::CONFIG, result.fetch(:config)
    assert_equal Shellwords.split(RbConfig::CONFIG.fetch('CC', 'cc')), result.fetch(:linker)
    assert_equal Shellwords.split(RbConfig::CONFIG.fetch('LIBRUBYARG_SHARED', '') + ' ' + RbConfig::CONFIG.fetch('LIBS', '')),
                 result.fetch(:libraries)
  end

  def test_quoted_toolchain_arguments
    config = { 'CC' => '"/compiler with spaces/cc" -target example',
               'LIBRUBYARG_SHARED' => '-lruby -L"/ruby libraries"', 'LIBS' => '-lm -pthread' }
    result = RubyButler::GemInstaller::RubyConfig.read(config: config, executable: '/ruby with spaces/ruby',
                                                       version: '3.4.0')
    assert_equal ['/compiler with spaces/cc', '-target', 'example'], result.fetch(:linker)
    assert_equal ['-lruby', '-L/ruby libraries', '-lm', '-pthread'], result.fetch(:libraries)
    assert_equal '/ruby with spaces/ruby', result.fetch(:executable)
    assert_equal '3.4.0', result.fetch(:version)
    assert_equal config, result.fetch(:config)
  end

  def test_optional_toolchain_fields
    result = RubyButler::GemInstaller::RubyConfig.read(config: {})
    assert_equal ['cc'], result.fetch(:linker)
    assert_empty result.fetch(:libraries)
  end

  def test_cli_emits_only_configuration_json_without_rubygems
    root = File.expand_path('..', __dir__)
    output, error, status = Open3.capture3({ 'RUBYOPT' => nil, 'RUBYLIB' => nil }, RbConfig.ruby,
                                           '--disable=gems', '-I', File.join(root, 'lib'), File.join(root, 'exe/rb-ruby-config'))
    assert status.success?, error
    assert_empty error
    expected = JSON.parse(JSON.generate(RubyButler::GemInstaller::RubyConfig.read))
    assert_equal expected, JSON.parse(output)
  end

  def test_import_has_no_output_and_needs_no_rubygems
    lib = File.expand_path('../lib', __dir__)
    program = 'require "rb_gem_installer"; raise if defined?(Gem) || defined?(Bundler)'
    output, error, status = Open3.capture3(RbConfig.ruby, '--disable=gems', '-I', lib, '-e', program)
    assert status.success?, error
    assert_empty output
    assert_empty error
  end
end
