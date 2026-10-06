git 'outer', branch: 'main' do
 gem 'registry', source: 'https://example.test'
 gem 'local', path: 'local'
 gem 'other', git: 'other', tag: 'v1'
end