#!/usr/bin/env bats

load helpers

setup() {
    setup_temp_dir
}

teardown() {
    teardown_temp_dir
}

@test "exec command shows usage when no program specified" {
    run_rb -R "$RUBIES_DIR" exec
    [ "$status" -ne 0 ]
    [[ "$output" == *"required arguments were not provided"* ]]
    [[ "$output" == *"<ARGS>..."* ]]
    [[ "$output" == *"rb exec <ARGS>"* ]]
}

@test "exec command alias 'x' works" {
    run_rb -R "$RUBIES_DIR" x
    [ "$status" -ne 0 ]
    [[ "$output" == *"required arguments were not provided"* ]]
    [[ "$output" == *"<ARGS>..."* ]]
}

@test "exec command runs ruby -v successfully" {
    run_rb -R "$RUBIES_DIR" exec ruby -v
    [ "$status" -eq 0 ]
    [[ "$output" == *"ruby $LATEST_RUBY"* ]]
}

@test "exec command runs gem env successfully" {
    run_rb -R "$RUBIES_DIR" exec gem env
    [ "$status" -eq 0 ]
    [[ "$output" == *"RUBYGEMS VERSION"* ]]
    [[ "$output" == *"RUBY VERSION"* ]]
    [[ "$output" == *"INSTALLATION DIRECTORY"* ]]
    [[ "$output" == *"GEM PATHS"* ]]
}

@test "exec command with specific ruby version" {
    run_rb -R "$RUBIES_DIR" -r "$OLDER_RUBY" exec ruby -v
    [ "$status" -eq 0 ]
    [[ "$output" == *"ruby $OLDER_RUBY"* ]]
}

@test "exec command shows error for non-existent program" {
    run_rb -R "$RUBIES_DIR" exec nonexistent-program
    [ "$status" -ne 0 ]
    [[ "$output" == *"Execution Failed"* ]]
    [[ "$output" == *"No such file or directory"* ]]
}

@test "exec command preserves program exit codes" {
    # Ruby with exit code 42
    run_rb -R "$RUBIES_DIR" exec ruby -e "exit 42"
    [ "$status" -eq 42 ]
}

@test "exec command with gem list shows installed gems" {
    run_rb -R "$RUBIES_DIR" exec gem list
    [ "$status" -eq 0 ]
    [[ "$output" == *"bundler"* ]]
}

@test "exec command with custom gem home shows correct environment" {
    run_rb -R "$RUBIES_DIR" -G "/tmp/custom-gems" exec gem env
    [ "$status" -eq 0 ]
    [[ "$output" == *"INSTALLATION DIRECTORY"* ]]
    [[ "$output" == *"/tmp/custom-gems"* ]]
}

@test "exec command with bundle add and rake execution" {
    local project_dir="$TEST_PROJECT_DIR/test-project"
    create_bundler_project "$project_dir" "$LATEST_RUBY"
    cd "$project_dir"
    
    # First sync the project
    run_rb -R "$RUBIES_DIR" sync
    [ "$status" -eq 0 ]
    
    # Add rake with specific version
    run_rb -R "$RUBIES_DIR" exec bundle add rake -v '~> 12.3.3'
    [ "$status" -eq 0 ]
    [[ "$output" == *"rake"* ]]
    
    # Sync again to update gem environment
    run_rb -R "$RUBIES_DIR" sync
    [ "$status" -eq 0 ]
    
    # Test that rake is available with correct version
    run_rb -R "$RUBIES_DIR" exec rake -V
    [ "$status" -eq 0 ]
    [[ "$output" == *"12.3"* ]]
}

@test "exec command in bundler project shows bundler gem environment" {
    local project_dir="$TEST_PROJECT_DIR/test-project" 
    create_bundler_project "$project_dir" "$LATEST_RUBY"
    cd "$project_dir"
    
    run_rb -R "$RUBIES_DIR" exec gem env
    [ "$status" -eq 0 ]
    [[ "$output" == *"INSTALLATION DIRECTORY"* ]]
    # Should show project-specific path with .rb directory
    [[ "$output" == *".rb"* ]]
}

@test "exec command with bundler project respects ruby version from Gemfile" {
    local project_dir="$TEST_PROJECT_DIR/test-project"
    create_bundler_project "$project_dir" "$OLDER_RUBY"
    cd "$project_dir"
    
    run_rb -R "$RUBIES_DIR" exec ruby -v
    [ "$status" -eq 0 ]
    [[ "$output" == *"ruby $OLDER_RUBY"* ]]
}

@test "exec command runs bundle commands in bundler project" {
    local project_dir="$TEST_PROJECT_DIR/test-project"
    create_bundler_project "$project_dir" "$LATEST_RUBY"
    cd "$project_dir"
    
    # First sync the project
    run_rb -R "$RUBIES_DIR" sync
    [ "$status" -eq 0 ]
    
    # Then run bundle list (empty Gemfile shows "No gems in the Gemfile")
    run_rb -R "$RUBIES_DIR" exec bundle list
    [ "$status" -eq 0 ]
    [[ "$output" == *"Gems included by the bundle"* ]]
}
