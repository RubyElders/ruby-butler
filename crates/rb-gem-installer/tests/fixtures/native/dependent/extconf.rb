require 'mkmf'
gem 'dinner', '= 1.2.3'
require 'butler_native'
raise 'Compiled dependency could not be used' unless ButlerNative.answer == 42
raise 'Wrong installed dependency root' unless Gem.loaded_specs.fetch('dinner').full_gem_path == ENV.fetch('EXPECTED_DEPENDENCY_ROOT')
create_makefile('butler_native')
