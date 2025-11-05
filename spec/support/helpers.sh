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
}

cleanup_test_project() {
    # Return to original working directory
    if [ -n "$ORIGINAL_PWD" ]; then
        cd "$ORIGINAL_PWD"
    fi

    # Clean up test directory
    if [ -n "$TEST_PROJECT_DIR" ] && [ -d "$TEST_PROJECT_DIR" ]; then
        rm -rf "$TEST_PROJECT_DIR"
    fi

    # Clean up variables
    unset TEST_PROJECT_DIR
    unset ORIGINAL_PWD
}

# Distinguished rb command function with complete isolation
rb() {
    # Use local build if available (for development), otherwise Docker path
    if [ -n "$RB_TEST_BINARY" ] && [ -f "$RB_TEST_BINARY" ]; then
        "$RB_TEST_BINARY" "$@"
    elif [ -f "/app/rb" ]; then
        "/app/rb" "$@"
    elif [ -f "./target/debug/rb" ]; then
        "./target/debug/rb" "$@"
    else
        # Fallback to PATH
        command rb "$@"
    fi
}

# Check if Ruby is available for testing
is_ruby_available() {
    command -v ruby >/dev/null 2>&1
}

# Check if Bundler is available for testing
is_bundler_available() {
    command -v bundle >/dev/null 2>&1
}
