#!/usr/bin/env bats

load helpers

setup() {
    setup_temp_dir
}

teardown() {
    teardown_temp_dir
}

@test "environment command shows current Ruby environment" {
    run_rb -R "$RUBIES_DIR" environment
    
    [ "$status" -eq 0 ]
    output_contains "Your Current Ruby Environment"
}

@test "environment command alias 'env' works" {
    run_rb -R "$RUBIES_DIR" env
    
    [ "$status" -eq 0 ]
    output_contains "Your Current Ruby Environment"
}

@test "environment command shows selected Ruby version" {
    run_rb -R "$RUBIES_DIR" -r "$LATEST_RUBY" environment
    
    [ "$status" -eq 0 ]
    output_contains "$LATEST_RUBY"
}

@test "environment command with custom gem home" {
    local custom_gem_home="/tmp/test-gems"
    
    run_rb -R "$RUBIES_DIR" -r "$LATEST_RUBY" -G "$custom_gem_home" environment
    
    [ "$status" -eq 0 ]
    output_contains "$custom_gem_home"
}

@test "environment command in bundler project shows bundler details" {
    local project_dir="$TEST_PROJECT_DIR/bundler_test"
    create_bundler_project "$project_dir"
    
    cd "$project_dir"
    run_rb -R "$RUBIES_DIR" environment
    
    [ "$status" -eq 0 ]
    output_contains "Bundler"
    output_contains "Gemfile"
}

@test "environment command detects ruby version from .ruby-version" {
    local project_dir="$TEST_PROJECT_DIR/ruby_version_test"
    create_bundler_project "$project_dir" "$OLDER_RUBY"
    
    cd "$project_dir"
    run_rb -R "$RUBIES_DIR" environment
    
    [ "$status" -eq 0 ]
    output_contains "$OLDER_RUBY"
}

@test "environment command shows different output outside bundler project" {
    local empty_dir="$TEST_PROJECT_DIR/empty"
    mkdir -p "$empty_dir"
    cd "$empty_dir"
    
    run_rb -R "$RUBIES_DIR" environment
    
    [ "$status" -eq 0 ]
    # Should not show bundler-specific information
    output_not_contains "Gemfile"
}

@test "environment command with bundler project shows gem home" {
    local project_dir="$TEST_PROJECT_DIR/gem_home_test"
    create_bundler_project "$project_dir"
    
    cd "$project_dir"
    run_rb -R "$RUBIES_DIR" environment
    
    [ "$status" -eq 0 ]
    output_contains "Gem home"
}

@test "environment command respects ruby version in Gemfile" {
    local project_dir="$TEST_PROJECT_DIR/gemfile_ruby_test"
    create_bundler_project "$project_dir" "" "$OLDER_RUBY"
    
    cd "$project_dir"
    run_rb -R "$RUBIES_DIR" environment
    
    [ "$status" -eq 0 ]
    output_contains "$OLDER_RUBY"
}

@test "environment command with no ruby version specifications uses latest" {
    local project_dir="$TEST_PROJECT_DIR/no_version_test"
    create_bundler_project "$project_dir"
    
    cd "$project_dir"
    run_rb -R "$RUBIES_DIR" environment
    
    [ "$status" -eq 0 ]
    output_contains "$LATEST_RUBY"
}

@test "environment command with both .ruby-version and Gemfile ruby prefers .ruby-version" {
    local project_dir="$TEST_PROJECT_DIR/both_versions_test"
    create_bundler_project "$project_dir" "$OLDER_RUBY" "$LATEST_RUBY"
    
    cd "$project_dir"
    run_rb -R "$RUBIES_DIR" environment
    
    [ "$status" -eq 0 ]
    output_contains "$OLDER_RUBY"
}

@test "environment command with only .ruby-version uses that version" {
    local project_dir="$TEST_PROJECT_DIR/only_ruby_version_test"
    create_bundler_project "$project_dir" "$OLDER_RUBY"
    
    cd "$project_dir"
    run_rb -R "$RUBIES_DIR" environment
    
    [ "$status" -eq 0 ]
    output_contains "$OLDER_RUBY"
}

@test "environment command with only Gemfile ruby uses that version" {
    local project_dir="$TEST_PROJECT_DIR/only_gemfile_ruby_test"
    create_bundler_project "$project_dir" "" "$LATEST_RUBY"
    
    cd "$project_dir"
    run_rb -R "$RUBIES_DIR" environment
    
    [ "$status" -eq 0 ]
    output_contains "$LATEST_RUBY"
}

@test "environment command override with -r flag takes precedence" {
    local project_dir="$TEST_PROJECT_DIR/override_test"
    create_bundler_project "$project_dir" "$OLDER_RUBY" "$OLDER_RUBY"
    
    cd "$project_dir"
    run_rb -R "$RUBIES_DIR" -r "$LATEST_RUBY" environment
    
    [ "$status" -eq 0 ]
    output_contains "$LATEST_RUBY"
}

@test "environment command supports Gemfile ruby file directive" {
    local project_dir="$TEST_PROJECT_DIR/ruby_file_test"
    create_bundler_project "$project_dir" "$OLDER_RUBY" ".ruby-version"
    
    run_rb -R "$RUBIES_DIR" environment
    
    [ "$status" -eq 0 ]
    output_contains "$OLDER_RUBY"
}
