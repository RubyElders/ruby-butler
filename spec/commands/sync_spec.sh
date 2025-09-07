#!/bin/bash

Describe 'rb sync command'
  Include spec/support/helpers.sh
  
  # Setup temporary directory before each test
  BeforeEach 'setup_test_project'
  AfterEach 'cleanup_test_project'

  Context 'when running sync in bundler project'
    BeforeEach 'create_bundler_project .'
    
    It 'successfully synchronizes bundler environment'
      When run rb sync
      The status should be success
      The output should include "Environment Successfully Synchronized"
      The output should include "Bundle complete!"
    End
  End

  Context 'when running sync in non-bundler project'
    # Already in empty test directory, no bundler project
    
    It 'fails gracefully with appropriate message'
      When run rb sync
      The status should be failure
      The output should include "Bundler Environment Not Detected"
      The stderr should include "Sync failed"
    End
  End

  Context 'when using sync command alias'
    Context 'with non-bundler project'
      # Already in empty test directory, no bundler project
      
      It 'fails gracefully with "s" alias when no proper bundler project'
        When run rb s
        The status should be failure
        The output should include "Bundler Environment Not Detected"
        The stderr should include "Sync failed"
      End
    End
    
    Context 'with bundler project'
      BeforeEach 'create_bundler_project .'
      
      It 'works with "s" alias in bundler project'
        When run rb s
        The status should be success
        The output should include "Environment Successfully Synchronized"
      End
    End
  End

  # Data-driven testing with different Ruby versions
  Context 'sync behavior with different Ruby versions'
    Parameters
      "$OLDER_RUBY" "older"
      "$LATEST_RUBY" "latest"
    End

    Example "works with Ruby $1 ($2 version)"
      create_bundler_project "." "$1"
      
      When run rb sync
      The status should be success
      The output should include "Synchronizing"
    End
  End

  # Complex project testing
  Context 'sync with complex projects'
    It 'handles complex Gemfile configurations'
      create_complex_gemfile "." "$LATEST_RUBY"
      
      When run rb sync
      The status should be success
      The output should include "Environment Successfully Synchronized"
    End
  End

  # Path management testing  
  Context 'sync with path management'
    BeforeEach 'create_bundler_project .'
    
    It 'works with path references'
      When run command test -d "/opt/rubies"
      The status should be success
    End
  End
End
