require 'json'
require 'rubygems'

def constraints(value)
  value.empty? ? [value] : value.split(/[,&]/, -1)
end

cases = JSON.parse(File.read(ARGV.fetch(0)))
versions = cases.fetch('versions').map { |value| Gem::Version.new(value) }
requirements = cases.fetch('requirements').map { |value| Gem::Requirement.new(*constraints(value)) }
platforms = cases.fetch('platforms').map { |value| Gem::Platform.new(value) }
parts = platforms.map { |platform| platform == 'ruby' ? [nil, 'ruby', nil] : platform.to_a }
puts JSON.generate({
  versions: versions.map(&:to_s),
  comparison: versions.product(versions).map { |a, b| a <=> b },
  release: versions.map { |version| version.release.to_s },
  bump: versions.map { |version| version.bump.to_s },
  prerelease: versions.map(&:prerelease?),
  invalid_versions: cases.fetch('invalid_versions').map { |value| !Gem::Version.correct?(value) },
  requirements: requirements.product(versions).map { |requirement, version| requirement.satisfied_by?(version) },
  requirement_prerelease: requirements.map(&:prerelease?),
  invalid_requirements: cases.fetch('invalid_requirements').map do |value|
    begin
      Gem::Requirement.new(*constraints(value))
      false
    rescue ArgumentError
      true
    end
  end,
  platforms: parts,
  platform_matches: platforms.product(platforms).map do |gem, target|
    gem == 'ruby' || (target != 'ruby' && !!(gem === target))
  end
})
