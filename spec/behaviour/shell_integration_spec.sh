#!/bin/bash

Describe "Ruby Butler Shell Integration Display"
  Include spec/support/helpers.sh

  Context "when run without shell argument"
    It "shows available integrations header"
      When run rb shell-integration
      The status should equal 0
      The output should include "🎩 Available Shell Integrations"
    End

    It "displays Shells section"
      When run rb shell-integration
      The status should equal 0
      The output should include "Shells:"
    End

    It "displays Installation section"
      When run rb shell-integration
      The status should equal 0
      The output should include "Installation:"
    End

    It "lists bash shell"
      When run rb shell-integration
      The status should equal 0
      The output should include "bash"
    End

    It "shows bash description"
      When run rb shell-integration
      The status should equal 0
      The output should include "Dynamic command completion for Bash shell"
    End

    It "shows bash installation instruction"
      When run rb shell-integration
      The status should equal 0
      The output should include "Add to ~/.bashrc"
      The output should include "eval"
      The output should include "rb shell-integration bash"
    End

    It "exits with success status"
      When run rb shell-integration
      The status should equal 0
      The output should include "Shells:"
    End
  End

  Context "when requesting help"
    It "shows help for shell-integration command"
      When run rb help shell-integration
      The status should equal 0
      The output should include "Generate shell integration (completions)"
      The output should include "Usage: shell-integration"
    End

    It "shows shell argument is optional"
      When run rb help shell-integration
      The status should equal 0
      The output should include "[SHELL]"
    End

    It "lists bash as possible value"
      When run rb help shell-integration
      The status should equal 0
      The output should include "possible values: bash"
    End
  End

  Context "when run with bash argument"
    It "generates bash completion script"
      When run rb shell-integration bash
      The status should equal 0
      The output should include "_rb_completion"
      The output should include "complete -F _rb_completion rb"
    End

    It "does not show the integrations list"
      When run rb shell-integration bash
      The status should equal 0
      The output should not include "Available Shell Integrations"
      The output should not include "Shells:"
    End

    It "does not show instructions when piped"
      When run rb shell-integration bash
      The status should equal 0
      The output should include "_rb_completion"
      The stderr should equal ""
    End
  End
End
