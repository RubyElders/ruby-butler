#!/bin/bash
# ShellSpec tests for directory completion nospace behavior
# Tests that bash completion for directories behaves correctly with trailing slashes

Describe "Ruby Butler Directory Completion Nospace"
  Include spec/support/helpers.sh

  Describe "bash completion script behavior"
    It "should add space after completing a partial directory name (allows next argument)"
      Skip "Manual test: source completion, type 'rb -C sp' then TAB"
    End

    It "should NOT add space when navigating within directory path (allows subdirectory completion)"
      Skip "Manual test: source completion, type 'rb -C spec/' then TAB"
    End
  End

  Describe "completion output correctness"
    It "outputs directory names with trailing slash"
      When run rb __bash_complete "rb -C sp" 9
      The output should include "spec/"
    End

    It "outputs subdirectories with full path and trailing slash"
      When run rb __bash_complete "rb -C spec/" 13
      The output should include "spec/behaviour/"
      The output should include "spec/commands/"
      The output should include "spec/support/"
    End
  End
End
