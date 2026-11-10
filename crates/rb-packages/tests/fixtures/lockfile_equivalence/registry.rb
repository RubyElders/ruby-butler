require 'rubygems/package'
require 'digest'
require 'fileutils'

root = ARGV.fetch(0)
FileUtils.mkdir_p(File.join(root, 'gems'))
FileUtils.mkdir_p(File.join(root, 'info'))
versions = []
{
  'porcelain' => [['1.0.0', []], ['1.5.0', []], ['2.0.0', []]],
  'tea' => [['1.0.0', [['porcelain', '~> 1', '>= 1.0.0']]]]
}.each do |name, releases|
  lines = []
  releases.each do |version, dependencies|
    spec = Gem::Specification.new do |gem|
      gem.name = name
      gem.version = version
      gem.summary = 'Lockfile comparison fixture'
      gem.authors = ['Fixture']
      gem.license = 'MIT'
      dependencies.each { |dependency, *requirements| gem.add_dependency(dependency, *requirements) }
    end
    archive = File.join(root, 'gems', "#{spec.full_name}.gem")
    Gem::Package.build(spec, true, false, archive)
    constraints = dependencies.map { |dependency, *requirements| "#{dependency}:#{requirements.join('&')}" }.join(',')
    lines << "#{version} #{constraints}|checksum:#{Digest::SHA256.file(archive).hexdigest}"
  end
  info = "---\n#{lines.join("\n")}\n"
  File.write(File.join(root, 'info', name), info)
  versions << "#{name} #{releases.map(&:first).join(',')} #{Digest::MD5.hexdigest(info)}"
end
File.write(File.join(root, 'versions'), "---\n#{versions.join("\n")}\n")
