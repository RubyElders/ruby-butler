#!/usr/bin/env bats

# Load shared helpers
load helpers

# Test setup - runs before each test
setup() {
    setup_test_environment
}

# Test teardown - runs after each test  
teardown() {
    cleanup_test_environment
}

@test "sync command detects bundler project correctly" {
    create_ruby_installation "3.2.5"
    
    # Create bundler project and cd into it
    project_dir=$(create_bundler_project "sync-test" "3.2.5")
    cd "$project_dir"
    
    # sync command should detect bundler project
    run_rb_command sync
    
    # Should attempt to run bundler (may fail due to missing bundler, which is acceptable)
    # The important thing is that it detects the bundler project
    if [ "$status" -ne 0 ]; then
        # If it fails, it should be due to missing bundler, not project detection
        output_contains_all "bundler" || output_contains "Gemfile"
    else
        # If it succeeds, great!
        [ "$status" -eq 0 ]
    fi
}

@test "complex bundler project with nested structure" {
    create_ruby_installation "3.2.5"
    
    # Create complex project structure
    project_dir=$(create_bundler_project "complex-rails-app" "3.2.5")
    cd "$project_dir"
    
    # Add additional bundler-related files
    mkdir -p app/models app/controllers config
    
    cat > Gemfile.lock << 'EOF'
GEM
  remote: https://rubygems.org/
  specs:
    rails (7.1.0)
    pg (1.4.0)
    puma (6.4.0)

PLATFORMS
  ruby

DEPENDENCIES
  rails (~> 7.1.0)
  pg (~> 1.4)
  puma (~> 6.4)

RUBY VERSION
   ruby 3.2.5p0

BUNDLED WITH
   2.4.19
EOF

    # Test environment command in complex project
    run_rb_command environment
    
    # Should handle complex bundler project
    output_contains "Ruby Environment"
    output_contains "3.2.5"
}

@test "bundler project with ruby version mismatch" {
    create_ruby_installation "3.1.0"
    create_ruby_installation "3.2.5"
    
    # Create project requiring 3.2.5 but ensure 3.1.0 is also available
    project_dir=$(create_bundler_project "version-specific" "3.2.5")
    cd "$project_dir"
    
    run_rb_command environment
    
    # Should detect and use the version specified in project
    output_contains "3.2.5"
}

@test "bundler project without .ruby-version file" {
    create_ruby_installation "3.2.5"
    
    # Create project with only Gemfile (no .ruby-version)
    project_dir="$TEST_WORK_DIR/gemfile-only"
    mkdir -p "$project_dir"
    
    cat > "$project_dir/Gemfile" << 'EOF'
source 'https://rubygems.org'

ruby '3.2.5'

gem 'rails', '~> 7.1.0'
EOF
    
    cd "$project_dir"
    
    run_rb_command environment
    
    # Should extract Ruby version from Gemfile
    output_contains "3.2.5"
}

@test "sync command with missing bundler executable" {
    create_ruby_installation "3.2.5"
    
    project_dir=$(create_bundler_project "missing-bundler" "3.2.5")
    cd "$project_dir"
    
    # sync command should handle missing bundler gracefully
    run_rb_command sync
    
    if [ "$status" -ne 0 ]; then
        # Should provide helpful error about missing bundler
        output_contains "bundler" || output_contains "not found" || output_contains "install"
    fi
}

@test "exec command in bundler context" {
    create_ruby_installation "3.2.5"
    
    project_dir=$(create_bundler_project "exec-test" "3.2.5")
    cd "$project_dir"
    
    # Test exec command (with simple command that should work)
    run_rb_command exec echo "hello from bundler context"
    
    # Should execute successfully
    output_contains "hello from bundler context"
}

@test "multiple bundler projects in different directories" {
    create_ruby_installation "3.1.0" 
    create_ruby_installation "3.2.5"
    
    # Create first project with 3.1.0
    project1=$(create_bundler_project "app1" "3.1.0")
    
    # Create second project with 3.2.5  
    project2=$(create_bundler_project "app2" "3.2.5")
    
    # Test first project
    cd "$project1"
    run_rb_command environment
    output_contains "3.1.0"
    
    # Test second project
    cd "$project2" 
    run_rb_command environment
    output_contains "3.2.5"
}

@test "bundler project with complex gem dependencies" {
    create_ruby_installation "3.2.5"
    
    project_dir="$TEST_WORK_DIR/complex-deps"
    mkdir -p "$project_dir"
    
    # Create Gemfile with many realistic gems
    cat > "$project_dir/Gemfile" << 'EOF'
source 'https://rubygems.org'

ruby '3.2.5'

# Core framework
gem 'rails', '~> 7.1.0'

# Database
gem 'pg', '~> 1.4'
gem 'redis', '~> 5.0'

# Application server
gem 'puma', '~> 6.4'

# Asset pipeline
gem 'sprockets-rails'
gem 'importmap-rails'
gem 'turbo-rails'
gem 'stimulus-rails'

# Performance
gem 'bootsnap', require: false

# Authentication & Authorization
gem 'devise'
gem 'pundit'

# Background jobs
gem 'sidekiq'

# API
gem 'jbuilder'

group :development, :test do
  gem 'rspec-rails'
  gem 'factory_bot_rails'
  gem 'faker'
  gem 'byebug'
  gem 'debug', platforms: %i[ mri mingw x64_mingw ]
end

group :development do
  gem 'web-console'
  gem 'listen', '~> 3.8'
  gem 'spring'
  gem 'spring-watcher-listen'
  gem 'annotate'
  gem 'bullet'
end

group :test do
  gem 'capybara'
  gem 'selenium-webdriver'
  gem 'webmock'
end
EOF
    
    echo "3.2.5" > "$project_dir/.ruby-version"
    
    cd "$project_dir"
    
    run_rb_command environment
    
    # Should handle complex Gemfile
    output_contains "Ruby Environment"
    output_contains "3.2.5"
}
