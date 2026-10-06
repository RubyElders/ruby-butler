Gem::Specification.new do |s|
 s.name = 'sample'
 s.version = '1.2.3'
 s.summary = 'Sample'
 s.authors = ['Example']
 s.required_ruby_version = '>= 3.2'
 s.add_dependency 'rack', '>= 3', '< 4'
 s.add_development_dependency 'rake', '~> 13'
end