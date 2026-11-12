require 'leaf'
require 'other'
File.write('dependency-location', ['leaf', 'other'].map { |name| Gem.loaded_specs.fetch(name).full_gem_path }.join("\n"))
File.open(ENV.fetch('BUILD_COUNTER'), 'a') { |file| file.puts 'build' }
require 'mkmf'
create_makefile('package_native')
