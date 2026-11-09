require 'minitest/autorun'
require 'tmpdir'
require_relative '../lib/rb_gemfile/file_inputs'

class FileInputsTest < Minitest::Test
  def test_records_loaded_code_and_data_reads
    Dir.mktmpdir do |directory|
      helper = File.join(directory, 'helper.rb')
      loaded = File.join(directory, 'loaded.rb')
      data = File.join(directory, 'data')
      File.write(helper, "File.binread(#{data.inspect})\n")
      File.write(loaded, "File.open(#{data.inspect}, 'r') { |file| file.read }\n")
      File.write(data, 'input')
      inputs = RubyButler::FileInputs.new
      inputs.capture do
        require helper
        load loaded
        File.readlines(data)
        File.read(RbConfig.const_source_location(:CONFIG).first)
      end
      assert_equal [helper, data, loaded], inputs.paths
      refute(inputs.paths.any? { |path| path.start_with?(RbConfig::CONFIG.fetch('rubylibdir')) })
    end
  end

  def test_scope_is_restored_after_failure_and_write_only_files_are_not_inputs
    Dir.mktmpdir do |directory|
      file = File.join(directory, 'data')
      File.write(file, 'input')
      inputs = RubyButler::FileInputs.new
      assert_raises(RuntimeError) do
        inputs.capture do
          File.open(file, 'w') { |output| output.write('output') }
          raise 'stop'
        end
      end
      File.read(file)
      assert_empty inputs.paths
    end
  end
end
