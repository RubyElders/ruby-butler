Gem::Specification.new do |s|
 s.name = 'sample'
 s.version = '1.0'
 s.add_dependency 'rack', '>= 3'
 s.add_dependency 'rack', '< 4'
 s.add_development_dependency 'rake', '>= 13'
 s.add_development_dependency 'rake', '< 14'
end