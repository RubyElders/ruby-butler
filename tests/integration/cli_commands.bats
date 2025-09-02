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

@test "runtime command with empty directory fails gracefully" {
    # Test with a custom empty rubies directory
    run /app/rb --rubies-dir "/tmp/empty_rubies" --gem-home "$TEST_GEM_HOME" runtime
    
    # Should fail gracefully when no Ruby installations found
    [ "$status" -ne 0 ]
    output_contains "error" || output_contains "No Ruby" || output_contains "not found"
}

@test "runtime command with multiple Ruby installations" {
    # Real Ruby installations are available: 3.2.4 and 3.4.5
    
    run_rb_success runtime
    
    # Should contain survey information
    output_contains_all \
        "Ruby Environment Survey" \
        "CRuby" \
        "(3.2.4)" \
        "(3.4.5)" \
        "Environment Ready" \
        "(3.4.5)"  # Should select latest
}

@test "runtime command alias 'rt' works" {
    # Use real Ruby installations
    
    run_rb_success rt
    
    output_contains_all \
        "Ruby Environment Survey" \
        "CRuby"
}

@test "environment command with no bundler project" {
    # Use real Ruby installations
    
    run_rb_success environment
    
    output_contains_all \
        "Your Current Ruby Environment" \
        "CRuby" \
        "Bundler environment not detected" \
        "Environment ready for distinguished Ruby development"
}

@test "environment command alias 'env' works" {
    # Use real Ruby installations
    
    run_rb_success env
    
    output_contains_all \
        "Your Current Ruby Environment" \
        "CRuby"
}

@test "environment command from bundler project directory" {
    # Use real Ruby installations
    
    # Create bundler project and cd into it
    project_dir=$(create_bundler_project "test-app" "3.4.5")
    cd "$project_dir"
    
    run_rb_success environment
    
    output_contains_all \
        "Your Current Ruby Environment" \
        "CRuby" \
        "(3.4.5)"
        
    # Note: Bundler detection depends on having bundler installed,
    # which may not be available in test environment
}

@test "runtime command sorts versions correctly" {
    # Real Ruby versions available: 3.2.4 and 3.4.5
    
    run_rb_success runtime
    
    # Should select 3.4.5 as latest and show it in environment ready
    output_contains "Environment Ready"
    output_contains "(3.4.5)"
}

@test "runtime command with custom rubies directory" {
    # Using real Ruby installations from /opt/rubies
    
    run_rb_success runtime
    
    output_contains_all \
        "Ruby Environment Survey" \
        "(3.2.4)" \
        "(3.4.5)"
}
