
platforms :jruby do
  gem 'jdbc', force_ruby_platform: true
end
install_if -> { false } do
  gem 'disabled'
  install_if -> { true } do
    gem 'also_disabled'
  end
end
gem 'enabled', platforms: [:ruby, :windows], install_if: -> { true }
