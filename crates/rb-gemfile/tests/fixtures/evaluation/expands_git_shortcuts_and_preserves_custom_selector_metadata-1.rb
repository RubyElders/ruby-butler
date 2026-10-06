
gem 'github', github: 'rack'
gem 'gitlab', gitlab: 'group/repo'
gem 'gist', gist: '1234'
gem 'bitbucket', bitbucket: 'owner'
git_source(:review) { |name| { 'git' => "https://example.test/#{name}", 'ref' => 'refs/pull/1/head' } }
gem 'review', review: 'repo'
github 'rails/rails', branch: 'main' do
  gem 'rails'
end
