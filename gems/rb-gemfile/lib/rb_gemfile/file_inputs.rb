require 'rbconfig'

module RubyButler
  class FileInputs
    KEY = :rb_gemfile_file_inputs

    attr_reader :paths

    def initialize(paths = [])
      @paths = paths
      @excluded = [File.expand_path('..', RbConfig::CONFIG.fetch('bindir')), File.expand_path('..', __dir__)]
      @excluded.concat(Gem.path.map { |path| File.expand_path(path) }) if defined?(Gem)
    end

    def capture
      previous = Thread.current[KEY]
      Thread.current[KEY] = self
      trace = TracePoint.new(:script_compiled) { |event| record(event.instruction_sequence&.path) }
      trace.enable(target_thread: Thread.current) { yield }
    ensure
      Thread.current[KEY] = previous
    end

    def record(path)
      return unless path.is_a?(String) || path.respond_to?(:to_path)

      path = File.expand_path(path)
      return if @excluded.any? { |directory| path.start_with?("#{directory}/") } || !File.file?(path)

      @paths << path unless @paths.include?(path)
    end

    Hooks = Module.new do
      %i[read binread readlines foreach].each do |method|
        define_method(method) do |path, *arguments, **options, &block|
          Thread.current[FileInputs::KEY]&.record(path)
          super(path, *arguments, **options, &block)
        end
      end

      def open(path, *arguments, **options, &block)
        mode = options.fetch(:mode, arguments.first || 'r')
        reading = mode.is_a?(Integer) ? mode & File::WRONLY == 0 : mode.to_s.match?(/[r+]/)
        Thread.current[FileInputs::KEY]&.record(path) if reading
        super
      end
    end
    File.singleton_class.prepend(Hooks)
  end
end
