#!/bin/bash
# ShellSpec tests for Ruby Butler runtime command
# Distinguished validation of Ruby environment surveying capabilities

Describe "Ruby Butler Runtime System"
  Include spec/support/helpers.sh
  
  Describe "runtime command"
    Context "when surveying available Ruby installations"
      It "elegantly lists distinguished Ruby installations"
        When run rb runtime
        The status should equal 0
        The output should include "$LATEST_RUBY"
        The output should include "$OLDER_RUBY"
      End

      It "presents the distinguished survey header"
        When run rb runtime
        The status should equal 0
        The output should include "Ruby Environment Survey"
      End

      It "gracefully handles non-existing paths"
        When run rb -R "/non/existing" runtime
        The status should not equal 0
        The stderr should include "No such file or directory"
      End

      It "presents latest Ruby with appropriate precedence"
        When run rb runtime
        The status should equal 0
        # Latest version should appear before older version in output
        The output should include "$LATEST_RUBY"
        The output should include "$OLDER_RUBY"
      End
    End

    Context "with distinguished customizations"
      It "elegantly displays custom gem environment"
        When run rb -r "$LATEST_RUBY" -G "/tmp/custom-gems" runtime
        The status should equal 0
        The output should include "/tmp/custom-gems"
      End

      It "respects specific Ruby version selection"
        When run rb -r "$OLDER_RUBY" runtime
        The status should equal 0
        The output should include "$OLDER_RUBY"
      End
    End

    Context "command aliases"
      It "responds gracefully to 'rt' alias"
        When run rb rt
        The status should equal 0
        The output should include "Ruby Environment Survey"
        The output should include "$LATEST_RUBY"
      End
    End
  End
End
