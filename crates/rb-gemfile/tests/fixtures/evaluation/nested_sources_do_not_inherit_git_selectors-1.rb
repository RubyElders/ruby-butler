git 'outer', branch: 'main', submodules: true do
 source 'https://example.test' do
 gem 'registry'
 end
 path 'local' do
 gem 'local'
 end
 git 'inner', tag: 'v1' do
 gem 'inner'
 end
 gem 'outer'
end