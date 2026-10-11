require 'rbconfig'
require 'shellwords'

module RubyButler
  module GemInstaller
    module RubyConfig
      def self.read(config: RbConfig::CONFIG, executable: RbConfig.ruby, version: RUBY_VERSION)
        {
          executable: executable,
          version: version,
          config: config,
          linker: Shellwords.split(config.fetch('CC', 'cc')),
          libraries: Shellwords.split(config.fetch('LIBRUBYARG_SHARED', '') + ' ' + config.fetch('LIBS', ''))
        }
      end
    end
  end
end
