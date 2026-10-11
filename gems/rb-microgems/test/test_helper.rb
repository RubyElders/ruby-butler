require 'minitest/autorun'
require 'tmpdir'
require 'fileutils'
require 'open3'
require_relative '../lib/microgems'

module RubyProcess
  LIB = File.expand_path('../lib', __dir__)

  def ruby_fixture(name, *arguments, environment: {}, compatibility: false)
    flags = compatibility ? ['--disable=gems', '-I', LIB] : []
    Open3.capture3(environment, RbConfig.ruby, *flags, File.join(__dir__, 'fixtures', name), *arguments)
  end
end
