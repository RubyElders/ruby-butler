#!/usr/bin/env bats
# BATS Test: Basic Ruby Butler Runtime Command
# 
# This test verifies that rb runtime command works correctly
# with multiple Ruby installations in a clean environment.

# Test setup - runs before each test
setup() {
    # Ensure clean environment for each test
    export GEM_HOME_BACKUP="$GEM_HOME"
    export PATH_BACKUP="$PATH"
}

# Test teardown - runs after each test
teardown() {
    # Restore environment
    export GEM_HOME="$GEM_HOME_BACKUP"
    export PATH="$PATH_BACKUP"
}

@test "rb binary is executable and shows version" {
    run rb --version
    [ "$status" -eq 0 ]
    [[ "$output" =~ "ruby-butler" ]]
}

@test "rb runtime shows Ruby installations survey" {
    run rb runtime
    [ "$status" -eq 0 ]
    [[ "$output" =~ "💎 Ruby Environment Survey" ]]
}

@test "rb runtime detects Ruby 3.2.4 installation" {
    run rb runtime
    [ "$status" -eq 0 ]
    [[ "$output" =~ "CRuby (3.2.4)" ]]
}

@test "rb runtime detects Ruby 3.4.5 installation" {
    run rb runtime  
    [ "$status" -eq 0 ]
    [[ "$output" =~ "CRuby (3.4.5)" ]]
}

@test "rb exec can execute Ruby commands" {
    run rb exec -- ruby -e 'puts "Hello from Ruby #{RUBY_VERSION}"'
    [ "$status" -eq 0 ]
    [[ "$output" =~ "Hello from Ruby" ]]
}

@test "rb with custom gem home works correctly" {
    run rb --gem-home /tmp/test-gems exec -- ruby -e 'puts ENV["GEM_HOME"]'
    [ "$status" -eq 0 ]
    [[ "$output" =~ "/tmp/test-gems/ruby" ]]
}

@test "rb environment command shows current Ruby details" {
    run rb environment
    [ "$status" -eq 0 ]
    [[ "$output" =~ "Ruby Environment Details" ]]
}

@test "rb can select specific Ruby version" {
    run rb --ruby 3.2.4 exec -- ruby -e 'puts RUBY_VERSION'
    [ "$status" -eq 0 ]
    [[ "$output" =~ "3.2.4" ]]
}
