#!/usr/bin/env bats

load helpers

@test "help command shows usage" {
    run_rb --help
    
    [ "$status" -eq 0 ]
    output_contains "Usage"
}

@test "help command shows available commands" {
    run_rb --help
    
    [ "$status" -eq 0 ]
    output_contains "Commands"
}

@test "help command shows available options" {
    run_rb --help
    
    [ "$status" -eq 0 ]
    output_contains "Options"
}

@test "help shows when no arguments provided" {
    run_rb
    
    [ "$status" -eq 2 ]  # rb returns exit code 2 when no args provided
    output_contains "Usage"
    output_contains "Commands"
}

@test "help command mentions runtime command" {
    run_rb --help
    
    [ "$status" -eq 0 ]
    output_contains "runtime"
}

@test "help command mentions exec command" {
    run_rb --help
    
    [ "$status" -eq 0 ]
    output_contains "exec"
}
