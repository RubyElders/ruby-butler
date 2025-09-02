#!/usr/bin/env bats

load helpers

@test "runtime command lists available Ruby installations" {
    run_rb -R "$RUBIES_DIR" runtime
    
    [ "$status" -eq 0 ]
    output_contains "$LATEST_RUBY"
    output_contains "$OLDER_RUBY"
}

@test "runtime command shows survey header" {
    run_rb -R "$RUBIES_DIR" runtime
    
    [ "$status" -eq 0 ]
    output_contains "Ruby Environment Survey"
}

@test "runtime command with non-existing path shows no rubies" {
    run_rb -R "/non/existing" runtime
    
    [ "$status" -ne 0 ]
    output_not_contains "$LATEST_RUBY"
    output_not_contains "$OLDER_RUBY"
}

@test "runtime command shows latest ruby first" {
    run_rb -R "$RUBIES_DIR" runtime
    
    [ "$status" -eq 0 ]
    # Latest version should appear before older version in output
    local latest_pos=$(echo "$output" | grep -n "$LATEST_RUBY" | head -1 | cut -d: -f1)
    local older_pos=$(echo "$output" | grep -n "$OLDER_RUBY" | head -1 | cut -d: -f1)
    
    [ "$latest_pos" -lt "$older_pos" ]
}

@test "runtime command with custom gem home shows gem environment" {
    local custom_gem_home="/tmp/custom-gems"
    
    run_rb -R "$RUBIES_DIR" -r "$LATEST_RUBY" -G "$custom_gem_home" runtime
    
    [ "$status" -eq 0 ]
    output_contains "$custom_gem_home"
}

@test "runtime command alias 'rt' works" {
    run_rb -R "$RUBIES_DIR" rt
    
    [ "$status" -eq 0 ]
    output_contains "Ruby Environment Survey"
    output_contains "$LATEST_RUBY"
}

@test "runtime command with specific ruby version" {
    run_rb -R "$RUBIES_DIR" -r "$OLDER_RUBY" runtime
    
    [ "$status" -eq 0 ]
    output_contains "$OLDER_RUBY"
}
