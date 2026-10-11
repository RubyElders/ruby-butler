require 'json'

puts RUBY_DESCRIPTION
abort unless system('rb', 'version')
rows = JSON.parse(File.read('/corpus/manifest.json')).fetch('gems')
names = ARGV.empty? ? %w[rmagick ffi-yajl charlock_holmes commonmarker prometheus-client-mmap rubydex] : ARGV
names.each do |name|
  row = rows.find { |gem| gem.fetch('name') == name } or abort "Missing #{name}"
  arguments = name == 'charlock_holmes' ? ['--with-cxxflags=-std=c++17'] : []
  abort "FAILED #{name}" unless system('butler-compile', "/corpus/#{row.fetch('path')}", "/work/#{name}", 'ruby', *arguments)
  puts "PASSED #{name}"
end
if names.include?('rmagick')
  abort unless system('ruby', '-I/work/rmagick/lib', '-rrmagick', '-e', 'raise unless Magick::Image.new(1, 1).columns == 1; puts "RMagick runtime passed"')
end
if names.include?('ffi-yajl')
  abort unless system('ruby', '-I/work/ffi-yajl/lib', '-rffi_yajl', '-e', 'raise unless FFI_Yajl::Parser.parse(%q({"ok":true}))["ok"]; puts "ffi-yajl runtime passed"')
end
