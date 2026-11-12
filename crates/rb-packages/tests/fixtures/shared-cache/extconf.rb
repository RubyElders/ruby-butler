File.open(ENV.fetch('BUILD_COUNTER'), 'a') { |file| file.puts 'build' }
raise 'fixture build failed' if ARGV.include?('--fail-cache-build')
require 'mkmf'
create_makefile('package_native')
