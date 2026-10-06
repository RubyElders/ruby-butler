
raise 'Bundler gem loaded' if $LOADED_FEATURES.any? { |path| path.include?('/bundler/') }
source 'https://rubygems.org'
ruby '>= 3.2', '< 5', engine: 'jruby', engine_version: '10.0.0'
gem 'rails', '~> 8.0', '>= 8.0.2'
group :development, :test, optional: true do
  gem 'debug', require: false
  group :tools do
    gem 'rake', require: ['rake', 'rake/task']
  end
end
gem 'rack', require: 'rack/builder'
