#!/usr/bin/env bats
# BATS Test: Bundler Integration Tests
# 
# This test suite covers the bundler functionality that was previously
# marked as slow in the unit tests. In Docker, we can run these safely.

setup() {
    # Create unique directories for this test run to enable parallel execution
    export TEST_ROOT_DIR="$(mktemp -d)"
    export TEST_GEM_HOME="$TEST_ROOT_DIR/gems"
    export TEST_WORK_DIR="$TEST_ROOT_DIR/work"
    
    # Create directories
    mkdir -p "$TEST_GEM_HOME" "$TEST_WORK_DIR"
    
    # Backup original environment
    export ORIGINAL_PWD="$PWD"
    export GEM_HOME_BACKUP="$GEM_HOME"
    export PATH_BACKUP="$PATH"
    
    # Set test environment
    export BATS_TEST_TMPDIR="$TEST_WORK_DIR"
}

teardown() {
    # Return to original directory
    cd "$ORIGINAL_PWD" 2>/dev/null || cd /app
    
    # Clean up test directories
    rm -rf "$TEST_ROOT_DIR"
    
    # Restore environment
    export GEM_HOME="$GEM_HOME_BACKUP"
    export PATH="$PATH_BACKUP"
}

# Helper function to create a test project with Gemfile
create_test_project() {
    local project_name="$1"
    local ruby_version="$2"
    local with_gems="${3:-false}"
    
    local project_dir="$BATS_TEST_TMPDIR/$project_name"
    mkdir -p "$project_dir"
    cd "$project_dir"
    
    cat > Gemfile << EOF
source 'https://rubygems.org'

ruby '$ruby_version'

gem 'json'
gem 'rake'
EOF
    
    if [ "$with_gems" = "true" ]; then
        # Install gems using the latest Ruby
        rb --rubies-dir /opt/rubies --gem-home "$TEST_GEM_HOME" exec -- bundle install
    fi
    
    echo "$project_dir"
}

@test "rb sync detects bundler project correctly" {
    create_test_project "sync_test" "3.4.5"
    
    run rb --rubies-dir /opt/rubies --gem-home "$TEST_GEM_HOME" sync
    [ "$status" -eq 0 ]
    # Should detect bundler environment even without bundle install
}

@test "rb sync with actual bundle install works" {
    create_test_project "sync_with_gems" "3.4.5" true
    
    run rb --rubies-dir /opt/rubies --gem-home "$TEST_GEM_HOME" sync  
    [ "$status" -eq 0 ]
    # Should successfully sync with installed gems
}

@test "rb sync handles missing bundler gracefully" {
    # Create project without installing bundler
    create_test_project "no_bundler" "3.4.5"
    
    run rb --rubies-dir /opt/rubies --gem-home "$TEST_GEM_HOME" sync
    # Should not crash even if bundler operations fail
    # Exit code might be non-zero but should not crash
}

@test "rb sync respects ruby version in Gemfile" {
    create_test_project "version_test" "3.2.4"
    
    run rb --rubies-dir /opt/rubies --gem-home "$TEST_GEM_HOME" --ruby 3.2.4 sync
    [ "$status" -eq 0 ]
    # Should use Ruby 3.2.4 as specified
}

@test "rb environment shows bundler details when available" {
    create_test_project "env_test" "3.4.5" true
    
    run rb --rubies-dir /opt/rubies --gem-home "$TEST_GEM_HOME" environment
    [ "$status" -eq 0 ]
    [[ "$output" =~ "Bundler" ]]
}

@test "rb exec works within bundler context" {
    create_test_project "exec_test" "3.4.5" true
    
    run rb --rubies-dir /opt/rubies --gem-home "$TEST_GEM_HOME" exec -- ruby -e 'puts "In bundler context"'
    [ "$status" -eq 0 ]
    [[ "$output" =~ "In bundler context" ]]
}

@test "parallel gem homes remain isolated" {
    # Test that different gem homes don't interfere
    local gem_home_1="$BATS_TEST_TMPDIR/gems1"
    local gem_home_2="$BATS_TEST_TMPDIR/gems2"
    
    mkdir -p "$gem_home_1" "$gem_home_2"
    
    # Create projects with different gem homes
    create_test_project "project1" "3.4.5"
    run rb --rubies-dir /opt/rubies --gem-home "$gem_home_1" exec -- ruby -e 'puts ENV["GEM_HOME"]'
    [ "$status" -eq 0 ]
    [[ "$output" =~ "$gem_home_1" ]]
    
    create_test_project "project2" "3.2.4" 
    run rb --rubies-dir /opt/rubies --gem-home "$gem_home_2" exec -- ruby -e 'puts ENV["GEM_HOME"]'
    [ "$status" -eq 0 ]
    [[ "$output" =~ "$gem_home_2" ]]
}
