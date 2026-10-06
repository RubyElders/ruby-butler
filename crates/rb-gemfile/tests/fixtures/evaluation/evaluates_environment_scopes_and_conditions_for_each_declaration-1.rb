
ENV.delete('RB_TEST_MISSING')
ENV['RB_TEST_FEATURE'] = 'yes'
env 'RB_TEST_FEATURE' do
  gem 'present'
end
env 'RB_TEST_MISSING' do
  gem 'missing'
end
env 'RB_TEST_FEATURE' => /^y/ do
  gem 'matching'
end
count = 0
install_if -> { count += 1; count == 1 } do
  gem 'first'
  gem 'second'
end
gem 'all', install_if: [-> { true }, -> { false }]
