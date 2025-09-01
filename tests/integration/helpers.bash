#!/usr/bin/env bats
# Shared test helpers for Ruby Butler BATS tests

# Set up base test environment
setup_test_environment() {
    # Create unique directories for this test run to enable parallel execution
    export TEST_ROOT_DIR="$(mktemp -d)"
    export TEST_GEM_HOME="$TEST_ROOT_DIR/gems"
    export TEST_WORK_DIR="$TEST_ROOT_DIR/work"
    export RUBIES_DIR="$TEST_ROOT_DIR/rubies"
    
    # Create directories
    mkdir -p "$TEST_GEM_HOME" "$TEST_WORK_DIR" "$RUBIES_DIR"
    
    # Change to test working directory
    cd "$TEST_WORK_DIR"
    
    # Backup original environment
    export ORIGINAL_GEM_HOME="${GEM_HOME:-}"
    export ORIGINAL_GEM_PATH="${GEM_PATH:-}"
    export ORIGINAL_PATH="${PATH:-}"
}

# Clean up test environment
cleanup_test_environment() {
    # Restore original environment
    export GEM_HOME="$ORIGINAL_GEM_HOME"
    export GEM_PATH="$ORIGINAL_GEM_PATH" 
    export PATH="$ORIGINAL_PATH"
    
    # Clean up test directories
    if [ -n "$TEST_ROOT_DIR" ] && [ -d "$TEST_ROOT_DIR" ]; then
        rm -rf "$TEST_ROOT_DIR"
    fi
}

# Create a Ruby installation directory structure
create_ruby_installation() {
    local version="$1"
    local ruby_dir="$RUBIES_DIR/ruby-$version"
    
    mkdir -p "$ruby_dir/bin"
    mkdir -p "$ruby_dir/lib/ruby/$version"
    
    # Create a mock ruby executable
    cat > "$ruby_dir/bin/ruby" << EOF
#!/bin/bash
echo "ruby $version"
EOF
    chmod +x "$ruby_dir/bin/ruby"
    
    echo "$ruby_dir"
}

# Create a bundler project with Gemfile
create_bundler_project() {
    local project_name="$1"
    local ruby_version="$2"
    local project_dir="$TEST_WORK_DIR/$project_name"
    
    mkdir -p "$project_dir"
    
    # Create Gemfile
    cat > "$project_dir/Gemfile" << EOF
source 'https://rubygems.org'

ruby '$ruby_version'

gem 'rails', '~> 7.1.0'
gem 'pg', '~> 1.4'
gem 'puma', '~> 6.4'
gem 'redis', '~> 5.0'
gem 'bootsnap', require: false

group :development, :test do
  gem 'rspec-rails'
  gem 'factory_bot_rails'
  gem 'byebug'
end

group :development do
  gem 'listen', '~> 3.8'
  gem 'spring'
  gem 'spring-watcher-listen'
end
EOF

    # Create .ruby-version file if version specified
    if [ -n "$ruby_version" ]; then
        echo "$ruby_version" > "$project_dir/.ruby-version"
    fi
    
    echo "$project_dir"
}

# Run rb command with test configuration
run_rb_command() {
    run /app/rb --rubies-dir "$RUBIES_DIR" --gem-home "$TEST_GEM_HOME" "$@"
}

# Run rb command and expect success
run_rb_success() {
    run_rb_command "$@"
    [ "$status" -eq 0 ]
}

# Run rb command and expect failure
run_rb_failure() {
    run_rb_command "$@"
    [ "$status" -ne 0 ]
}

# Check if output contains text (case insensitive)
output_contains() {
    local expected="$1"
    echo "$output" | grep -qi "$expected"
}

# Check if output contains all provided texts
output_contains_all() {
    local result=0
    for text in "$@"; do
        if ! output_contains "$text"; then
            echo "Expected output to contain: $text"
            echo "Actual output:"
            echo "$output"
            result=1
        fi
    done
    return $result
}

# Debug helper - print current environment and output
debug_test_state() {
    echo "=== TEST DEBUG INFO ==="
    echo "TEST_ROOT_DIR: $TEST_ROOT_DIR"
    echo "TEST_GEM_HOME: $TEST_GEM_HOME"
    echo "TEST_WORK_DIR: $TEST_WORK_DIR"
    echo "RUBIES_DIR: $RUBIES_DIR"
    echo "Current directory: $(pwd)"
    echo "Command status: $status"
    echo "Command output:"
    echo "$output"
    echo "======================="
}
