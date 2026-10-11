require 'rbconfig'
require 'json'

module MicroGems
  VERSION = '0.3.1'
  WIN_PATTERNS = [/mswin/, /mingw/, /cygwin/].freeze
  class Exception < StandardError; end
  class InstallError < Exception; end

  class LoadError < ::LoadError
    attr_accessor :name, :requirement
  end

  class MissingSpecError < LoadError; end
  class MissingSpecVersionError < MissingSpecError; end
end

require_relative 'microgems/version'
require_relative 'microgems/requirement'
require_relative 'microgems/platform'
require_relative 'microgems/specification'

module MicroGems
  FEATURES = %w[rubygems rubygems/version rubygems/requirement rubygems/platform rubygems/specification rubygems/ext
                rubygems/ext/builder rubygems/command rubygems/package].freeze

  def self.configure(manifest)
    @specs = manifest.fetch('gems', []).map { |record| MicroGems::Specification.new(record) }
    @loaded = {}
    @arguments = manifest.fetch('arguments', [])
    duplicates = @specs.group_by { |spec|
      [spec.name, spec.version.to_s, spec.platform.to_s]
    }.select { |_, specs| specs.length > 1 }
    raise ArgumentError, 'Duplicate installed gem identities' unless duplicates.empty?
  end

  def self.specs = @specs || []
  def self.loaded = @loaded ||= {}
  def self.arguments = @arguments || []

  def self.activate(name, *requirements)
    requirement = MicroGems::Requirement.new(*requirements)
    if (active = loaded[name.to_s])
      raise MicroGems::LoadError,
            "#{name} #{active.version} is already activated; requested #{requirement}" unless requirement.satisfied_by?(active.version)

      return false
    end
    spec = MicroGems::Specification.find_by_name(name, requirement)
    spec.full_require_paths.reverse_each { |path| $LOAD_PATH.unshift(path) unless $LOAD_PATH.include?(path) }
    loaded[spec.name] = spec
    true
  end

  module Requires
    def require(feature)
      name = feature.to_s.delete_suffix('.rb')
      if name.start_with?('rubygems/') && !MicroGems::FEATURES.include?(name)
        raise ::LoadError, "Unsupported RubyGems compatibility feature: #{feature}"
      end

      super
    rescue ::LoadError => error
      raise unless error.respond_to?(:path) && error.path == feature.to_s

      candidates = MicroGems.specs.select do |spec|
        MicroGems::Platform.match(spec.platform) && spec.full_require_paths.any? do |path|
          MicroGems.suffixes.any? { |suffix| File.file?(File.join(path, feature.to_s + suffix)) }
        end
      end.sort.reverse
      spec = candidates.find { |candidate|
        !MicroGems.loaded.key?(candidate.name) || MicroGems.loaded[candidate.name].version == candidate.version
      }
      raise unless spec

      MicroGems.activate(spec.name, "= #{spec.version}")
      super(feature)
    end
  end
end

module MicroGems
  class << self
    def win_platform? = WIN_PATTERNS.any? { |pattern| pattern.match?(RbConfig::CONFIG.fetch('host_os', RUBY_PLATFORM)) }
    def ruby = RbConfig.ruby
    def ruby_version = Version.new(RUBY_VERSION)
    def target_rbconfig = RbConfig
    def loaded_specs = MicroGems.loaded
    def platforms = [Platform::RUBY, Platform.local]
    def install_extension_in_lib = true
    def configuration = @configuration ||= {}
    def default_exec_format = '%s'
    def suffixes = ['', '.rb', ".#{RbConfig::CONFIG.fetch('DLEXT')}"]
    def binary_mode = 'rb'

    def bin_path(name, executable, *requirements)
      spec = Specification.find_by_name(name, *requirements)
      raise MicroGems::Exception,
            "Missing executable #{executable} for #{name}" unless spec.executables.include?(executable)

      File.join(spec.full_gem_path, spec.bindir, executable)
    end
  end
end
