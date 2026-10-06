module Bundler
  VERSION = "2.5.0"

  def self.read_file(path)
    RubyButler.evaluator.read_file(path)
  end

  def self.load_gemspec(path, validate = false)
    RubyButler.evaluator.load_gemspec(path, validate)
  end

  module Plugin
    def self.index
      self
    end

    def self.load_paths(_name)
      []
    end
  end
end
