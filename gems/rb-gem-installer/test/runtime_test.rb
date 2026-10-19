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

  def test_build_tools_belong_to_the_selected_ruby
    tools = RubyButler::GemInstaller::Runtime.read.fetch(:build_tools)
    expected = Gem::Specification.find_all_by_name('rake')
                                 .select { |spec| spec.full_gem_path.start_with?(File.join(Gem.default_dir, 'gems') + File::SEPARATOR) }
                                 .max_by(&:version)
    assert_equal expected ? 1 : 0, tools.length
    return unless expected

    tool = tools.fetch(0)
    assert_equal 'rake', tool.fetch(:name)
    assert_equal expected.version.to_s, tool.fetch(:version)
    assert_equal expected.full_gem_path, tool.fetch(:root)
    assert_equal expected.require_paths, tool.fetch(:require_paths)
    assert_includes RubyButler::GemInstaller::Runtime.read.fetch(:configuration_inputs), expected.full_gem_path
  end

  def test_configuration_inputs_cover_the_runtime_and_compatibility_libraries
    inputs = RubyButler::GemInstaller::Runtime.read.fetch(:configuration_inputs)
    assert_includes inputs, File.expand_path(RbConfig.ruby)
    assert_includes inputs, File.expand_path(Gem.default_specifications_dir)
    Gem.path.each { |path| assert_includes inputs, File.expand_path(File.join(path, 'specifications')) }
    Gem::Specification.find_all_by_name('bundler').each do |spec|
      assert_includes inputs, File.expand_path(spec.loaded_from)
      assert_includes inputs, File.expand_path(File.join(spec.full_gem_path, 'lib'))
    end
    shared = RbConfig::CONFIG['LIBRUBY_SO']
    if shared && !shared.empty?
      %w[bindir libdir].each do |key|
        assert_includes inputs, File.expand_path(File.join(RbConfig::CONFIG.fetch(key), shared))
      end
    end
    assert(inputs.any? { |path| File.basename(path) == 'rbconfig.rb' })
    assert(inputs.any? { |path| File.basename(path) == 'rubygems.rb' })
    assert_equal inputs.uniq.sort, inputs
  end
end
