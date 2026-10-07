require 'bundler'
require 'open3'
require 'tmpdir'

def snapshot(text)
  parser = Bundler::LockfileParser.new(text)
  {
    sources: parser.sources.map(&:to_lock).sort,
    specs: parser.specs.map { |spec|
      [spec.name, spec.version.to_s, spec.platform.to_s,
       spec.source.to_lock,
       spec.dependencies.map { |dep| [dep.name, dep.requirement.to_s] }.sort,
       parser.checksums ? spec.source.checksum_store.to_lock(spec) : nil]
    }.sort,
    dependencies: parser.dependencies.map { |name, dep|
      [name, dep.requirement.to_s, dep.source&.to_lock]
    }.sort,
    platforms: parser.platforms.map(&:to_s).sort,
    ruby: parser.ruby_version,
    bundler: parser.bundler_version&.to_s,
    checksums: parser.checksums
  }
end

binary = File.expand_path(ARGV.fetch(0))
fixtures = Dir[File.join(__dir__, 'fixtures', '*.lock')].sort
Dir.mktmpdir('rb-lockfile-oracle') do |directory|
  File.write(File.join(directory, 'Gemfile'), "source 'https://rubygems.org'\n")
  Dir.chdir(directory) do
    fixtures.each do |path|
      rendered, error, status = Open3.capture3(binary, File.expand_path(path))
      raise error unless status.success?
      original = snapshot(File.read(path))
      restored = snapshot(rendered)
      raise "Bundler semantic mismatch: #{path}" unless original == restored
      puts "PASS #{File.basename(path)}"
    end
  end
end
