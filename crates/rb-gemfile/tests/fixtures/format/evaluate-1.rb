
source 'https://rubygems.org'
gem 'rails', '~> 8.0'
gem 'rake'
group :development do
  gem 'debug', require: false
end
source 'https://gems.example.com' do
  gem 'internal', '~> 2.0'
  gem 'another', require: false
end
gem 'git-gem', git: 'https://github.com/example/gem', tag: 'v1.0'
gem 'a.b', '~> 1.0'
install_if -> { false } do
  gem 'excluded'
end
