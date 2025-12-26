#!/bin/bash
# ShellSpec tests for Ruby Butler help command
# Distinguished validation of assistance and guidance features

Describe "Ruby Butler Help System"
  Include spec/support/helpers.sh

  Describe "help command (command-based interface)"
    Context "when invoked with 'help' command"
      It "presents distinguished usage information"
        When run rb help
        The status should equal 0
        The output should include "Usage"
      End

      It "elegantly displays available commands"
        When run rb help
        The status should equal 0
        The output should include "Commands"
      End

      It "gracefully presents available options"
        When run rb help
        The status should equal 0
        The output should include "Options"
      End

      It "mentions the distinguished runtime command"
        When run rb help
        The status should equal 0
        The output should include "runtime"
      End

      It "references the sophisticated exec command"
        When run rb help
        The status should equal 0
        The output should include "exec"
      End

      It "lists version as a command"
        When run rb help
        The status should equal 0
        The output should include "version"
      End

      It "lists help as a command"
        When run rb help
        The status should equal 0
        The output should include "help"
      End
    End

    Context "when requesting help for specific command"
      It "shows runtime command help"
        When run rb help runtime
        The status should equal 0
        The output should include "runtime"
      End

      It "shows environment command help"
        When run rb help environment
        The status should equal 0
        The output should include "environment"
      End

      It "shows exec command help"
        When run rb help exec
        The status should equal 0
        The output should include "exec"
      End

      It "shows sync command help"
        When run rb help sync
        The status should equal 0
        The output should include "sync"
      End

      It "shows run command help"
        When run rb help run
        The status should equal 0
        The output should include "run"
      End

      It "shows init command help"
        When run rb help init
        The status should equal 0
        The output should include "init"
      End

      It "shows config command help"
        When run rb help config
        The status should equal 0
        The output should include "config"
      End

      It "shows version command help"
        When run rb help version
        The status should equal 0
        The output should include "version"
      End

      It "shows shell-integration command help"
        When run rb help shell-integration
        The status should equal 0
        The output should include "shell-integration"
      End

      It "groups commands in main help"
        When run rb help
        The status should equal 0
        The output should include "Commands:"
        The output should include "Utility Commands:"
      End

      It "reports error for nonexistent command"
        When run rb help nonexistent
        The status should not equal 0
        The error should include "Unknown command"
      End
    End

    Context "when --help flag is used (deprecated)"
      It "rejects --help flag with error"
        When run rb --help
        The status should not equal 0
        The error should include "unexpected argument"
      End

      It "rejects -h flag with error"
        When run rb -h
        The status should not equal 0
        The error should include "unexpected argument"
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
