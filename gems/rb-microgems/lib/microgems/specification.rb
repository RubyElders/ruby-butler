module MicroGems
  class Specification
    include Comparable

    attr_reader :name, :version, :platform, :require_paths, :metadata, :executables, :bindir

    def initialize(record)
      @name = record.fetch('name')
      @version = Version.new(record.fetch('version'))
      @platform = Platform.new(record.fetch('platform', 'ruby'))
      @root = File.expand_path(record.fetch('root'))
      @require_paths = record.fetch('require_paths', ['lib'])
      @metadata = record.fetch('metadata', {})
      @executables = record.fetch('executables', [])
      @bindir = record.fetch('bindir', 'bin')
    end

    def full_gem_path = @root
    alias gem_dir full_gem_path
    def full_require_paths = require_paths.map { |path| File.expand_path(path, full_gem_path) }
    alias raw_require_paths require_paths
    def full_name = [name, version.to_s, platform.to_s == 'ruby' ? nil : platform.to_s].compact.join('-')
    def <=>(other) = version <=> other.version
    def activate = MicroGems.activate(name, "= #{version}")

    def self.to_a = MicroGems.specs
    def self.each(&block) = to_a.each(&block)

    def self.find_all_by_name(name, *requirements)
      requirement = Requirement.new(*requirements)
      to_a.select { |spec|
        spec.name == name.to_s && requirement.satisfied_by?(spec.version) && Platform.match(spec.platform)
      }.sort.reverse
    end

    def self.find_by_name(name, *requirements)
      find_all_by_name(name,
                       *requirements).first || raise(MissingSpecError,
                                                     "Missing installed gem #{name} (#{Requirement.new(*requirements)})")
    end
  end
end
