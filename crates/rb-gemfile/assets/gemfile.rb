require "json"
require "rubygems"
require "pathname"

module RubyButler
  class Gemfile
    attr_reader :inputs

    def initialize
      @bundle = { source: nil, ruby: [], optional_groups: [] }
      @declarations = {}
      @dependencies = []
      @gemspecs = []
      @gemspec_development = {}
      @inputs = []
      @stack = []
      @context = { groups: [], platforms: [], conditions: [] }
      @git_sources = {
        "github" => ->(name) { "https://github.com/#{name.include?('/') ? name : "#{name}/#{name}"}.git" },
        "gitlab" => ->(name) { "https://gitlab.com/#{name.include?('/') ? name : "#{name}/#{name}"}.git" },
        "gist" => ->(name) { "https://gist.github.com/#{name}.git" },
        "bitbucket" => ->(name) {
          user, repo = name.split('/')
          "https://#{user}@bitbucket.org/#{user}/#{repo || user}.git"
        }
      }
    end

    def dependencies
      @dependencies
    end

    def plugin(name, *requirements, **options)
      raise ArgumentError, "Plugin options are unsupported" unless options.empty?
      requirements = Gem::Requirement.new(requirements.flatten.compact).as_list
      @bundle[:plugins] ||= []
      @bundle[:plugins] << { name: name, version: requirements }
    end

    def manifest
      names = @dependencies.map(&:name)
      { bundle: @bundle, dependencies: @declarations.select { |name, _| names.include?(name) }, gemspecs: @gemspecs }
    end

    def eval_gemfile(path, contents = nil)
      path = File.expand_path(path, @stack.empty? ? Dir.pwd : File.dirname(@stack.last))
      raise ArgumentError, "Recursive eval_gemfile: #{path}" if @stack.include?(path)
      @inputs << path unless @inputs.include?(path)
      @stack.push(path)
      begin
        instance_eval(contents || read_file(path), path, 1)
      ensure
        @stack.pop
      end
    end

    def read_file(path)
      path = File.expand_path(path)
      @inputs << path unless @inputs.include?(path)
      File.read(path)
    end

    def lockfile(path)
      @bundle[:lockfile] = File.expand_path(path, File.dirname(@stack.last))
    end

    def env(value, &block)
      scoped({ environment: value }, &block)
    end

    def gem(name, *requirements, **options)
      options = requirements.pop.merge(options) if requirements.last.is_a?(Hash)
      options = options.transform_keys(&:to_sym)
      @git_sources.each do |key, expansion|
        if options.key?(key.to_sym)
          raise ArgumentError, "Conflicting Git sources for #{name}" if options[:git]
          expansion = expansion.call(options.delete(key.to_sym))
          expansion = { git: expansion } if expansion.is_a?(String)
          expansion = expansion.transform_keys(&:to_sym)
          raise ArgumentError, "Conflicting Git shortcut options for #{name}" unless (expansion.keys & options.keys).empty?
          options.merge!(expansion)
        end
      end
      valid = %i[group groups platform platforms require source git path branch tag ref rev glob submodules force_ruby_platform install_if]
      unknown = options.keys - valid
      raise ArgumentError, "Unsupported gem options: #{unknown.join(', ')}" unless unknown.empty?
      raise ArgumentError, "Gem names must be strings" unless name.is_a?(String)
      raise ArgumentError, "Empty gem name" if name.empty?
      raise ArgumentError, "Gem names cannot contain whitespace" if name.match?(/\s/)
      @declarations.delete(name) unless @dependencies.any? { |dependency| dependency.name == name }
      options[:require] = false if options.key?(:require) && options[:require].nil?
      requirements = requirements.flatten.compact.map(&:to_s)
      requirements = [">= 0"] if requirements.empty?
      Gem::Requirement.new(requirements)
      if @gemspec_development.delete(name) && (existing = @declarations.delete(name))
        requirements = (existing[:version] + requirements).uniq
      end
      groups = @context[:groups] + Array(options[:group]) + Array(options[:groups])
      platforms = @context[:platforms] + Array(options[:platform]) + Array(options[:platforms])
      platforms.each do |platform|
        unless platform.to_s.match?(/\A(?:ruby|mri|rbx|truffleruby|jruby|windows|mswin|mswin64|mingw|x64_mingw)(?:_\d+)?\z/)
          raise ArgumentError, "Unknown platform: #{platform}"
        end
      end
      dependency = @context.reject { |key, _| %i[groups platforms conditions environment].include?(key) }
      dependency.compact!
      if %i[source git path].any? { |key| options.key?(key) }
        dependency.reject! { |key, _| %i[source git path branch tag rev glob submodules].include?(key) }
      end
      dependency.merge!(options.reject { |key, _| %i[group groups platform platforms install_if].include?(key) })
      raise ArgumentError, "Specify ref or rev, not both for #{name}" if dependency[:ref] && dependency[:rev]
      dependency[:rev] = dependency.delete(:ref) if dependency.key?(:ref)
      if dependency[:path]
        dependency[:path] = File.expand_path(dependency[:path], File.dirname(@stack.last))
      end
      raise ArgumentError, "Conflicting sources for #{name}" if %i[source git path].count { |key| dependency[key] } > 1
      raise ArgumentError, "Conflicting Git selectors for #{name}" if %i[branch tag rev].count { |key| dependency[key] } > 1
      if !dependency[:git] && %i[branch tag rev submodules].any? { |key| dependency.key?(key) }
        raise ArgumentError, "Git options require a Git source for #{name}"
      end
      dependency.merge!(version: requirements, groups: groups.empty? ? ["default"] : groups.map(&:to_s).uniq,
                        platforms: platforms.map(&:to_s).uniq,
                        enabled: (@context[:conditions] + Array(options[:install_if])).all? { |value| condition(value) } && environment_enabled?)
      if (current = @declarations[name])
        unless Gem::Requirement.new(current[:version]) == Gem::Requirement.new(dependency[:version])
          raise ArgumentError, "Conflicting declarations for #{name}"
        end
        source_keys = %i[source git path branch tag rev glob submodules]
        unless source_keys.all? { |key| current[key] == dependency[key] }
          raise ArgumentError, "Conflicting sources for #{name}"
        end
        primary = current.reject { |key, _| key == :variants }
        unless primary == dependency || Array(current[:variants]).include?(dependency)
          (current[:variants] ||= []) << dependency
        end
      else
        @declarations[name] = dependency
      end
      refresh_dependency(name)
    end

    def group(*names, optional: false, &block)
      @bundle[:optional_groups] |= names.map(&:to_s) if optional
      scoped({ groups: @context[:groups] + names }, &block)
    end

    def platforms(*names, &block)
      scoped({ platforms: @context[:platforms] + names }, &block)
    end
    alias platform platforms

    def install_if(*conditions, &block)
      scoped({ conditions: @context[:conditions] + conditions }, &block)
    end

    private

    def refresh_dependency(name)
      @dependencies.reject! { |dependency| dependency.name == name }
      @dependencies << Gem::Dependency.new(name, @declarations.fetch(name).fetch(:version))
    end

    def environment_enabled?
      value = @context[:environment]
      return true unless value
      return !!ENV[value.to_s] unless value.is_a?(Hash)
      value.all? do |name, expected|
        actual = ENV[name.to_s]
        actual && (expected.is_a?(Regexp) ? expected.match?(actual) : actual == expected)
      end
    end

    def condition(value)
      !!(value.respond_to?(:call) ? value.call : value)
    end

    def scoped(values)
      raise ArgumentError, "This declaration requires a block" unless block_given?
      previous = @context
      @context = previous.merge(values)
      begin
        yield
      ensure
        @context = previous
      end
    end
  end
end
