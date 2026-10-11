require 'json'
versions = %w[0 1 1.0 1.0.0 1.0.a 1.0.a.0 1.0.rc1 1.0-rc1 1.2.3 1.2.4 2 2.0.pre 10.4]
requirements = ['>= 0', '~> 1.2', '~> 1.2.3', '>= 1', '< 2', '!= 1.0', '= 1', '~> 1.0.a', '> 1.0.a']
platforms = %w[x86_64-linux x86_64-linux-gnu aarch64-linux-musl arm-linux i686-linux x86_64-darwin-23 arm64-darwin-24 x64-mingw-ucrt x86-mswin32-60 java]
puts JSON.generate(
  comparisons: versions.product(versions).map do |a, b|
    [Gem::Version.new(a) <=> Gem::Version.new(b), Gem::Version.new(a).eql?(Gem::Version.new(b))]
  end,
  requirements: requirements.product(versions).map do |a, b|
    Gem::Requirement.new(a).satisfied_by?(Gem::Version.new(b))
  end,
  versions: versions.map do |v|
    x = Gem::Version.new(v)
    [x.to_s, x.release.to_s, x.bump.to_s, x.prerelease?, x.approximate_recommendation]
  end,
  generic_platforms: [nil, '', 'ruby', 'current'].map do |p|
    x = Gem::Platform.new(p)
    [x.to_s, x.is_a?(String)]
  end,
  platforms: platforms.map do |p|
    x = Gem::Platform.new(p)
    [x.to_a, x.to_s]
  end
)
