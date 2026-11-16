counter = ENV.fetch('BUILD_COUNTER')
File.open(counter, 'a') { |file| file.puts 'build' }
require 'mkmf'
$defs << "-DFIXTURE_ANSWER=#{41 + File.readlines(counter).length}"
create_makefile('source_native')
