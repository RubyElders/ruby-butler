#!/usr/bin/env bats

load helpers

setup() {
    setup_temp_dir
}

teardown() {
    teardown_temp_dir
}

@test "sync command installs gems on first run in new project" {
    local project_dir="$TEST_PROJECT_DIR/new_project"
    create_bundler_project "$project_dir"
    
    cd "$project_dir"
    run_rb -R "$RUBIES_DIR" sync
    
    [ "$status" -eq 0 ]
    # Should show bundler installation activity and rb confirmation
    output_contains "Installing"
    output_contains "Bundle complete!"
    output_contains "Environment Successfully Synchronized"
}

@test "sync command reports installed on second run" {
    local project_dir="$TEST_PROJECT_DIR/synced_project"
    create_bundler_project "$project_dir"
    
    cd "$project_dir"
    # First run to install
    run_rb -R "$RUBIES_DIR" sync
    [ "$status" -eq 0 ]
    
    # Second run should report already installed
    run_rb -R "$RUBIES_DIR" sync
    [ "$status" -eq 0 ]
    output_contains "Environment Already Synchronized"
}

@test "sync command fails gracefully in non-bundler project" {
    local empty_dir="$TEST_PROJECT_DIR/empty"
    mkdir -p "$empty_dir"
    cd "$empty_dir"
    
    run_rb -R "$RUBIES_DIR" sync
    
    [ "$status" -ne 0 ]  # sync should return non-zero when no Gemfile found
    output_contains "Bundler Environment Not Detected" || output_contains "No Gemfile found"
}

@test "sync command alias 's' works" {
    local project_dir="$TEST_PROJECT_DIR/alias_test"
    create_bundler_project "$project_dir"
    
    cd "$project_dir"
    run_rb -R "$RUBIES_DIR" s
    
    [ "$status" -eq 0 ]
    output_contains "Installing"
    output_contains "Bundle complete!"
    output_contains "Environment Successfully Synchronized"
}

@test "sync command installs new gems when Gemfile updated" {
    local project_dir="$TEST_PROJECT_DIR/update_test"
    create_bundler_project "$project_dir"
    
    cd "$project_dir"
    # First sync
    run_rb -R "$RUBIES_DIR" sync
    [ "$status" -eq 0 ]
    
    # Add a new gem to Gemfile
    echo "gem 'minitest'" >> Gemfile
    
    # Sync again - should install new gem
    run_rb -R "$RUBIES_DIR" sync
    [ "$status" -eq 0 ]
    output_contains "Installing"
    output_contains "Bundle complete!"
    output_contains "Environment Successfully Synchronized"
}

@test "sync command after adding gem then reports complete on next run" {
    local project_dir="$TEST_PROJECT_DIR/complete_test"
    create_bundler_project "$project_dir"
    
    cd "$project_dir"
    # First sync
    run_rb -R "$RUBIES_DIR" sync
    [ "$status" -eq 0 ]
    
    # Add gem and sync
    echo "gem 'minitest'" >> Gemfile
    run_rb -R "$RUBIES_DIR" sync
    [ "$status" -eq 0 ]
    
    # Third run should report complete
    run_rb -R "$RUBIES_DIR" sync
    [ "$status" -eq 0 ]
    output_contains "Environment Already Synchronized"
}

@test "sync command respects ruby version from project" {
    local project_dir="$TEST_PROJECT_DIR/ruby_version_test"
    create_bundler_project "$project_dir" "$OLDER_RUBY"
    
    cd "$project_dir"
    run_rb -R "$RUBIES_DIR" sync
    
    [ "$status" -eq 0 ]
    # Should work with the specified Ruby version
    output_contains "Installing"
    output_contains "Bundle complete!"
    output_contains "Environment Successfully Synchronized"
}

@test "sync command with custom gem home" {
    local project_dir="$TEST_PROJECT_DIR/gem_home_test"
    local custom_gem_home="/tmp/test-sync-gems"
    create_bundler_project "$project_dir"
    
    cd "$project_dir"
    run_rb -R "$RUBIES_DIR" -G "$custom_gem_home" sync
    
    [ "$status" -eq 0 ]
    output_contains "Installing"
    output_contains "Bundle complete!"
    output_contains "Environment Successfully Synchronized"
}
