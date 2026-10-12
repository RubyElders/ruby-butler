require 'rbconfig'

module RubyButler
  module GemInstaller
    module Runtime
      def self.read
        require 'rubygems'
        { executable: RbConfig.ruby, version: RUBY_VERSION, engine: RUBY_ENGINE,
          engine_version: RUBY_ENGINE_VERSION, patchlevel: RUBY_PATCHLEVEL,
          platform: Gem::Platform.local.to_s, abi: Gem.extension_api_version,
          rubygems: Gem::VERSION,
          bundler: Gem::Specification.find_all_by_name('bundler').map(&:version).max&.to_s }
      end
    end
  end
end
