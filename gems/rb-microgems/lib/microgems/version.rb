module MicroGems
  class Version
    include Comparable

    VERSION_PATTERN = '[0-9]+(?:\.[0-9a-zA-Z]+)*(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?'.freeze
    attr_reader :version

    def self.new(value)
      return value if value.is_a?(self)

      super
    end

    def self.correct?(value)
      /\A\s*(?:#{VERSION_PATTERN})?\s*\z/.match?(value.to_s)
    end

    def self.create(value)
      new(value)
    end

    def initialize(value)
      raise ArgumentError, "Malformed version #{value.inspect}" unless self.class.correct?(value)

      @version = value.to_s.strip.gsub('-', '.pre.')
      @version = '0' if @version.empty?
    end

    def segments
      @segments ||= version.scan(/[0-9]+|[a-zA-Z]+/).map { |part| /\A\d+\z/.match?(part) ? part.to_i : part }.freeze
    end

    def canonical_segments
      boundary = segments.index { |part| part.is_a?(String) } || segments.size
      [segments.take(boundary), segments.drop(boundary)].flat_map do |parts|
        parts.pop while parts.last == 0
        parts
      end
    end

    def <=>(other)
      other = self.class.new(other) if other.is_a?(String)
      return unless other.is_a?(Version)

      left, right = canonical_segments, other.canonical_segments
      [left.size, right.size].max.times do |i|
        a, b = left.fetch(i, 0), right.fetch(i, 0)
        next if a == b
        return a <=> b if a.class == b.class

        return a.is_a?(String) ? -1 : 1
      end
      0
    end

    def prerelease? = segments.any?(String)

    def release
      self.class.new(segments.take_while { |part| part.is_a?(Integer) }.join('.'))
    end

    def bump
      parts = release.segments.dup
      parts.pop if parts.size > 1
      parts[-1] += 1
      self.class.new(parts.join('.'))
    end

    def approximate_recommendation
      parts = release.segments.first(2)
      parts << 0 if parts.length == 1
      "~> #{parts.join('.')}#{prerelease? ? '.a' : ''}"
    end

    def to_s = version
    alias to_str to_s
    def inspect = "#<MicroGems::Version #{version.inspect}>"
    def eql?(other) = other.is_a?(Version) && version == other.version
    def hash = canonical_segments.hash
  end
end
