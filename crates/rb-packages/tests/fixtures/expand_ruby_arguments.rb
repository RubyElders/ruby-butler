#!/usr/bin/env ruby
require 'rbconfig'

arguments = ARGV.flat_map do |argument|
  matches = Dir.glob(argument) if argument.include?('*')
  matches && !matches.empty? ? matches.sort : [argument]
end
exec ENV.fetch('RB_GEM_BUILD_RUBY', RbConfig.ruby), *arguments
