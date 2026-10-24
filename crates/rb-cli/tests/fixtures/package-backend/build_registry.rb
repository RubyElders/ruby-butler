require 'fileutils'
require 'rubygems/package'
require 'zlib'

source, registry = ARGV
FileUtils.mkdir_p(File.join(registry, 'gems'))
specification = nil
Dir.chdir(source) do
  specification = Gem::Specification.load('backend_fixture.gemspec')
  archive = Gem::Package.build(specification)
  FileUtils.mv(archive, File.join(registry, 'gems', archive))
end

record = [specification.name, specification.version, specification.platform.to_s]
{ 'specs' => [record], 'latest_specs' => [record], 'prerelease_specs' => [] }.each do |name, records|
  Zlib::GzipWriter.open(File.join(registry, "#{name}.4.8.gz")) do |file|
    file.write(Marshal.dump(records))
  end
end
quick = File.join(registry, 'quick', 'Marshal.4.8')
FileUtils.mkdir_p(quick)
File.binwrite(File.join(quick, "#{specification.full_name}.gemspec.rz"), Zlib::Deflate.deflate(Marshal.dump(specification)))
