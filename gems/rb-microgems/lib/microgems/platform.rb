module MicroGems
  class Platform
    RUBY = 'ruby'
    CURRENT = 'current'
    attr_reader :cpu, :os, :version

    def self.new(value)
      return RUBY if value.nil? || value == '' || value == RUBY
      return local if value == CURRENT

      super
    end

    def self.local
      arch = RUBY_ENGINE == 'jruby' ? 'java' : RbConfig::CONFIG.fetch('arch')
      arch += '_60' if arch.end_with?('mswin32', 'mswin64')
      new(arch)
    end

    def self.match(platform) = platform.to_s == RUBY || new(platform) === local
    def self.match_spec?(spec) = match(spec.platform)
    def self.match_gem?(platform, _name) = match(platform)

    def initialize(value)
      if value.is_a?(Array)
        @cpu, @os, @version = value
        return
      end
      parts = value.to_s.sub(/-+\z/, '').split('-', 2)
      @cpu, system = parts.length == 1 ? [nil, parts.first] : parts
      @cpu = 'x86' if @cpu&.match?(/\Ai\d86\z/)
      match = system.match(/\A(mswin32|mswin64|mingw32|mingw|netbsdelf|freebsd|openbsd|solaris|darwin|linux|cygwin|macruby|dalvik|dotnet|java|aix|wasi)(.*)\z/)
      @os = match ? match[1] : system
      @os = 'java' if @os == 'jruby'
      @version = match && match[2].sub(/\A[-_]+/, '')
      @version = case @os
                 when 'cygwin', 'mingw32', 'netbsdelf', 'wasi' then nil
                 when 'aix', 'darwin', 'freebsd', 'dalvik', 'mswin32', 'mswin64' then @version&.slice(/\A\d+/)
                 when 'openbsd', 'solaris', 'macruby', 'java', 'dotnet' then @version&.slice(/\A\d+(?:\.\d+)*/)
                 else @version
                 end
      @version = nil if @version == ''
      @cpu ||= 'x86' if @os == 'mswin32'
    end

    def to_a = [cpu, os, version]
    alias deconstruct to_a
    def to_s = to_a.compact.join(cpu ? '-' : '')
    def ==(other) = other.respond_to?(:cpu) && [cpu, os, version] == [other.cpu, other.os, other.version]
    alias eql? ==
    def hash = [cpu, os, version].hash

    def ===(other)
      return false unless other.is_a?(Platform)
      return true if [cpu, other.cpu].include?('universal') && os.start_with?('mingw') && other.os.start_with?('mingw')
      return false unless os == other.os && (!cpu || !other.cpu || cpu == other.cpu || [cpu, other.cpu].include?('universal') || (cpu == 'arm' && other.cpu.start_with?('armv')))
      return !version || !other.version || version == other.version unless os == 'linux'

      normalize = ->(v) { v.to_s.sub(/\Agnu/, '').sub(/eabi(?:hf)?\z/, '') }
      normalize.call(version) == normalize.call(other.version) || %w[musl musleabi musleabihf].any? { |libc|
        other.version == libc + version.to_s
      }
    end

    def =~(other) = self === (other.is_a?(Platform) ? other : self.class.new(other))
  end
end
