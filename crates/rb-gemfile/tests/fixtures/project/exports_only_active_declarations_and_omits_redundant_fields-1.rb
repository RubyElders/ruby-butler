
source 'https://rubygems.org'
install_if -> { false } do
  gem 'excluded'
  group :test do
    gem 'mixed', require: false
  end
end
group :development do
  gem 'mixed', require: 'mixed/runtime'
end
install_if -> { false } do
  group :test do
    gem 'active_first'
  end
end
gem 'active_first'
gem 'retained', require: false
platforms :jruby do
  gem 'retained', require: 'retained/java'
end
