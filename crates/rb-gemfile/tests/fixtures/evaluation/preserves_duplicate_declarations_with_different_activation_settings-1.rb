
platforms :ruby do
  group :development do
    gem 'rake', '>= 13', require: false
  end
end
platforms :jruby do
  group :production do
    gem 'rake', '>= 13', require: 'rake/task'
  end
end
