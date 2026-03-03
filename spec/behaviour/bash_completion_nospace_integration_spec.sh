#!/bin/bash
# ShellSpec tests for bash completion nospace integration
# Tests that the bash completion function logic is correct

Describe "Ruby Butler Bash Completion Nospace Integration"
  Include spec/support/helpers.sh

  Describe "compopt nospace behavior with directory completions"
    It "single directory completion does not add space (allows subdirectory navigation)"
      When run rb __bash_complete "rb -C sp" 9
      The status should equal 0
      The output should equal "spec/"
      The lines of output should equal 1
    End

    It "multiple directory completions do not add space"
      When run rb __bash_complete "rb -C spec/" 13
      The status should equal 0
      The line 1 of output should end with "/"
      The line 2 of output should end with "/"
      The line 3 of output should end with "/"
    End
  End

  Describe "completion script logic validation"
    It "generated script contains all_dirs flag"
      When run rb shell-integration bash
      The output should include "local all_dirs=true"
    End

    It "generated script applies nospace when all completions are directories"
      When run rb shell-integration bash
      # shellcheck disable=SC2016
      The output should include 'if [ "$all_dirs" = true ]; then'
      The output should include "compopt -o nospace"
    End
  End
End
