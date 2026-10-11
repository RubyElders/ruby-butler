require 'mkmf'
raise 'Caller make flags were lost' unless ENV.fetch('MAKEFLAGS') == '-s -j2'
require 'rubygems/command'
raise 'Build arguments changed' unless Gem::Command.build_args == ['--with-extra=a b'] && ARGV.include?('--with-extra=a b')
require 'helper_api'
raise 'Wrong helper API' unless HELPER_ANSWER == 42
raise 'Dependency was not activated' unless Gem.loaded_specs.fetch('build-helper').version.to_s == '2.1.0'
raise 'Wrong current gem root' unless Gem.loaded_specs.fetch('dinner').full_gem_path == File.realpath(__dir__)
create_makefile('butler_native')
