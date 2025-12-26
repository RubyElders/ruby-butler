#!/bin/bash
# ShellSpec tests for Ruby Butler version command
# Distinguished validation of version information display

Describe "Ruby Butler Version Command"
  Include spec/support/helpers.sh

  Describe "version command (command-based interface)"
    Context "when invoked with 'version' command"
      It "displays Ruby Butler version successfully"
        When run rb version
        The status should equal 0
        The output should include "Ruby Butler"
      End

      It "shows version number"
        When run rb version
        The status should equal 0
        The output should include "v0."
      End

      It "displays distinguished butler identity"
        When run rb version
        The status should equal 0
        The output should include "gentleman"
      End

      It "includes attribution to RubyElders"
        When run rb version
        The status should equal 0
        The output should include "RubyElders"
      End
    End

    Context "when --version flag is used (deprecated)"
      It "rejects --version flag with error"
        When run rb --version
        The status should not equal 0
        The error should include "unexpected argument"
      End
    End

    Context "environment variable support"
      It "version command works with RB_VERBOSE environment variable"
        export RB_VERBOSE=true
        When run rb version
        The status should equal 0
        The output should include "Ruby Butler"
      End

      It "version command works with RB_LOG_LEVEL environment variable"
        export RB_LOG_LEVEL=info
        When run rb version
        The status should equal 0
        The output should include "Ruby Butler"
      End
    End
  End
End
