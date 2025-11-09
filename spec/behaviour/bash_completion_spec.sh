#!/bin/bash
# ShellSpec tests for Ruby Butler bash completion
# Distinguished validation of completion behavior

Describe "Ruby Butler Bash Completion"
  Include spec/support/helpers.sh

  Describe "--complete flag"
    Context "command completion"
      It "suggests all commands when no prefix given"
        When run rb --complete "rb " 3
        The status should equal 0
        The output should include "runtime"
        The output should include "rt"
        The output should include "environment"
        The output should include "env"
        The output should include "exec"
        The output should include "x"
        The output should include "sync"
        The output should include "s"
        The output should include "run"
        The output should include "r"
        The output should include "init"
        The output should include "shell-integration"
      End

      It "filters commands by prefix 'ru'"
        When run rb --complete "rb ru" 5
        The status should equal 0
        The output should include "runtime"
        The output should include "run"
        The output should not include "exec"
        The output should not include "sync"
        The output should not include "environment"
      End

      It "filters commands by prefix 'e'"
        When run rb --complete "rb e" 4
        The status should equal 0
        The output should include "exec"
        The output should include "x"
        The output should include "environment"
        The output should include "env"
        The output should not include "runtime"
        The output should not include "sync"
      End

      It "filters commands by prefix 'sh'"
        When run rb --complete "rb sh" 5
        The status should equal 0
        The output should include "shell-integration"
        The output should not include "sync"
        The output should not include "runtime"
      End
    End

    Context "flag completion"
      It "suggests all flags when dash prefix given"
        When run rb --complete "rb -" 4
        The status should equal 0
        The output should include "-v"
        The output should include "--verbose"
        The output should include "-r"
        The output should include "--ruby"
        The output should include "-R"
        The output should include "--rubies-dir"
        The output should include "-c"
        The output should include "--config"
        The output should include "-P"
        The output should include "--project"
        The output should include "-G"
        The output should include "--gem-home"
        The output should include "-B"
        The output should include "--no-bundler"
      End

      It "does not suggest hidden --complete flag"
        When run rb --complete "rb -" 4
        The status should equal 0
        The output should not include "--complete"
      End
    End

    Context "Ruby version completion"
      It "suggests all Ruby versions after -r flag"
        When run rb --complete "rb -r " 7 --rubies-dir "$RUBIES_DIR"
        The status should equal 0
        The output should include "$LATEST_RUBY"
        The output should include "$OLDER_RUBY"
        The output should not include "CRuby"
      End

      It "filters Ruby versions by prefix '3.4'"
        When run rb --complete "rb -r 3.4" 9 --rubies-dir "$RUBIES_DIR"
        The status should equal 0
        The output should include "3.4"
        The output should not include "3.2"
        The output should not include "3.3"
      End

      It "suggests Ruby versions after --ruby flag"
        When run rb --complete "rb --ruby " 10 --rubies-dir "$RUBIES_DIR"
        The status should equal 0
        The output should include "$LATEST_RUBY"
        The output should include "$OLDER_RUBY"
      End

      It "provides only version numbers without CRuby prefix"
        When run rb --complete "rb -r " 7 --rubies-dir "$RUBIES_DIR"
        The status should equal 0
        The lines of output should not include "CRuby-3.4.5"
        The lines of output should not include "CRuby-3.2.4"
      End
    End

    Context "shell-integration completion"
      It "suggests only bash shell option"
        When run rb --complete "rb shell-integration " 21
        The status should equal 0
        The output should include "bash"
        The output should not include "zsh"
        The output should not include "fish"
        The output should not include "powershell"
      End

      It "filters bash by prefix 'ba'"
        When run rb --complete "rb shell-integration ba" 24
        The status should equal 0
        The output should include "bash"
      End

      It "returns nothing for non-matching prefix 'zs'"
        When run rb --complete "rb shell-integration zs" 24
        The status should equal 0
        The output should be blank
      End
    End

    Context "script completion from rbproject.toml"
      setup_project_with_scripts() {
        PROJECT_DIR="$SHELLSPEC_TMPBASE/test-project"
        mkdir -p "$PROJECT_DIR"
        cat > "$PROJECT_DIR/rbproject.toml" << 'EOF'
[project]
ruby = "3.4.5"

[scripts]
test = "bundle exec rspec"
build = "rake build"
deploy = "cap production deploy"
dev = "rails server"
EOF
      }

      BeforeEach 'setup_project_with_scripts'

      It "suggests all scripts after 'run' command"
        cd "$PROJECT_DIR"
        When run rb --complete "rb run " 7
        The status should equal 0
        The output should include "test"
        The output should include "build"
        The output should include "deploy"
        The output should include "dev"
      End

      It "filters scripts by prefix 'te'"
        cd "$PROJECT_DIR"
        When run rb --complete "rb run te" 9
        The status should equal 0
        The output should include "test"
        The output should not include "build"
        The output should not include "deploy"
        The output should not include "dev"
      End

      It "filters scripts by prefix 'd'"
        cd "$PROJECT_DIR"
        When run rb --complete "rb run d" 8
        The status should equal 0
        The output should include "deploy"
        The output should include "dev"
        The output should not include "test"
        The output should not include "build"
      End

      It "works with 'r' alias for run command"
        cd "$PROJECT_DIR"
        When run rb --complete "rb r " 5
        The status should equal 0
        The output should include "test"
        The output should include "build"
        The output should include "deploy"
        The output should include "dev"
      End

      It "returns nothing when no rbproject.toml exists"
        cd "$SHELLSPEC_TMPBASE"
        When run rb --complete "rb run " 7
        The status should equal 0
        The output should be blank
      End
    End

    Context "empty prefix handling"
      It "completes command after 'rb ' with space"
        When run rb --complete "rb " 3
        The status should equal 0
        The output should include "runtime"
        The output should include "exec"
      End

      It "completes Ruby version after 'rb -r ' with space"
        When run rb --complete "rb -r " 7 --rubies-dir "$RUBIES_DIR"
        The status should equal 0
        The output should include "$LATEST_RUBY"
      End

      It "completes shell after 'rb shell-integration ' with space"
        When run rb --complete "rb shell-integration " 21
        The status should equal 0
        The output should include "bash"
      End
    End

    Context "cursor position handling"
      It "uses cursor position for completion context"
        When run rb --complete "rb runtime --help" 3
        The status should equal 0
        The output should include "runtime"
      End

      It "completes at cursor position in middle of line"
        When run rb --complete "rb ru --help" 5
        The status should equal 0
        The output should include "runtime"
        The output should include "run"
      End
    End

    Context "no completion scenarios"
      It "returns nothing for invalid command prefix"
        When run rb --complete "rb xyz" 6
        The status should equal 0
        The output should be blank
      End

      It "returns nothing for exec command args"
        When run rb --complete "rb exec bundle " 16
        The status should equal 0
        The output should be blank
      End

      It "returns nothing after complete command"
        When run rb --complete "rb runtime " 11
        The status should equal 0
        The output should be blank
      End
    End

    Context "special characters and edge cases"
      It "handles line without trailing space for partial word"
        When run rb --complete "rb run" 6
        The status should equal 0
        The output should include "runtime"
        The output should include "run"
      End

      It "handles multiple spaces between words"
        When run rb --complete "rb  runtime" 4
        The status should equal 0
        The output should include "runtime"
      End
    End
  End

  Describe "shell-integration command"
    Context "bash completion script generation"
      It "generates bash completion script"
        When run rb shell-integration bash
        The status should equal 0
        The output should include "_rb_completion"
        The output should include "complete -F _rb_completion rb"
      End

      It "includes --complete callback in generated script"
        When run rb shell-integration bash
        The status should equal 0
        The output should include "rb --complete"
      End

      It "does not show instructions when output is piped"
        When run rb shell-integration bash
        The status should equal 0
        The output should include "_rb_completion"
        The stderr should equal ""
      End

      It "uses COMP_LINE and COMP_POINT variables"
        When run rb shell-integration bash
        The status should equal 0
        The output should include "COMP_LINE"
        The output should include "COMP_POINT"
      End

      It "uses compgen for word completion"
        When run rb shell-integration bash
        The status should equal 0
        The output should include 'COMPREPLY=($(compgen -W "$completions" -- "$cur"))'
      End

      It "includes fallback to default bash completion"
        When run rb shell-integration bash
        The status should equal 0
        The output should include "compopt -o default"
        The output should include "# No rb completions, fall back to default bash completion"
      End
    End
  End

  Describe "performance characteristics"
    Context "completion speed"
      It "completes commands quickly"
        When run rb --complete "rb " 3
        The status should equal 0
        # Just verify it completes without timeout
        The output should include "runtime"
      End

      It "completes Ruby versions quickly even with many versions"
        When run rb --complete "rb -r " 7 --rubies-dir "$RUBIES_DIR"
        The status should equal 0
        # Just verify it completes without timeout
        The output should not be blank
      End
    End
  End

  Describe "integration with global flags"
    Context "completion works with global flags present"
      It "completes commands after global flags"
        When run rb --complete "rb -v " 6
        The status should equal 0
        The output should include "runtime"
        The output should include "exec"
      End

      It "completes Ruby version with rubies-dir flag"
        When run rb --complete "rb -R /opt/rubies -r " 23 --rubies-dir "$RUBIES_DIR"
        The status should equal 0
        The output should include "$LATEST_RUBY"
      End
    End
  End
End
