module RubyButler
  class EnvironmentInputs
    KEY = :rb_gemfile_environment_inputs
    KEYED = %i[[] fetch key? include? has_key? member? assoc].freeze

    attr_reader :names

    def initialize
      @names = []
    end

    def capture
      previous = Thread.current[KEY]
      Thread.current[KEY] = self
      yield
    ensure
      Thread.current[KEY] = previous
    end

    def record(method, arguments)
      if KEYED.include?(method)
        name = arguments.first.to_s
        @names << name if @names && !@names.include?(name)
      elsif method == :values_at
        arguments.each { |name| record(:[], [name]) }
      else
        @names = nil
      end
    end

    Hooks = Module.new do
      ENV.singleton_methods.each do |method|
        define_method(method) do |*arguments, &block|
          Thread.current[EnvironmentInputs::KEY]&.record(method, arguments)
          super(*arguments, &block)
        end
      end
    end
    ENV.singleton_class.prepend(Hooks)
  end
end
