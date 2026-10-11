module MicroGems
  class Requirement
    attr_reader :requirements

    def self.create(value) = value.is_a?(self) ? value : new(*Array(value))
    def self.default = new('>= 0')

    def initialize(*values)
      values = values.flatten.compact.flat_map { |v|
        v.is_a?(Requirement) ? v.requirements.map { |op, version|
          "#{op} #{version}"
        } : v.to_s.split(',')
      }
      values = ['>= 0'] if values.empty?
      @requirements = values.map do |value|
        match = /\A\s*(~>|>=|<=|!=|=|>|<)?\s*(\S+)\s*\z/.match(value)
        raise ArgumentError, "Malformed requirement #{value.inspect}" unless match

        [match[1] || '=', Version.new(match[2])]
      end.uniq
    end

    def satisfied_by?(version)
      version = Version.new(version)
      requirements.all? do |op, target|
        case op
        when '=' then version == target
        when '!=' then version != target
        when '>' then version > target
        when '<' then version < target
        when '>=' then version >= target
        when '<=' then version <= target
        when '~>' then version >= target && version.release < target.bump
        end
      end
    end
    alias === satisfied_by?
    def prerelease? = requirements.any? { |_, version| version.prerelease? }
    def exact? = requirements.size == 1 && requirements.first.first == '='
    def none? = self == self.class.default
    def to_s = requirements.map { |op, version| "#{op} #{version}" }.join(', ')
    def as_list = requirements.map { |op, version| "#{op} #{version}" }
    def ==(other) = other.is_a?(Requirement) && requirements.sort_by(&:to_s) == other.requirements.sort_by(&:to_s)
  end
end
