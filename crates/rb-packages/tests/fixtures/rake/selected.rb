require 'tool-helper'

module Rake
  VERSION = TOOL_VERSION

  def self.application
    self
  end

  def self.run
    ARGV.grep(/\ARUBY(?:ARCH|LIB)DIR=/).each do |argument|
      key, value = argument.split('=', 2)
      ENV[key] = value
    end
    load ARGV.fetch(ARGV.index('-f') + 1)
  end
end

def task(*)
  yield
end
