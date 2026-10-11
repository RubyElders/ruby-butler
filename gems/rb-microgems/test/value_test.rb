require_relative 'test_helper'

class ValueTest < Minitest::Test
  VERSIONS = %w[0 1 1.0 1.0.0 1.0.a 1.0.a.0 1.0.rc1 1.0-rc1 1.2.3 1.2.4 2 2.0.pre 10.4].freeze
  REQUIREMENTS = ['>= 0', '~> 1.2', '~> 1.2.3', '>= 1', '< 2', '!= 1.0', '= 1', '~> 1.0.a', '> 1.0.a'].freeze

  def test_version_order_and_identity_match_rubygems
    VERSIONS.product(VERSIONS).each do |left, right|
      expected = Gem::Version.new(left)
      actual = MicroGems::Version.new(left)
      assert_equal expected <=> Gem::Version.new(right), actual <=> MicroGems::Version.new(right)
      assert_equal expected.eql?(Gem::Version.new(right)), actual.eql?(MicroGems::Version.new(right))
    end
  end

  def test_version_operations_match_rubygems
    VERSIONS.each do |version|
      expected = Gem::Version.new(version)
      actual = MicroGems::Version.new(version)
      assert_equal expected.release.to_s, actual.release.to_s
      assert_equal expected.bump.to_s, actual.bump.to_s
      assert_equal expected.prerelease?, actual.prerelease?
      assert_equal expected.approximate_recommendation, actual.approximate_recommendation
    end
    assert_raises(ArgumentError) { MicroGems::Version.new('not-a-version') }
  end

  def test_requirements_match_rubygems
    REQUIREMENTS.product(VERSIONS).each do |requirement, version|
      assert_equal Gem::Requirement.new(requirement).satisfied_by?(Gem::Version.new(version)),
                   MicroGems::Requirement.new(requirement).satisfied_by?(MicroGems::Version.new(version))
    end
    constraint = MicroGems::Requirement.new('>= 1', '< 2')
    assert constraint.satisfied_by?('1.5')
    refute constraint.satisfied_by?('2')
    assert_raises(ArgumentError) { MicroGems::Requirement.new('? 1') }
  end

  def test_platform_fields_match_rubygems
    %w[x86_64-linux x86_64-linux-gnu aarch64-linux-musl arm-linux i686-linux x86_64-darwin-23 arm64-darwin-24
       x64-mingw-ucrt x86-mswin32-60 java].each do |platform|
      expected = Gem::Platform.new(platform)
      actual = MicroGems::Platform.new(platform)
      assert_equal expected.to_a, actual.to_a
      assert_equal expected.to_s, actual.to_s
    end
    [nil, '', 'ruby', 'current'].each do |platform|
      assert_equal Gem::Platform.new(platform).to_s, MicroGems::Platform.new(platform).to_s
    end
  end
end
