require "minitest/autorun"
require_relative '../lib/rb_gemfile'

class EnvironmentInputsTest < Minitest::Test
  def test_records_named_reads_without_recording_unrelated_environment
    inputs = RubyButler::EnvironmentInputs.new
    inputs.capture do
      ENV['RB_TEST_FIRST']
      ENV.fetch('RB_TEST_SECOND', nil)
      ENV.key?('RB_TEST_THIRD')
      ENV.values_at('RB_TEST_FIRST', 'RB_TEST_FOURTH')
    end
    assert_equal %w[RB_TEST_FIRST RB_TEST_SECOND RB_TEST_THIRD RB_TEST_FOURTH], inputs.names
  end

  def test_enumeration_tracks_the_whole_environment
    inputs = RubyButler::EnvironmentInputs.new
    inputs.capture { ENV.to_h }
    assert_nil inputs.names
  end

  def test_tracking_does_not_escape_the_evaluation_scope
    inputs = RubyButler::EnvironmentInputs.new
    assert_raises(RuntimeError) { inputs.capture { raise 'stop' } }
    ENV['RB_TEST_OUTSIDE']
    assert_empty inputs.names
  end
end
