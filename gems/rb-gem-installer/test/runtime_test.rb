require 'minitest/autorun'
require_relative '../lib/rb_gem_installer'

class RuntimeTest < Minitest::Test
  def test_reports_selected_ruby_and_available_compatibility_runtime
    runtime = RubyButler::GemInstaller::Runtime.read
    assert_equal RbConfig.ruby, runtime.fetch(:executable)
    assert_equal RUBY_VERSION, runtime.fetch(:version)
    assert_equal RUBY_ENGINE, runtime.fetch(:engine)
    assert_equal RUBY_ENGINE_VERSION, runtime.fetch(:engine_version)
    assert_equal RUBY_PATCHLEVEL, runtime.fetch(:patchlevel)
    assert_equal Gem::Platform.local.to_s, runtime.fetch(:platform)
    assert_equal Gem.extension_api_version, runtime.fetch(:abi)
    assert_equal Gem::VERSION, runtime.fetch(:rubygems)
    assert_equal Gem::Specification.find_all_by_name('bundler').map(&:version).max&.to_s, runtime.fetch(:bundler)
  end
end
