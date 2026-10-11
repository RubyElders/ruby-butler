require 'open3'
require 'shellwords'

module MicroGems
  module Ext
    class Builder
      class NoMakefileError < MicroGems::InstallError; end
      class << self
        def shellsplit(value) = Shellwords.split(value)
        alias shellsplit_command shellsplit
        def ruby = [RbConfig.ruby]

        def run(command, results, name = nil, directory = Dir.pwd, environment = {})
          raise ArgumentError, 'Build command must be an argv array' unless command.is_a?(Array) && !command.empty?

          results << "current directory: #{directory}" << Shellwords.join(command)
          output = +''
          status = nil
          Open3.popen2e(environment, *command, chdir: directory) do |input, stream, wait|
            input.close
            loop do
              chunk = stream.readpartial(8192)
              $stdout.write(chunk)
              $stdout.flush
              output << chunk
              output = output.byteslice(-1_048_576, 1_048_576) if output.bytesize > 1_048_576
            rescue EOFError
              break
            end
            status = wait.value
          end
          results << output
          raise MicroGems::InstallError, "#{name || command.first} failed (#{status}): #{output}" unless status.success?

          results
        end
      end
    end
  end
end
