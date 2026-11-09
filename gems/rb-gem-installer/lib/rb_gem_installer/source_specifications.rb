module RubyButler
  module GemInstaller
    module SourceSpecifications
      DEFAULT_GLOB = "{,*,*/*}.gemspec".freeze

      def self.export(request_path, tracker)
        require 'json'
        request = JSON.parse(File.read(request_path))
        specifications = tracker.capture { read(request.fetch('directory'), request.fetch('glob')) }
        specifications.each { |specification| specification[:inputs] = tracker.paths }
        File.write(request.fetch('output'), JSON.generate(specifications))
      end

      def self.read(directory, glob = DEFAULT_GLOB)
        require 'rubygems'
        root = File.realpath(directory)
        Dir.glob(glob, base: root).sort.map do |path|
          absolute = File.realpath(File.join(root, path))
          raise "Gemspec escapes source directory" unless absolute.start_with?("#{root}/")

          specification = Dir.chdir(File.dirname(absolute)) { Gem::Specification.load(absolute) }
          raise "Unable to load #{path}" unless specification

          { gemspec: absolute.delete_prefix("#{root}/"), metadata: specification.to_yaml, specification: specification.to_ruby,
            files: specification.files, require_paths: specification.raw_require_paths, extensions: specification.extensions }
        end
      end
    end
  end
end
