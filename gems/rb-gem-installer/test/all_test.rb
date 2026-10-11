Dir.glob(File.join(__dir__, '*_test.rb')).each do |path|
  require path unless path == File.expand_path(__FILE__)
end
