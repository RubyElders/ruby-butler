#!/bin/bash
# ShellSpec helper functions for Ruby Butler

# Ruby versions available in Docker environment
LATEST_RUBY="4.0.1"
OLDER_RUBY="3.4.5"
RUBIES_DIR="/opt/rubies"

# Set RB_RUBIES_DIR for all tests so they use Docker Ruby installations
export RB_RUBIES_DIR="$RUBIES_DIR"

# Get Ruby ABI version from full version (e.g., "4.0.1" -> "4.0.0")
get_ruby_abi_version() {
    local version="$1"
    echo "$version" | sed -E 's/^([0-9]+\.[0-9]+).*/\1.0/'
}

# Create bundler project for testing
create_bundler_project() {
    local project_dir="$1"
    local ruby_version="${2:-}"
    local gemfile_ruby="${3:-}"

    mkdir -p "$project_dir"

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

    cat >> "$project_dir/Gemfile" << EOF
gem 'rake'
EOF
}


setup_test_project() {
    TEST_PROJECT_DIR=$(mktemp -d "/tmp/rb-test-$$-XXXXXXXX")
    export TEST_PROJECT_DIR

    ORIGINAL_PWD=$(pwd)
    export ORIGINAL_PWD

    cd "$TEST_PROJECT_DIR"
}

cleanup_test_project() {
    if [ -n "$ORIGINAL_PWD" ]; then
        cd "$ORIGINAL_PWD"
    fi

    if [ -n "$TEST_PROJECT_DIR" ] && [ -d "$TEST_PROJECT_DIR" ]; then
        rm -rf "$TEST_PROJECT_DIR"
    fi

    unset TEST_PROJECT_DIR
    unset ORIGINAL_PWD
}


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
