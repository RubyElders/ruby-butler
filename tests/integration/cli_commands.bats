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
    # No Ruby installations created - should fail gracefully
    run_rb_failure runtime
    
    # Should contain error message
    output_contains "error"
}

@test "runtime command with multiple Ruby installations" {
    # Create multiple Ruby installations
    create_ruby_installation "3.1.0"
    create_ruby_installation "3.2.5" 
    create_ruby_installation "3.3.1"
    
    run_rb_success runtime
    
    # Should contain survey information
    output_contains_all \
        "Ruby Environment Survey" \
        "CRuby" \
        "(3.1.0)" \
        "(3.2.5)" \
        "(3.3.1)" \
        "Environment Ready" \
        "(3.3.1)"  # Should select latest
}

@test "runtime command alias 'rt' works" {
    create_ruby_installation "3.2.5"
    
    run_rb_success rt
    
    output_contains_all \
        "Ruby Environment Survey" \
        "CRuby" \
        "(3.2.5)"
}

@test "environment command with no bundler project" {
    create_ruby_installation "3.2.5"
    
    run_rb_success environment
    
    output_contains_all \
        "Your Current Ruby Environment" \
        "CRuby" \
        "(3.2.5)" \
        "Bundler environment not detected" \
        "Environment ready for distinguished Ruby development"
}

@test "environment command alias 'env' works" {
    create_ruby_installation "3.2.5"
    
    run_rb_success env
    
    output_contains_all \
        "Your Current Ruby Environment" \
        "CRuby" \
        "(3.2.5)"
}

@test "environment command from bundler project directory" {
    create_ruby_installation "3.2.5"
    
    # Create bundler project and cd into it
    project_dir=$(create_bundler_project "test-app" "3.2.5")
    cd "$project_dir"
    
    run_rb_success environment
    
    output_contains_all \
        "Your Current Ruby Environment" \
        "CRuby" \
        "(3.2.5)"
        
    # Note: Bundler detection depends on having bundler installed,
    # which may not be available in test environment
}

@test "runtime command sorts versions correctly" {
    # Create Ruby versions in non-sorted order
    create_ruby_installation "3.0.1"
    create_ruby_installation "3.2.0"
    create_ruby_installation "3.1.5"
    create_ruby_installation "3.3.0"
    
    run_rb_success runtime
    
    # Should select 3.3.0 as latest and show it in environment ready
    output_contains "Environment Ready"
    output_contains "(3.3.0)"
}

@test "runtime command with custom rubies directory" {
    # Create Ruby in custom location (already using TEST_ROOT_DIR/rubies)
    create_ruby_installation "3.2.4"
    
    run_rb_success runtime
    
    output_contains_all \
        "Ruby Environment Survey" \
        "(3.2.4)"
}
