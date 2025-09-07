#!/bin/bash
# ShellSpec helper functions for Ruby Butler
# Minimal, essential utilities for sophisticated testing

# Ruby versions available in Docker environment
LATEST_RUBY="3.4.5"
OLDER_RUBY="3.2.4" 
RUBIES_DIR="/opt/rubies"

# Essential project creation for bundler testing with complete isolation
create_bundler_project() {
    local project_dir="$1"
    local ruby_version="${2:-}"
    local gemfile_ruby="${3:-}"
    
    mkdir -p "$project_dir"
    
    # Create .ruby-version if specified
    if [ -n "$ruby_version" ]; then
        echo "$ruby_version" > "$project_dir/.ruby-version"
    fi
    
    # Create Gemfile with real source for integration testing
    cat > "$project_dir/Gemfile" << EOF
source 'https://rubygems.org'

EOF
    
    if [ -n "$gemfile_ruby" ]; then
        echo "ruby '$gemfile_ruby'" >> "$project_dir/Gemfile"
        echo "" >> "$project_dir/Gemfile"
    fi
    
    # Add minimal gems for integration testing
    cat >> "$project_dir/Gemfile" << EOF
# Minimal gems for integration testing
gem 'rake'
EOF
    
    # Create isolated bundle config to prevent global pollution
    mkdir -p "$project_dir/.bundle"
    cat > "$project_dir/.bundle/config" << EOF
---
BUNDLE_PATH: "$project_dir/.bundle"
BUNDLE_DISABLE_SHARED_GEMS: "true"
BUNDLE_DEPLOYMENT: "false"
EOF
}

# Helper to create complex Gemfile with dependencies for integration testing
create_complex_gemfile() {
    local project_dir="$1"
    local ruby_version="$2"
    
    mkdir -p "$project_dir"
    # Create real Gemfile for integration testing
    cat > "$project_dir/Gemfile" << EOF
source 'https://rubygems.org'

ruby '$ruby_version'

gem 'rake', '~> 13.0'

group :development do
  gem 'bundler'
end

group :test do
  gem 'minitest'
end
EOF

    # Create isolated bundle config
    mkdir -p "$project_dir/.bundle"
    cat > "$project_dir/.bundle/config" << EOF
---
BUNDLE_PATH: "$project_dir/.bundle"
BUNDLE_DISABLE_SHARED_GEMS: "true"
BUNDLE_DEPLOYMENT: "false"
EOF
}

# Distinguished temporary directory management with complete isolation
setup_test_project() {
    # Create unique isolated test directory for parallel execution
    TEST_PROJECT_DIR=$(mktemp -d "/tmp/rb-test-$$-XXXXXXXX")
    export TEST_PROJECT_DIR
    
    # Store original working directory
    ORIGINAL_PWD=$(pwd)
    export ORIGINAL_PWD
    
    # Change to isolated test directory
    cd "$TEST_PROJECT_DIR"
    
    # Create isolated gem home for this test to prevent global pollution
    TEST_GEM_HOME="$TEST_PROJECT_DIR/.test_gems"
    mkdir -p "$TEST_GEM_HOME"
    export TEST_GEM_HOME
    
    # Store original GEM_HOME to restore later
    ORIGINAL_GEM_HOME="$GEM_HOME"
    export ORIGINAL_GEM_HOME
    
    # Set isolated gem environment for this test
    export GEM_HOME="$TEST_GEM_HOME"
    export GEM_PATH="$TEST_GEM_HOME"
}

cleanup_test_project() {
    # Return to original working directory
    if [ -n "$ORIGINAL_PWD" ]; then
        cd "$ORIGINAL_PWD"
    fi
    
    # Restore original gem environment
    if [ -n "$ORIGINAL_GEM_HOME" ]; then
        export GEM_HOME="$ORIGINAL_GEM_HOME"
        export GEM_PATH="$ORIGINAL_GEM_HOME"
    fi
    
    # Clean up test directory
    if [ -n "$TEST_PROJECT_DIR" ] && [ -d "$TEST_PROJECT_DIR" ]; then
        rm -rf "$TEST_PROJECT_DIR"
    fi
    
    # Clean up variables
    unset TEST_PROJECT_DIR
    unset TEST_GEM_HOME
    unset ORIGINAL_GEM_HOME
    unset ORIGINAL_PWD
}

# Distinguished rb command function with complete isolation
rb() {
    local cmd="/app/rb"
    "$cmd" "$@"
}
