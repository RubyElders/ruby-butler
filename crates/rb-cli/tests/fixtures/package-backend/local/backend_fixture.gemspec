Gem::Specification.new do |s|
  s.name = 'backend_fixture'
  s.version = '1.0.0'
  s.summary = 'Local package backend fixture'
  s.authors = ['Ruby Butler']
  s.files = ['lib/backend_fixture.rb', 'exe/backend_context', 'ext/backend_fixture/extconf.rb', 'ext/backend_fixture/backend_fixture_native.c']
  s.bindir = 'exe'
  s.executables = ['backend_context']
  s.extensions = ['ext/backend_fixture/extconf.rb']
end
