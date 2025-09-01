#!/usr/bin/env bats
# BATS Test: Basic Ruby Butler Runtime Command
# 
# This test verifies that rb runtime command works correctly
# with multiple Ruby installations in a clean environment.

# Test setup - runs before each test
setup() {
    # Create unique directories for this test run to enable parallel execution
    export TEST_ROOT_DIR="$(mktemp -d)"
    export TEST_GEM_HOME="$TEST_ROOT_DIR/gems"
    export TEST_WORK_DIR="$TEST_ROOT_DIR/work"
    
    # Create directories
    mkdir -p "$TEST_GEM_HOME" "$TEST_WORK_DIR"
    
    # Change to test working directory
    cd "$TEST_WORK_DIR"
    
    # Backup original environment
    export GEM_HOME_BACKUP="$GEM_HOME"
    export PATH_BACKUP="$PATH"
    export RUBIES_DIR_BACKUP="$RUBIES_DIR"
    export PWD_BACKUP="$PWD"
    
    # Set test environment
    export RUBIES_DIR="/opt/rubies"
}

# Test teardown - runs after each test
teardown() {
    # Return to original directory
    cd "$PWD_BACKUP" 2>/dev/null || cd /app
    
    # Clean up test directories
    rm -rf "$TEST_ROOT_DIR"
    
    # Restore environment
    export GEM_HOME="$GEM_HOME_BACKUP"
    export PATH="$PATH_BACKUP"
    export RUBIES_DIR="$RUBIES_DIR_BACKUP"
}

@test "rb binary is executable and shows version" {
    run rb --version
    [ "$status" -eq 0 ]
    [[ "$output" =~ "rb 0.1.0" ]]
}

@test "rb runtime shows Ruby installations survey" {
    run rb --rubies-dir /opt/rubies --gem-home "$TEST_GEM_HOME" runtime
    [ "$status" -eq 0 ]
    [[ "$output" =~ "💎 Ruby Environment Survey" ]]
}

@test "rb runtime detects Ruby 3.2.4 installation" {
    run rb --rubies-dir /opt/rubies --gem-home "$TEST_GEM_HOME" runtime
    [ "$status" -eq 0 ]
    [[ "$output" =~ "CRuby (3.2.4)" ]]
}

@test "rb runtime detects Ruby 3.4.5 installation" {
    run rb --rubies-dir /opt/rubies --gem-home "$TEST_GEM_HOME" runtime  
    [ "$status" -eq 0 ]
    [[ "$output" =~ "CRuby (3.4.5)" ]]
}

@test "rb exec can execute Ruby commands" {
    run rb --rubies-dir /opt/rubies --gem-home "$TEST_GEM_HOME" exec -- ruby -e 'puts "Hello from Ruby #{RUBY_VERSION}"'
    [ "$status" -eq 0 ]
    [[ "$output" =~ "Hello from Ruby" ]]
}

@test "rb with custom gem home works correctly" {
    run rb --rubies-dir /opt/rubies --gem-home "$TEST_GEM_HOME" exec -- ruby -e 'puts ENV["GEM_HOME"]'
    [ "$status" -eq 0 ]
    [[ "$output" =~ "$TEST_GEM_HOME/ruby" ]]
}

@test "rb environment command shows current Ruby details" {
    run rb --rubies-dir /opt/rubies --gem-home "$TEST_GEM_HOME" environment
    [ "$status" -eq 0 ]
    [[ "$output" =~ "Your Current Ruby Environment" ]]
}

@test "rb can select specific Ruby version" {
    run rb --rubies-dir /opt/rubies --gem-home "$TEST_GEM_HOME" --ruby 3.2.4 exec -- ruby -e 'puts RUBY_VERSION'
    [ "$status" -eq 0 ]
    [[ "$output" =~ "3.2.4" ]]
}
