#!/usr/bin/env bats
# Shared helpers for command-specific tests

# Ruby versions available in Docker
export LATEST_RUBY="3.4.5"
export OLDER_RUBY="3.2.4"
export RUBIES_DIR="/opt/rubies"

# Helper to run rb command and capture output
run_rb() {
    run /app/rb "$@"
}

# Helper to check if output contains a string
output_contains() {
    local expected="$1"
    [[ "$output" == *"$expected"* ]]
}

# Helper to check if output does NOT contain a string
output_not_contains() {
    local expected="$1"
    [[ "$output" != *"$expected"* ]]
}

# Helper to create a temporary bundler project
create_bundler_project() {
    local project_dir="$1"
    local ruby_version="$2"
    local gemfile_ruby="$3"
    
    mkdir -p "$project_dir"
    cd "$project_dir"
    
    # Create Gemfile
    cat > Gemfile << EOF
source 'https://rubygems.org'
EOF
    
    # Add ruby version to Gemfile if specified
    if [ -n "$gemfile_ruby" ]; then
        echo "" >> Gemfile
        echo "ruby '$gemfile_ruby'" >> Gemfile
    fi
    
    # Add a simple gem
    echo "" >> Gemfile
    echo "gem 'ABO'" >> Gemfile
    
    # Create .ruby-version file if ruby_version is specified
    if [ -n "$ruby_version" ]; then
        echo "$ruby_version" > .ruby-version
    fi
}

# Setup function for tests that need temporary directories
setup_temp_dir() {
    export TEST_PROJECT_DIR="$(mktemp -d)"
}

# Teardown function to clean up temporary directories
teardown_temp_dir() {
    if [ -n "$TEST_PROJECT_DIR" ] && [ -d "$TEST_PROJECT_DIR" ]; then
        rm -rf "$TEST_PROJECT_DIR"
    fi
}
