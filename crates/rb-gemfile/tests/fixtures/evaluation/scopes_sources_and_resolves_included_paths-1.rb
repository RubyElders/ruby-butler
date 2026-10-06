
source 'https://rubygems.org'
source 'https://gems.example.test' do
  gem 'private-gem'
end
git 'https://example.test/repo.git', ref: 'abc123', submodules: true do
  gem 'git-gem'
end
git_source(:company) { |name| "https://example.test/#{name}.git" }
gem 'custom', company: 'custom', tag: 'v1'
eval_gemfile 'nested/Gemfile'
gem 'normal'
