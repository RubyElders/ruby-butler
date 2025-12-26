#!/bin/bash

Describe 'rb sync command'
  Include spec/support/helpers.sh

  # Setup temporary directory before each test
  BeforeEach 'setup_test_project'
  AfterEach 'cleanup_test_project'

  Context 'when running sync in bundler project'
    BeforeEach 'create_bundler_project .'

    It 'successfully synchronizes bundler environment'
      When run rb -R "$RUBIES_DIR" sync
      The status should be success
      The output should include "Environment Successfully Synchronized"
      The output should include "Bundle complete!"
    End
  End

  Context 'when running sync in non-bundler project'
    # Already in empty test directory, no bundler project

    It 'fails gracefully with appropriate message'
      When run rb -R "$RUBIES_DIR" sync
      The status should be failure
      The stderr should include "Bundler environment not detected"
    End
  End

  Context 'when using sync command alias'
    Context 'with non-bundler project'
      # Already in empty test directory, no bundler project

      It 'fails gracefully with "s" alias when no proper bundler project'
        When run rb -R "$RUBIES_DIR" s
        The status should be failure
        The stderr should include "Bundler environment not detected"
      End
    End

    Context 'with bundler project'
      BeforeEach 'create_bundler_project .'

      It 'works with "s" alias in bundler project'
        When run rb -R "$RUBIES_DIR" s
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

      When run rb -R "$RUBIES_DIR" sync
      The status should be success
      The output should include "Synchronizing"
    End
  End

  Context 'when gem is removed from Gemfile'
    It 'updates Gemfile.lock to reflect removed gem'
      # Create Gemfile with TWO gems
      cat > Gemfile << 'EOF'
source 'https://rubygems.org'
gem 'rake'
gem 'minitest'
EOF

      # Initial sync - install both gems
      rb -R "$RUBIES_DIR" sync >/dev/null 2>&1

      # Verify both gems are in Gemfile.lock
      grep -q "rake" Gemfile.lock || fail "rake should be in initial Gemfile.lock"
      grep -q "minitest" Gemfile.lock || fail "minitest should be in initial Gemfile.lock"

      # Remove minitest from Gemfile
      cat > Gemfile << 'EOF'
source 'https://rubygems.org'
gem 'rake'
EOF

      # Run sync again
      When run rb -R "$RUBIES_DIR" sync
      The status should be success
      The output should include "Synchronizing"

      # Verify rake is still in lockfile but minitest is removed
      The path Gemfile.lock should be exist
      The contents of file Gemfile.lock should include "rake"
      The contents of file Gemfile.lock should not include "minitest"
    End
  End

  Context 'environment variable support'
    It 'respects RB_RUBIES_DIR environment variable'
      create_bundler_project "."
      export RB_RUBIES_DIR="$RUBIES_DIR"
      When run rb sync
      The status should be success
      The output should include "Environment Successfully Synchronized"
    End

    It 'respects RB_RUBY_VERSION environment variable'
      create_bundler_project "." "$OLDER_RUBY"
      export RB_RUBY_VERSION="$OLDER_RUBY"
      When run rb -R "$RUBIES_DIR" sync
      The status should be success
      The output should include "Synchronizing"
    End

    It 'respects RB_NO_BUNDLER environment variable (disables sync)'
      create_bundler_project "."
      export RB_NO_BUNDLER=true
      When run rb -R "$RUBIES_DIR" sync
      The status should be failure
      The stderr should include "Bundler environment not detected"
    End

    It 'allows CLI flags to override environment variables'
      create_bundler_project "." "$OLDER_RUBY"
      export RB_RUBY_VERSION="$LATEST_RUBY"
      When run rb -R "$RUBIES_DIR" -r "$OLDER_RUBY" sync
      The status should be success
      The output should include "Synchronizing"
    End
  End
End
