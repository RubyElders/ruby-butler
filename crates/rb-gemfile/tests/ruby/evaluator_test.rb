require "minitest/autorun"
require "tmpdir"
require "fileutils"
require "open3"
require "rbconfig"
require_relative "../../assets/gemfile"
require_relative "../../assets/sources"
require_relative "../../assets/gemspec"
require_relative "../../assets/compatibility"

class EvaluatorTest < Minitest::Test
  FIXTURES = File.expand_path("../fixtures", __dir__)
  ENTRYPOINT = File.expand_path("../../assets/evaluate.rb", __dir__)

  def setup
    @evaluator = RubyButler::Gemfile.new
    evaluator = @evaluator
    RubyButler.define_singleton_method(:evaluator) { evaluator }
  end

  def teardown
    RubyButler.singleton_class.remove_method(:evaluator)
  end

  def test_compatibility_read_file_tracks_inputs
    path = File.join(FIXTURES, "format/evaluate-1.rb")
    assert_equal File.read(path), Bundler.read_file(path)
    assert_equal [path], @evaluator.inputs
    refute Gem.loaded_specs.key?("bundler")
  end

  def test_plugin_index_does_not_load_plugins
    assert_same Bundler::Plugin, Bundler::Plugin.index
    assert_empty Bundler::Plugin.index.load_paths("bundler-inject")
    refute Gem.loaded_specs.key?("bundler-inject")
  end

  def test_groups_sources_and_conditions
    @evaluator.eval_gemfile(File.join(FIXTURES, "format/evaluate-1.rb"))
    dependencies = @evaluator.manifest.fetch(:dependencies)
    assert_equal ["~> 8.0"], dependencies.fetch("rails").fetch(:version)
    assert_equal ["development"], dependencies.fetch("debug").fetch(:groups)
    assert_equal false, dependencies.fetch("debug").fetch(:require)
    assert_equal "https://gems.example.com", dependencies.fetch("internal").fetch(:source)
    assert_equal "v1.0", dependencies.fetch("git-gem").fetch(:tag)
    assert_equal false, dependencies.fetch("excluded").fetch(:enabled)
  end

  def test_source_cooldowns_do_not_change_declarations
    @evaluator.eval_gemfile(File.join(FIXTURES, "ruby/source_options/Gemfile"))
    refute @evaluator.manifest.fetch(:bundle).key?(:source_cooldowns)
    assert_equal "https://example.test", @evaluator.manifest.fetch(:dependencies).fetch("scoped").fetch(:source)
  end

  def test_source_cooldowns_reject_invalid_values
    [-1, "7", 1.5, true].each do |value|
      error = assert_raises(ArgumentError) { @evaluator.source("https://rubygems.org", cooldown: value) }
      assert_match "non-negative integer", error.message
    end
  end

  def test_source_accepts_zero_cooldown_and_rejects_unknown_options
    @evaluator.source("https://rubygems.org", cooldown: 0)
    assert_equal "https://rubygems.org", @evaluator.manifest.fetch(:bundle).fetch(:source)
    error = assert_raises(ArgumentError) { @evaluator.source("https://example.test", mystery: true) }
    assert_match "Unsupported source options", error.message
  end

  def test_gemspec_accepts_require_without_changing_bundler_activation
    @evaluator.eval_gemfile(File.join(FIXTURES, "ruby/gemspec_options/Gemfile"))
    assert_equal ["= 1.0.0"], @evaluator.manifest.fetch(:dependencies).fetch("sample").fetch(:version)
    refute @evaluator.manifest.fetch(:dependencies).fetch("sample").key?(:require)
  end

  def test_pathname_is_available_without_loading_bundler
    Dir.mktmpdir do |directory|
      gemfile = File.join(directory, "Gemfile")
      result = File.join(directory, "manifest.json")
      File.write(gemfile, "gem Pathname.new('rake').to_s")
      _, stderr, status = Open3.capture3(RbConfig.ruby, ENTRYPOINT, gemfile, result)
      assert status.success?, stderr
      assert JSON.parse(File.read(result)).fetch("manifest").fetch("dependencies").key?("rake")
    end
  end

  def test_require_nil_disables_autorequire
    @evaluator.gem("rake", require: nil)
    @evaluator.gem("json")
    assert_equal false, @evaluator.manifest.fetch(:dependencies).fetch("rake").fetch(:require)
    refute @evaluator.manifest.fetch(:dependencies).fetch("json").key?(:require)
  end

  def test_gemfile_helpers_can_inspect_and_remove_dependencies
    @evaluator.eval_gemfile(File.join(FIXTURES, "ruby/compatibility_helpers/Gemfile"))
    dependencies = @evaluator.manifest.fetch(:dependencies)
    refute dependencies.key?("keep")
    assert_equal ["~> 3"], dependencies.fetch("replace").fetch(:version)
    assert_equal ["~> 3"], dependencies.fetch("rack").fetch(:version)
    assert_equal ["windows"], dependencies.fetch("tzinfo-data").fetch(:platforms)
    refute Gem.loaded_specs.key?("bundler")
  end

  def test_platforms_and_ruby_constraints_are_preserved
    @evaluator.ruby(">= 3.2", "< 5")
    @evaluator.platforms(:jruby) { @evaluator.gem("jdbc", require: false) }
    assert_equal [">= 3.2", "< 5"], @evaluator.manifest.fetch(:bundle).fetch(:ruby)
    dependency = @evaluator.manifest.fetch(:dependencies).fetch("jdbc")
    assert_equal ["jruby"], dependency.fetch(:platforms)
    assert_equal false, dependency.fetch(:require)
  end

  def test_removed_gemspec_dependency_can_be_redeclared
    Dir.mktmpdir do |directory|
      File.write(File.join(directory, "Gemfile"), "gemspec")
      FileUtils.cp(File.join(FIXTURES, "evaluation/loads_project_gemspec_and_ruby_version_file-1.rb"),
                   File.join(directory, "sample.gemspec"))
      @evaluator.eval_gemfile(File.join(directory, "Gemfile"))
      @evaluator.dependencies.reject! { |dependency| dependency.name == "rake" }
      @evaluator.gem("rake", "~> 14")
      assert_equal ["~> 14"], @evaluator.manifest.fetch(:dependencies).fetch("rake").fetch(:version)
      assert_equal Gem::Requirement.new("~> 14"), @evaluator.dependencies.find { |dependency| dependency.name == "rake" }.requirement
    end
  end

  def test_gemspec_metadata_and_development_dependencies
    Dir.mktmpdir do |directory|
      File.write(File.join(directory, "Gemfile"), "gemspec")
      FileUtils.cp(File.join(FIXTURES, "evaluation/loads_project_gemspec_and_ruby_version_file-1.rb"),
                   File.join(directory, "sample.gemspec"))
      @evaluator.eval_gemfile(File.join(directory, "Gemfile"))
      package = @evaluator.manifest.fetch(:gemspecs).first
      assert_equal "sample", package.fetch(:name)
      assert_equal "1.2.3", package.fetch(:version)
      assert_equal ["~> 3.0"], package.fetch(:runtime_dependencies).fetch("rack")
      assert_equal [">= 13"], package.fetch(:development_dependencies).fetch("rake")
      assert_equal ["development"], @evaluator.manifest.fetch(:dependencies).fetch("rake").fetch(:groups)
    end
  end

  def test_plugins_are_metadata_only
    @evaluator.plugin("bundler-inject", "~> 2.0")
    assert_equal [{ name: "bundler-inject", version: ["~> 2.0"] }], @evaluator.manifest.fetch(:bundle).fetch(:plugins)
    refute Gem.loaded_specs.key?("bundler-inject")
  end

  def test_nested_source_scopes_restore_context
    @evaluator.eval_gemfile(File.join(FIXTURES, "ruby/nested_sources.rb"))
    dependencies = @evaluator.manifest.fetch(:dependencies)
    assert_equal "https://example.test", dependencies.fetch("registry").fetch(:source)
    refute dependencies.fetch("registry").key?(:branch)
    assert_equal "main", dependencies.fetch("outer").fetch(:branch)
    refute dependencies.fetch("normal").key?(:git)
  end

  def test_invalid_options_and_recursive_includes_fail
    error = assert_raises(ArgumentError) { @evaluator.gem("rake", mystery: true) }
    assert_match "Unsupported gem options", error.message
    error = assert_raises(ArgumentError) { @evaluator.eval_gemfile("Gemfile", "eval_gemfile 'Gemfile'") }
    assert_match "Recursive eval_gemfile", error.message
  end

  def test_entrypoint_writes_json_separately_from_diagnostics
    Dir.mktmpdir do |directory|
      gemfile = File.join(directory, "Gemfile")
      result = File.join(directory, "manifest.json")
      File.write(gemfile, "puts 'hello'; warn 'warning'; gem 'rake'")
      stdout, stderr, status = Open3.capture3(RbConfig.ruby, ENTRYPOINT, gemfile, result)
      assert status.success?, stderr
      assert_equal ["hello"], stdout.lines.map(&:chomp)
      assert_equal ["warning"], stderr.lines.map(&:chomp)
      manifest = JSON.parse(File.read(result))
      assert_equal [gemfile], manifest.fetch("inputs")
      assert_equal [">= 0"], manifest.fetch("manifest").fetch("dependencies").fetch("rake").fetch("version")
    end
  end

  def test_entrypoint_reports_failures_without_a_manifest
    Dir.mktmpdir do |directory|
      gemfile = File.join(directory, "Gemfile")
      result = File.join(directory, "manifest.json")
      File.write(gemfile, "raise 'invalid teacup'")
      stdout, stderr, status = Open3.capture3(RbConfig.ruby, ENTRYPOINT, gemfile, result)
      refute status.success?
      assert_empty stdout
      assert_match "RuntimeError: invalid teacup", stderr
      refute File.exist?(result)
    end
  end
end
