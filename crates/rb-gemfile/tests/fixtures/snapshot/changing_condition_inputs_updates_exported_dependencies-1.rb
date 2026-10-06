install_if -> { ENV['WITH_DEBUG'] == '1' } do
 gem 'debug', require: false
end
gem 'rake'
