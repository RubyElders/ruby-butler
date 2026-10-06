module RubyButler
  class Gemfile
    def source(url, options = {}, **keywords, &block)
      options = options.merge(keywords).transform_keys(&:to_sym)
      unknown = options.keys - [:cooldown]
      raise ArgumentError, "Unsupported source options: #{unknown.join(', ')}" unless unknown.empty?
      cooldown = options[:cooldown]
      unless cooldown.nil? || (cooldown.is_a?(Integer) && cooldown >= 0)
        raise ArgumentError, "Expected cooldown to be a non-negative integer"
      end
      if block
        scoped(source_context.merge(source: url.to_s), &block)
      else
        if @bundle[:source] && @bundle[:source] != url.to_s
          raise ArgumentError, "Multiple global sources are unsupported; use source blocks"
        end
        @bundle[:source] = url.to_s
      end
    end

    def git(url, **options, &block)
      unknown = options.keys - %i[branch tag ref rev glob submodules]
      raise ArgumentError, "Unsupported Git options: #{unknown.join(', ')}" unless unknown.empty?
      scoped(source_context.merge(options).merge(git: url.to_s), &block)
    end

    def path(directory, **options, &block)
      unknown = options.keys - [:glob]
      raise ArgumentError, "Unsupported path options: #{unknown.join(', ')}" unless unknown.empty?
      directory = File.expand_path(directory, File.dirname(@stack.last))
      scoped(source_context.merge(options).merge(path: directory), &block)
    end

    def git_source(name, &block)
      raise ArgumentError, "git_source requires a block" unless block
      @git_sources[name.to_s] = block
    end

    def github(repository, **options, &block)
      expanded = @git_sources.fetch("github").call(repository)
      expanded = { git: expanded } if expanded.is_a?(String)
      expanded = expanded.transform_keys(&:to_sym)
      raise ArgumentError, "Conflicting Git shortcut options" unless (expanded.keys & options.keys).empty?
      url = expanded.delete(:git)
      git(url, **expanded.merge(options), &block)
    end

    private

    def source_context
      %i[source git path branch tag ref rev glob submodules].to_h { |key| [key, nil] }
    end
  end
end
