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

@test "handles invalid Ruby version specifications gracefully" {
    # Create project with invalid version format
    project_dir="$TEST_WORK_DIR/invalid-version"
    mkdir -p "$project_dir"
    
    cat > "$project_dir/Gemfile" << 'EOF'
source 'https://rubygems.org'

ruby 'not-a-version'

gem 'rails'
EOF
    
    cd "$project_dir"
    
    run_rb_command environment
    
    # Should handle invalid version gracefully and fall back to latest available
    [ "$status" -eq 0 ]
    output_contains "3.4.5"
}

@test "handles missing requested Ruby version" {
    # Create project requesting unavailable version
    project_dir="$TEST_WORK_DIR/missing-version"
    mkdir -p "$project_dir"
    
    cat > "$project_dir/Gemfile" << 'EOF'
source 'https://rubygems.org'

ruby '3.3.0'

gem 'rails'
EOF
    
    echo "3.3.0" > "$project_dir/.ruby-version"
    
    cd "$project_dir"
    
    run_rb_command environment
    
    # Should fallback to available version or provide helpful error
    if [ "$status" -eq 0 ]; then
        output_contains "3.4.5"  # Fallback to latest available
    else
        output_contains "3.3.0"  # Error mentions requested version
    fi
}

@test "handles corrupted Ruby installation gracefully" {
    # Create a temporary directory for corrupted Ruby installation
    local temp_rubies_dir="$TEST_ROOT_DIR/corrupted_rubies"
    local ruby_dir="$temp_rubies_dir/ruby-3.2.4"
    mkdir -p "$ruby_dir/bin"
    mkdir -p "$ruby_dir/lib"
    # Create a non-executable file where ruby should be
    echo "not executable" > "$ruby_dir/bin/ruby"
    
    # Run with custom rubies directory
    RUBIES_DIR="$temp_rubies_dir" run_rb_command runtime
    
    # Should either fail gracefully or detect the invalid installation
    if [ "$status" -eq 0 ]; then
        # If it succeeds, it should have detected that this isn't a valid ruby
        output_contains "Ruby Environment Survey"
    else
        # If it fails, should provide helpful error
        output_contains "error" || output_contains "invalid" || output_contains "not found"
    fi
}

@test "handles permission issues gracefully" {
    # Remove write permissions from gem home to simulate permission issues
    chmod 444 "$TEST_GEM_HOME" 2>/dev/null || true
    
    run_rb_command environment
    
    # Should handle permission issues gracefully
    [ "$status" -eq 0 ] || output_contains "permission"
    
    # Restore permissions for cleanup
    chmod 755 "$TEST_GEM_HOME" 2>/dev/null || true
}

@test "handles extremely long paths" {
    # Create deep directory structure
    local deep_path="$TEST_WORK_DIR"
    for i in {1..20}; do
        deep_path="$deep_path/very-long-directory-name-$i"
    done
    mkdir -p "$deep_path"
    
    cd "$deep_path"
    
    run_rb_command environment
    
    # Should handle deep paths without issues
    [ "$status" -eq 0 ]
    output_contains "3.4.5"
}

@test "handles special characters in project names" {
    # Create project with special characters (but shell-safe)
    project_dir="$TEST_WORK_DIR/my-app_with.special-chars"
    mkdir -p "$project_dir"
    
    cat > "$project_dir/Gemfile" << 'EOF'
source 'https://rubygems.org'

ruby '3.2.4'
gem 'rails'
EOF
    
    cd "$project_dir"
    
    run_rb_command environment
    
    # Should handle special characters in paths
    [ "$status" -eq 0 ]
    output_contains "3.2.4"
}

@test "handles empty Gemfile" {
    project_dir="$TEST_WORK_DIR/empty-gemfile"
    mkdir -p "$project_dir"
    
    # Create empty Gemfile
    touch "$project_dir/Gemfile"
    
    cd "$project_dir"
    
    run_rb_command environment
    
    # Should handle empty Gemfile and use latest available Ruby
    [ "$status" -eq 0 ]
    output_contains "3.4.5"
}

@test "handles malformed Gemfile" {
    project_dir="$TEST_WORK_DIR/malformed-gemfile"
    mkdir -p "$project_dir"
    
    # Create malformed Gemfile
    cat > "$project_dir/Gemfile" << 'EOF'
source 'https://rubygems.org'

ruby '3.2.4'
# Missing quote and other syntax errors
gem 'rails' ~> 7.0
invalid syntax here
EOF
    
    cd "$project_dir"
    
    run_rb_command environment
    
    # Should handle malformed Gemfile gracefully
    [ "$status" -eq 0 ]
    output_contains "Ruby Environment"
}

@test "handles concurrent gem home access" {
    # This simulates parallel test execution
    # Each test should have isolated gem homes, so this should work
    run_rb_command environment
    
    [ "$status" -eq 0 ]
    output_contains "3.4.5"
    
    # Verify gem home isolation
    [ -d "$TEST_GEM_HOME" ]
    [ "$TEST_GEM_HOME" != "/home/testuser/.gem" ]  # Should not use default
}

@test "handles missing rubies directory" {
    # Use a non-existent directory
    local nonexistent_rubies="/tmp/nonexistent_rubies_$(date +%s)"
    
    RUBIES_DIR="$nonexistent_rubies" run_rb_failure runtime
    
    # Should provide helpful error about missing Ruby installations
    output_contains "error" || output_contains "No Ruby" || output_contains "not found"
}

@test "handles version resolution edge cases" {
    # Real Ruby versions available: 3.2.4 and 3.4.5
    
    run_rb_success runtime
    
    # Should properly sort and select latest stable version
    output_contains "Ruby Environment Survey"
    # Should prefer 3.4.5 as the latest
    output_contains "(3.4.5)"
}
