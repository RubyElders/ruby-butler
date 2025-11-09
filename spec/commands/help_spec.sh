#!/bin/bash
# ShellSpec tests for Ruby Butler help command
# Distinguished validation of assistance and guidance features

Describe "Ruby Butler Help System"
  Include spec/support/helpers.sh

  Describe "help command"
    Context "when invoked with --help flag"
      It "presents distinguished usage information"
        When run rb --help
        The status should equal 0
        The output should include "Usage"
      End

      It "elegantly displays available commands"
        When run rb --help
        The status should equal 0
        The output should include "Commands"
      End

      It "gracefully presents available options"
        When run rb --help
        The status should equal 0
        The output should include "Options"
      End

      It "mentions the distinguished runtime command"
        When run rb --help
        The status should equal 0
        The output should include "runtime"
      End

      It "references the sophisticated exec command"
        When run rb --help
        The status should equal 0
        The output should include "exec"
      End
    End

    Context "when no arguments are provided"
      It "gracefully displays help with appropriate exit code"
        When run rb
        The status should equal 0
        The output should include "Usage"
        The output should include "Commands"
      End
    End
  End
End
