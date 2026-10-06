module RubyButler
  class Gemfile
    def ruby(*versions, **options)
      options = versions.pop.merge(options) if versions.last.is_a?(Hash)
      options = options.transform_keys(&:to_sym)
      unknown = options.keys - %i[engine engine_version file patchlevel]
      raise ArgumentError, "Unsupported ruby options: #{unknown.join(', ')}" unless unknown.empty?
      engine, engine_version, file, patchlevel = options.values_at(:engine, :engine_version, :file, :patchlevel)
      if file
        raise ArgumentError, "ruby accepts versions or file, not both" unless versions.empty?
        location = File.expand_path(file, File.dirname(@stack.last))
        @inputs << location unless @inputs.include?(location)
        text = read_file(location)
        match = /^ruby[\s-]*(?:=\s*)?(?:"([^"]+)"|'([^']+)'|([^\s#"']+))/.match(text)
        versions = [match ? match.captures.compact.first : text.strip]
      end
      raise ArgumentError, "ruby requires a version" if versions.empty?
      versions = versions.flatten
      Gem::Requirement.new(versions)
      raise ArgumentError, "engine and engine_version must be specified together" if engine.nil? != engine_version.nil?
      if engine == "ruby" && engine_version && versions != Array(engine_version)
        raise ArgumentError, "Ruby requirements must match engine_version for MRI"
      end
      @bundle.merge!(ruby: versions.map(&:to_s), engine: engine, engine_version: engine_version, patchlevel: patchlevel&.to_s)
    end

    def gemspec(options = {}, **keywords)
      options = options.merge(keywords).transform_keys(&:to_sym)
      unknown = options.keys - %i[path name glob development_group require]
      raise ArgumentError, "Unsupported gemspec options: #{unknown.join(', ')}" unless unknown.empty?
      path = options.fetch(:path, ".")
      name = options[:name]
      glob = options[:glob]
      development_group = options.fetch(:development_group, :development)
      directory = File.expand_path(path, File.dirname(@stack.last))
      specs = Dir.glob(File.join(directory, "{*,.*}.gemspec")).uniq.map { |file| load_gemspec(file) }
      specs.select! { |spec| spec.name == name } if name
      groups = specs.group_by { |spec| [spec.name, spec.version] }
      raise ArgumentError, "Expected one gemspec in #{directory}, found #{groups.length}" unless groups.length == 1
      choices = groups.values.first
      spec = choices.find { |candidate| candidate.platform.to_s == Gem::Platform.local.to_s } ||
             choices.find { |candidate| candidate.platform.to_s == "ruby" } || choices.first
      @gemspecs << {
        name: spec.name, version: spec.version.to_s, platform: spec.platform.to_s, path: directory,
        required_ruby_version: spec.required_ruby_version.as_list,
        required_rubygems_version: spec.required_rubygems_version.as_list,
        runtime_dependencies: package_dependencies(spec.runtime_dependencies),
        development_dependencies: package_dependencies(spec.development_dependencies)
      }
      source = { path: directory }
      source[:glob] = glob if glob
      gem(spec.name, "= #{spec.version}", **source)
      spec.development_dependencies.each do |dependency|
        if (existing = @declarations[dependency.name])
          requirements = (existing[:version] + dependency.requirement.as_list).uniq
          existing[:version] = requirements
          Array(existing[:variants]).each { |variant| variant[:version] = requirements }
          refresh_dependency(dependency.name)
        else
          gem(dependency.name, *dependency.requirement.as_list, group: development_group)
          @gemspec_development[dependency.name] = true
        end
      end
    end

    def load_gemspec(path, validate = false)
      path = File.expand_path(path)
      contents = read_file(path)
      spec = Dir.chdir(File.dirname(path)) do
        if contents.start_with?("---")
          Gem::Specification.from_yaml(contents)
        else
          eval(contents, TOPLEVEL_BINDING.dup, path, 1)
        end
      end
      raise ArgumentError, "Unable to evaluate #{path}" unless spec.is_a?(Gem::Specification)
      spec.loaded_from = path
      spec.validate if validate
      spec
    end

    private

    def package_dependencies(dependencies)
      dependencies.group_by(&:name).transform_values do |declarations|
        declarations.flat_map { |dependency| dependency.requirement.as_list }.uniq
      end
    end
  end
end
