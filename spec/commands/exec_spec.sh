#!/bin/bash
# ShellSpec tests for Ruby Butler exec command
# Distinguished validation of command execution capabilities

Describe "Ruby Butler Execution System"
  Include spec/support/helpers.sh
  
  Describe "exec command"
    Context "when no program is specified"
      It "gracefully presents usage guidance"
        When run rb exec
        The status should not equal 0
        The stderr should include "required arguments were not provided"
        The stderr should include "<ARGS>..."
      End
    End

    Context "command aliases"
      It "responds elegantly to 'x' alias"
        When run rb x
        The status should not equal 0
        The stderr should include "required arguments were not provided"
        The stderr should include "<ARGS>..."
      End
    End

    Context "executing Ruby commands"
      It "runs ruby -v with distinguished precision"
        When run rb exec ruby -v
        The status should equal 0
        The output should include "ruby 3.4.5"
      End

      It "executes gem env with appropriate ceremony"
        When run rb exec gem env
        The status should equal 0
        The output should include "RUBYGEMS VERSION"
        The output should include "RUBY VERSION"
        The output should include "INSTALLATION DIRECTORY"
        The output should include "GEM PATHS"
      End

      It "respects specific Ruby version selection"
        When run rb -r "3.2.4" exec ruby -v
        The status should equal 0
        The output should include "ruby 3.2.4"
      End

      It "gracefully handles non-existent programs"
        When run rb exec nonexistent-program
        The status should not equal 0
        The stderr should include "Execution Failed"
        The stderr should include "No such file or directory"
      End

      It "preserves program exit codes with dignity"
        When run rb exec ruby -e "exit 42"
        The status should equal 42
      End
    End

    Context "gem management"
      It "displays installed gems elegantly"
        When run rb exec gem list
        The status should equal 0
        The output should include "bundler"
      End

      It "respects custom gem home environments"
        When run rb -G "/tmp/custom-gems" exec gem env
        The status should equal 0
        The output should include "INSTALLATION DIRECTORY"
        The output should include "/tmp/custom-gems"
      End
    End
  End
End
