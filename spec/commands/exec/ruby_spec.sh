#!/bin/bash
# ShellSpec tests for Ruby Butler exec command - Ruby environment testing
# Distinguished validation of Ruby execution capabilities

Describe "Ruby Butler Exec Command - Ruby Environment"
  Include spec/support/helpers.sh

  Describe "exec command with Ruby environment"
    Context "basic execution"
      It "gracefully presents usage guidance when no program specified"
        When run rb -R "$RUBIES_DIR" exec
        The status should not equal 0
        The stderr should include "Request Incomplete: No program specified for execution"
        The stderr should include "Proper usage: rb exec <program>"
      End

      It "responds elegantly to 'x' alias"
        When run rb -R "$RUBIES_DIR" x
        The status should not equal 0
        The stderr should include "Request Incomplete: No program specified for execution"
        The stderr should include "Proper usage: rb exec <program>"
      End

      It "preserves program exit codes with dignity"
        When run rb -R "$RUBIES_DIR" exec ruby -e "exit 42"
        The status should equal 42
      End

      It "gracefully handles non-existent programs"
        When run rb -R "$RUBIES_DIR" exec nonexistent-program
        The status should not equal 0
        The stderr should include "appears to be"
        The stderr should include "entirely absent from your distinguished Ruby environment"
      End
    End

    Context "ruby version selection (-r, --ruby)"
      It "runs ruby -v with default version"
        When run rb -R "$RUBIES_DIR" exec ruby -v
        The status should equal 0
        The output should include "ruby $LATEST_RUBY"
      End

      It "respects specific Ruby version with -r flag"
        When run rb -R "$RUBIES_DIR" -r "$OLDER_RUBY" exec ruby -v
        The status should equal 0
        The output should include "ruby $OLDER_RUBY"
      End

      It "respects specific Ruby version with --ruby flag"
        When run rb --rubies-dir "$RUBIES_DIR" --ruby "$LATEST_RUBY" exec ruby -v
        The status should equal 0
        The output should include "ruby $LATEST_RUBY"
      End

      It "works with latest Ruby version variable"
        When run rb -R "$RUBIES_DIR" -r "$LATEST_RUBY" exec ruby -v
        The status should equal 0
        The output should include "$LATEST_RUBY"
      End

      It "works with older Ruby version variable"
        When run rb -R "$RUBIES_DIR" -r "$OLDER_RUBY" exec ruby -v
        The status should equal 0
        The output should include "$OLDER_RUBY"
      End

      It "handles non-existent Ruby version gracefully"
        When run rb -R "$RUBIES_DIR" -r "9.9.9" exec ruby -v
        The status should not equal 0
        The stderr should include "No suitable Ruby installation found"
        The stdout should include "Requested Ruby version 9.9.9 not found"
      End
    End

    Context "rubies directory specification (-R, --rubies-dir)"
      It "respects custom rubies directory with -R flag"
        When run rb -R "$RUBIES_DIR" exec ruby -v
        The status should equal 0
        The output should include "ruby"
      End

      It "respects custom rubies directory with --rubies-dir flag"
        When run rb --rubies-dir "$RUBIES_DIR" exec ruby -v
        The status should equal 0
        The output should include "ruby"
      End

      It "handles non-existent rubies directory gracefully"
        When run rb -R "/non/existent/path" exec ruby -v
        The status should not equal 0
        The stderr should include "designated Ruby estate directory"
        The stderr should include "appears to be absent from your system"
      End

      It "combines rubies directory with specific Ruby version"
        When run rb -R "$RUBIES_DIR" -r "$LATEST_RUBY" exec ruby -v
        The status should equal 0
        The output should include "ruby $LATEST_RUBY"
      End
    End

    Context "gem environment testing with gem env"
      It "executes gem env with appropriate ceremony"
        When run rb -R "$RUBIES_DIR" exec gem env
        The status should equal 0
        The output should include "RUBYGEMS VERSION"
        The output should include "RUBY VERSION"
        The output should include "INSTALLATION DIRECTORY"
        The output should include "RUBY EXECUTABLE"
        The output should include "EXECUTABLE DIRECTORY"
      End

      It "shows correct Ruby version in gem env"
        When run rb -R "$RUBIES_DIR" -r "$OLDER_RUBY" exec gem env
        The status should equal 0
        The output should include "RUBY VERSION: $OLDER_RUBY"
        The output should include "ruby-$OLDER_RUBY/bin/ruby"
      End

      It "shows correct Ruby executable path"
        When run rb -R "$RUBIES_DIR" -r "$LATEST_RUBY" exec gem env
        The status should equal 0
        The output should include "RUBY EXECUTABLE"
        The output should include "/opt/rubies/ruby-$LATEST_RUBY/bin/ruby"
      End
    End

    Context "gem home specification (-G, --gem-home)"
      It "respects custom gem home with -G flag"
        When run rb -R "$RUBIES_DIR" -G "/tmp/test-gems" exec gem env
        The status should equal 0
        The output should include "INSTALLATION DIRECTORY"
        The output should include "/tmp/test-gems"
      End

      It "respects custom gem home with --gem-home flag"
        # Use /app/rb directly to avoid conflicts
        When run rb -R "$RUBIES_DIR" --gem-home "/tmp/custom-gems" exec gem env
        The status should equal 0
        The output should include "INSTALLATION DIRECTORY"
        The output should include "/tmp/custom-gems"
      End

      It "combines gem home with specific Ruby version"
        When run rb -R "$RUBIES_DIR" -r "$LATEST_RUBY" -G "/tmp/version-gems" exec gem env
        The status should equal 0
        The output should include "RUBY VERSION: $LATEST_RUBY"
        The output should include "/tmp/version-gems"
      End

      It "shows correct executable directory with custom gem home"
        When run rb -R "$RUBIES_DIR" -G "/tmp/exec-gems" exec gem env
        The status should equal 0
        The output should include "EXECUTABLE DIRECTORY"
        The output should include "/tmp/exec-gems"
      End
    End

    Context "parameter combinations"
      It "handles all parameters together"
        When run rb -R "$RUBIES_DIR" -r "$OLDER_RUBY" -G "/tmp/combined-gems" exec gem env
        The status should equal 0
        The output should include "RUBY VERSION: $OLDER_RUBY"
        The output should include "/tmp/combined-gems"
      End

      It "handles long-form parameters together"
        When run rb --rubies-dir "$RUBIES_DIR" --ruby "$LATEST_RUBY" --gem-home "/tmp/long-gems" exec gem env
        The status should equal 0
        The output should include "RUBY VERSION: $LATEST_RUBY"
        The output should include "/tmp/long-gems"
      End

      It "handles mixed short and long parameters"
        When run rb --rubies-dir "$RUBIES_DIR" --ruby "$LATEST_RUBY" -G "/tmp/mixed-gems" exec gem env
        The status should equal 0
        The output should include "RUBY VERSION: $LATEST_RUBY"
        The output should include "/tmp/mixed-gems"
      End
    End

    Context "gem management commands"
      It "displays installed gems elegantly"
        When run rb -R "$RUBIES_DIR" exec gem list
        The status should equal 0
        The output should include "bundler"
      End

      It "shows gem list with specific Ruby version"
        When run rb -R "$RUBIES_DIR" -r "$OLDER_RUBY" exec gem list
        The status should equal 0
        The output should include "bundler"
      End

      It "executes gem commands in custom gem home"
        When run rb -R "$RUBIES_DIR" -G "/tmp/gem-list" exec gem list
        The status should equal 0
        The output should include "default:"
      End
    End
  End
End
