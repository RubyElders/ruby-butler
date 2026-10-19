require 'rbconfig'

module RubyButler
  module GemInstaller
    module Runtime
      def self.read
        require 'rubygems'
        bundlers = Gem::Specification.find_all_by_name('bundler')
        { executable: RbConfig.ruby, version: RUBY_VERSION, engine: RUBY_ENGINE,
          engine_version: RUBY_ENGINE_VERSION, patchlevel: RUBY_PATCHLEVEL,
          platform: Gem::Platform.local.to_s, abi: Gem.extension_api_version,
          rubygems: Gem::VERSION, bundler: bundlers.map(&:version).max&.to_s,
          build_tools: build_tools, configuration_inputs: configuration_inputs(bundlers) }
      end

      def self.build_tools
        Gem::Specification.find_all_by_name('rake')
                          .select { |spec| spec.full_gem_path.start_with?(File.join(Gem.default_dir, 'gems') + File::SEPARATOR) }
                          .max_by(&:version).then do |spec|
          spec ? [{ name: spec.name, version: spec.version.to_s, platform: spec.platform.to_s,
                    root: spec.full_gem_path, require_paths: spec.require_paths,
                    metadata: spec.metadata, executables: spec.executables, bindir: spec.bindir }] : []
        end
      end
      private_class_method :build_tools

      def self.configuration_inputs(bundlers)
        loaded = $LOADED_FEATURES.select { |path| %w[rbconfig.rb rubygems.rb].include?(File.basename(path)) }
        libraries = loaded.select { |path| File.basename(path) == 'rubygems.rb' }
                          .map { |path| File.join(File.dirname(path), 'rubygems') }
        shared = RbConfig::CONFIG['LIBRUBY_SO']
        paths = [RbConfig.ruby, *loaded, *libraries, Gem.default_specifications_dir,
                 *Gem.path.map { |path| File.join(path, 'specifications') }]
        paths.concat(%w[bindir libdir].map { |key| File.join(RbConfig::CONFIG.fetch(key), shared) }) if shared && !shared.empty?
        bundlers.each { |spec| paths.concat([spec.loaded_from, File.join(spec.full_gem_path, 'lib')]) }
        build_tools.each { |tool| paths << tool.fetch(:root) }
        paths.compact.map { |path| File.expand_path(path) }.uniq.sort
      end
      private_class_method :configuration_inputs
    end
  end
end
