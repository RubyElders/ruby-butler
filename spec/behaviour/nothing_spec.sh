#!/bin/bash

Describe "Ruby Butler No Command Behavior"
  Include spec/support/helpers.sh

  Context "when run without arguments"
    It "shows help message"
      When run rb
      The status should equal 0
      The output should include "Usage: rb [OPTIONS] COMMAND [COMMAND_OPTIONS]"
    End

    It "displays all available commands"
      When run rb
      The status should equal 0
      The output should include "Commands:"
      The output should include "runtime"
      The output should include "environment"
      The output should include "exec"
      The output should include "sync"
      The output should include "run"
      The output should include "init"
      The output should include "shell-integration"
    End

    It "displays command aliases"
      When run rb
      The status should equal 0
      The output should include "[aliases: rt]"
      The output should include "[aliases: env]"
      The output should include "[aliases: x]"
      The output should include "[aliases: s]"
      The output should include "[aliases: r]"
    End

    It "displays global options"
      When run rb
      The status should equal 0
      The output should include "Options:"
      The output should include "--log-level"
      The output should include "--verbose"
      The output should include "--config"
      The output should include "--ruby"
      The output should include "--rubies-dir"
      The output should include "--gem-home"
      The output should include "--no-bundler"
    End

    It "shows Ruby Butler title with emoji"
      When run rb
      The status should equal 0
      The output should include "🎩 Ruby Butler"
    End

    It "describes itself as environment manager"
      When run rb
      The status should equal 0
      The output should include "Ruby environment manager"
    End

    It "includes help command"
      When run rb
      The status should equal 0
      The output should include "help"
    End

    It "includes version command"
      When run rb
      The status should equal 0
      The output should include "version"
    End
  End

  Context "when run with help command"
    It "shows help information"
      When run rb help
      The status should equal 0
      The output should include "Usage: rb [OPTIONS] COMMAND [COMMAND_OPTIONS]"
      The output should include "Commands:"
    End
  End
End
