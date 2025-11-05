#!/bin/bash
# ShellSpec tests for Ruby Butler environment command
# Distinguished validation of environment inspection capabilities

Describe "Ruby Butler Environment System"
  Include spec/support/helpers.sh

  Describe "environment command"
    Context "basic environment inspection"
      It "presents distinguished current Ruby environment"
        When run rb -R "$RUBIES_DIR" environment
        The status should equal 0
        The output should include "Your Current Ruby Environment"
      End

      It "responds gracefully to 'env' alias"
        When run rb -R "$RUBIES_DIR" env
        The status should equal 0
        The output should include "Your Current Ruby Environment"
      End
    End

    Context "ruby version selection (-r, --ruby)"
      It "displays selected Ruby version with -r flag"
        When run rb -R "$RUBIES_DIR" -r "$LATEST_RUBY" environment
        The status should equal 0
        The output should include "$LATEST_RUBY"
      End

      It "displays selected Ruby version with --ruby flag"
        When run rb -R "$RUBIES_DIR" --ruby "$OLDER_RUBY" environment
        The status should equal 0
        The output should include "$OLDER_RUBY"
      End

      It "works with latest Ruby version"
        When run rb -R "$RUBIES_DIR" -r "$LATEST_RUBY" environment
        The status should equal 0
        The output should include "$LATEST_RUBY"
      End

      It "works with older Ruby version"
        When run rb -R "$RUBIES_DIR" -r "$OLDER_RUBY" environment
        The status should equal 0
        The output should include "$OLDER_RUBY"
      End

      It "handles non-existent Ruby version gracefully"
        When run rb -R "$RUBIES_DIR" -r "9.9.9" environment
        The status should not equal 0
        The stderr should include "No suitable Ruby installation found"
        The stdout should include "Requested Ruby version 9.9.9 not found"
      End
    End

    Context "rubies directory specification (-R, --rubies-dir)"
      It "respects custom rubies directory with -R flag"
        When run rb -R "$RUBIES_DIR" environment
        The status should equal 0
        The output should include "Your Current Ruby Environment"
      End

      It "respects custom rubies directory with --rubies-dir flag"
        When run rb --rubies-dir "$RUBIES_DIR" environment
        The status should equal 0
        The output should include "Your Current Ruby Environment"
      End

      It "handles non-existent rubies directory gracefully"
        When run rb -R "/non/existent/path" environment
        The status should not equal 0
        The stderr should include "appears to be absent from your system"
      End

      It "combines rubies directory with specific Ruby version"
        When run rb -R "$RUBIES_DIR" -r "$LATEST_RUBY" environment
        The status should equal 0
        The output should include "$LATEST_RUBY"
      End
    End

    Context "gem home specification (-G, --gem-home)"
      It "respects custom gem home with -G flag"
        When run rb -R "$RUBIES_DIR" -G "/tmp/test-gems" environment
        The status should equal 0
        The output should include "/tmp/test-gems"
      End

      It "respects custom gem home with --gem-home flag"
        When run rb -R "$RUBIES_DIR" --gem-home "/tmp/custom-gems" environment
        The status should equal 0
        The output should include "/tmp/custom-gems"
      End

      It "combines gem home with specific Ruby version"
        When run rb -R "$RUBIES_DIR" -r "$LATEST_RUBY" -G "/tmp/version-gems" environment
        The status should equal 0
        The output should include "$LATEST_RUBY"
        The output should include "/tmp/version-gems"
      End

      It "shows gem home directory structure"
        When run rb -R "$RUBIES_DIR" -G "/tmp/structured-gems" environment
        The status should equal 0
        The output should include "Gem home"
        The output should include "/tmp/structured-gems"
      End
    End

    Context "parameter combinations"
      It "handles all parameters together"
        When run rb -R "$RUBIES_DIR" -r "$OLDER_RUBY" -G "/tmp/combined-gems" environment
        The status should equal 0
        The output should include "$OLDER_RUBY"
        The output should include "/tmp/combined-gems"
      End

      It "handles long-form parameters together"
        When run rb --rubies-dir "$RUBIES_DIR" --ruby "$LATEST_RUBY" --gem-home "/tmp/long-gems" environment
        The status should equal 0
        The output should include "$LATEST_RUBY"
        The output should include "/tmp/long-gems"
      End

      It "handles mixed short and long parameters"
        When run rb --rubies-dir "$RUBIES_DIR" --ruby "$LATEST_RUBY" -G "/tmp/mixed-gems" environment
        The status should equal 0
        The output should include "$LATEST_RUBY"
        The output should include "/tmp/mixed-gems"
      End
    End

    Context "bundler project integration"
      BeforeEach 'setup_test_project'
      BeforeEach 'create_bundler_project .'
      AfterEach 'cleanup_test_project'

      It "detects bundler environment in project"
        When run rb -R "$RUBIES_DIR" environment
        The status should equal 0
        The output should include "Bundler Environment"
      End

      It "shows bundler details with specific Ruby version"
        When run rb -R "$RUBIES_DIR" -r "$LATEST_RUBY" environment
        The status should equal 0
        The output should include "$LATEST_RUBY"
        The output should include "Bundler Environment"
      End

      It "respects custom gem home in bundler project"
        When run rb -R "$RUBIES_DIR" -G "/tmp/bundler-gems" environment
        The status should equal 0
        The output should include "/tmp/bundler-gems"
        The output should include "Bundler Environment"
      End
    End

    Context "ruby version detection from project files"
      BeforeEach 'setup_test_project'
      AfterEach 'cleanup_test_project'

      It "detects Ruby version from .ruby-version file"
        create_bundler_project "." "$OLDER_RUBY"

        When run rb -R "$RUBIES_DIR" environment
        The status should equal 0
        The output should include "$OLDER_RUBY"
      End

      It "detects Ruby version from Gemfile ruby directive"
        create_bundler_project "." "" "$LATEST_RUBY"

        When run rb -R "$RUBIES_DIR" environment
        The status should equal 0
        The output should include "$LATEST_RUBY"
      End

      It "prefers .ruby-version over Gemfile ruby directive"
        create_bundler_project "." "$OLDER_RUBY" "$LATEST_RUBY"

        When run rb -R "$RUBIES_DIR" environment
        The status should equal 0
        The output should include "$OLDER_RUBY"
      End

      It "overrides project version with -r flag"
        create_bundler_project "." "$OLDER_RUBY"

        When run rb -R "$RUBIES_DIR" -r "$LATEST_RUBY" environment
        The status should equal 0
        The output should include "$LATEST_RUBY"
      End
    End

    Context "environment variable display"
      It "shows gem home configuration"
        When run rb -R "$RUBIES_DIR" -G "/tmp/gem-display" environment
        The status should equal 0
        The output should include "Gem home"
        The output should include "/tmp/gem-display"
      End

      It "shows gem libraries configuration"
        When run rb -R "$RUBIES_DIR" -G "/tmp/gem-path" environment
        The status should equal 0
        The output should include "Gem libraries"
        The output should include "/tmp/gem-path"
      End

      It "displays executable paths"
        When run rb -R "$RUBIES_DIR" -r "$LATEST_RUBY" environment
        The status should equal 0
        The output should include "Executable paths"
        The output should include "ruby-$LATEST_RUBY/bin"
      End
    End
  End
End
