git 'outer', branch: 'main' do
  source 'https://example.test' do
    gem 'registry'
  end
  gem 'outer'
end
gem 'normal'
