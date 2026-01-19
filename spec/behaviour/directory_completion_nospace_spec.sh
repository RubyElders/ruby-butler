#!/bin/bash
# ShellSpec tests for directory completion nospace behavior
# Tests that bash completion for directories behaves correctly with trailing slashes

Describe "Ruby Butler Directory Completion Nospace"
  Include spec/support/helpers.sh

  Describe "bash completion script behavior"
    It "should add space after completing a partial directory name (allows next argument)"
      # When completing "rb -C sp<TAB>" -> "rb -C spec/ "
      # Bash SHOULD add a space because user might want to continue with next arg
      # The completion function detects $cur doesn't end with / yet

      Skip "Manual test: source completion, type 'rb -C sp' then TAB"

      # Expected: "rb -C spec/ " (with space)
      # This allows: "rb -C spec/ run" or other commands
    End

    It "should NOT add space when navigating within directory path (allows subdirectory completion)"
      # When completing "rb -C spec/<TAB>" -> suggests subdirs
      # Bash should NOT add space because $cur ends with /
      # This allows continued navigation: "rb -C spec/commands/<TAB>"

      Skip "Manual test: source completion, type 'rb -C spec/' then TAB"

      # Expected: suggests "spec/behaviour/", "spec/commands/", "spec/support/"
      # Then "rb -C spec/commands/" (no space) allows further TAB completion
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
