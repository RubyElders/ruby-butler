#!/bin/bash
# ShellSpec tests for Ruby Butler bash completion
# Distinguished validation of completion behavior

Describe "Ruby Butler Bash Completion"
  Include spec/support/helpers.sh

  Describe "__bash_complete subcommand"
    Context "command completion"
      It "suggests all commands when no prefix given"
        When run rb __bash_complete "rb " 3
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
        When run rb __bash_complete "rb ru" 5
        The status should equal 0
        The output should include "runtime"
        The output should include "run"
        The output should not include "exec"
        The output should not include "sync"
        The output should not include "environment"
      End

      It "filters commands by prefix 'e'"
        When run rb __bash_complete "rb e" 4
        The status should equal 0
        The output should include "exec"
        The output should include "x"
        The output should include "environment"
        The output should include "env"
        The output should not include "runtime"
        The output should not include "sync"
      End

      It "filters commands by prefix 'sh'"
        When run rb __bash_complete "rb sh" 5
        The status should equal 0
        The output should include "shell-integration"
        The output should not include "sync"
        The output should not include "runtime"
      End
    End

    Context "flag completion"
      It "suggests all flags when dash prefix given"
        When run rb __bash_complete "rb -" 4
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

      It "does not suggest hidden __bash_complete subcommand"
        When run rb __bash_complete "rb -" 4
        The status should equal 0
        The output should not include "__bash_complete"
      End
    End

    Context "Ruby version completion"
      It "suggests all Ruby versions after -r flag"
        When run rb __bash_complete "rb -r " 7 --rubies-dir "$RUBIES_DIR"
        The status should equal 0
        The output should include "$LATEST_RUBY"
        The output should include "$OLDER_RUBY"
        The output should not include "CRuby"
      End

      It "filters Ruby versions by prefix '3.4'"
        When run rb __bash_complete "rb -r 3.4" 9 --rubies-dir "$RUBIES_DIR"
        The status should equal 0
        The output should include "3.4"
        The output should not include "3.2"
        The output should not include "3.3"
      End

      It "suggests Ruby versions after --ruby flag"
        When run rb __bash_complete "rb --ruby " 10 --rubies-dir "$RUBIES_DIR"
        The status should equal 0
        The output should include "$LATEST_RUBY"
        The output should include "$OLDER_RUBY"
      End

      It "provides only version numbers without CRuby prefix"
        When run rb __bash_complete "rb -r " 7 --rubies-dir "$RUBIES_DIR"
        The status should equal 0
        The lines of output should not include "CRuby-3.4.5"
        The lines of output should not include "CRuby-3.2.4"
      End
    End

    Context "shell-integration completion"
      It "suggests only bash shell option"
        When run rb __bash_complete "rb shell-integration " 21
        The status should equal 0
        The output should include "bash"
        The output should not include "zsh"
        The output should not include "fish"
        The output should not include "powershell"
      End

      It "filters bash by prefix 'ba'"
        When run rb __bash_complete "rb shell-integration ba" 24
        The status should equal 0
        The output should include "bash"
      End

      It "returns nothing for non-matching prefix 'zs'"
        When run rb __bash_complete "rb shell-integration zs" 24
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
        When run rb __bash_complete "rb run " 7
        The status should equal 0
        The output should include "test"
        The output should include "build"
        The output should include "deploy"
        The output should include "dev"
      End

      It "filters scripts by prefix 'te'"
        cd "$PROJECT_DIR"
        When run rb __bash_complete "rb run te" 9
        The status should equal 0
        The output should include "test"
        The output should not include "build"
        The output should not include "deploy"
        The output should not include "dev"
      End

      It "filters scripts by prefix 'd'"
        cd "$PROJECT_DIR"
        When run rb __bash_complete "rb run d" 8
        The status should equal 0
        The output should include "deploy"
        The output should include "dev"
        The output should not include "test"
        The output should not include "build"
      End

      It "works with 'r' alias for run command"
        cd "$PROJECT_DIR"
        When run rb __bash_complete "rb r " 5
        The status should equal 0
        The output should include "test"
        The output should include "build"
        The output should include "deploy"
        The output should include "dev"
      End

      It "returns nothing when no rbproject.toml exists"
        cd "$SHELLSPEC_TMPBASE"
        When run rb __bash_complete "rb run " 7
        The status should equal 0
        The output should be blank
      End
    End

    Context "empty prefix handling"
      It "completes command after 'rb ' with space"
        When run rb __bash_complete "rb " 3
        The status should equal 0
        The output should include "runtime"
        The output should include "exec"
      End

      It "completes Ruby version after 'rb -r ' with space"
        When run rb __bash_complete "rb -r " 7 --rubies-dir "$RUBIES_DIR"
        The status should equal 0
        The output should include "$LATEST_RUBY"
      End

      It "completes shell after 'rb shell-integration ' with space"
        When run rb __bash_complete "rb shell-integration " 21
        The status should equal 0
        The output should include "bash"
      End
    End

    Context "cursor position handling"
      It "uses cursor position for completion context"
        When run rb __bash_complete "rb runtime --help" 3
        The status should equal 0
        The output should include "runtime"
      End

      It "completes at cursor position in middle of line"
        When run rb __bash_complete "rb ru --help" 5
        The status should equal 0
        The output should include "runtime"
        The output should include "run"
      End
    End

    Context "no completion scenarios"
      It "returns nothing for invalid command prefix"
        When run rb __bash_complete "rb xyz" 6
        The status should equal 0
        The output should be blank
      End

      It "returns binstubs for exec command with bundler project"
        setup_test_project
        create_bundler_project "."

        # Need to sync to create binstubs
        rb -R "$RUBIES_DIR" sync >/dev/null 2>&1

        When run rb __bash_complete "rb exec " 8
        The status should equal 0
        # After sync, rake binstub should be available
        The output should include "rake"
      End

      It "returns nothing after complete command"
        When run rb __bash_complete "rb runtime " 11
        The status should equal 0
        The output should be blank
      End
    End

    Context "special characters and edge cases"
      It "handles line without trailing space for partial word"
        When run rb __bash_complete "rb run" 6
        The status should equal 0
        The output should include "runtime"
        The output should include "run"
      End

      It "handles multiple spaces between words"
        When run rb __bash_complete "rb  runtime" 4
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

      It "includes __bash_complete callback in generated script"
        When run rb shell-integration bash
        The status should equal 0
        The output should include "rb __bash_complete"
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
        The output should include "COMPREPLY=(\$(compgen -W \"\$completions\" -- \"\$cur\"))"
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
        When run rb __bash_complete "rb " 3
        The status should equal 0
        # Just verify it completes without timeout
        The output should include "runtime"
      End

      It "completes Ruby versions quickly even with many versions"
        When run rb __bash_complete "rb -r " 7 --rubies-dir "$RUBIES_DIR"
        The status should equal 0
        # Just verify it completes without timeout
        The output should not be blank
      End
    End
  End

  Describe "integration with global flags"
    Context "completion works with global flags present"
      It "completes commands after global flags"
        When run rb __bash_complete "rb -v " 6
        The status should equal 0
        The output should include "runtime"
        The output should include "exec"
      End

      It "completes Ruby version with rubies-dir flag"
        When run rb __bash_complete "rb -R /opt/rubies -r " 23 --rubies-dir "$RUBIES_DIR"
        The status should equal 0
        The output should include "$LATEST_RUBY"
      End
    End
  End

  Describe "--no-bundler flag completion behavior"
    setup_bundler_test_project() {
      setup_test_project

      # Create Gemfile to simulate bundler project
      echo "source 'https://rubygems.org'" > "$TEST_PROJECT_DIR/Gemfile"

      # Create bundler binstubs directory with versioned ruby path using actual Ruby ABI
      local ruby_abi
      ruby_abi=$(get_ruby_abi_version "$LATEST_RUBY")
      BUNDLER_BIN="$TEST_PROJECT_DIR/.rb/vendor/bundler/ruby/$ruby_abi/bin"
      mkdir -p "$BUNDLER_BIN"

      # Create bundler-specific binstubs
      echo '#!/usr/bin/env ruby' > "$BUNDLER_BIN/bundler-tool"
      chmod +x "$BUNDLER_BIN/bundler-tool"

      echo '#!/usr/bin/env ruby' > "$BUNDLER_BIN/rails"
      chmod +x "$BUNDLER_BIN/rails"

      echo '#!/usr/bin/env ruby' > "$BUNDLER_BIN/rspec-bundler"
      chmod +x "$BUNDLER_BIN/rspec-bundler"
    }

    BeforeEach 'setup_bundler_test_project'
    AfterEach 'cleanup_test_project'

    Context "without -B flag in bundler project"
      It "suggests bundler binstubs"
        cd "$TEST_PROJECT_DIR"
        When run rb __bash_complete "rb exec b" 9
        The status should equal 0
        The output should include "bundler-tool"
      End

      It "suggests bundler binstubs with x alias"
        cd "$TEST_PROJECT_DIR"
        When run rb __bash_complete "rb x b" 6
        The status should equal 0
        The output should include "bundler-tool"
      End

      It "suggests multiple bundler binstubs with prefix 'r'"
        cd "$TEST_PROJECT_DIR"
        When run rb __bash_complete "rb exec r" 9
        The status should equal 0
        The output should include "rails"
        The output should include "rspec-bundler"
      End
    End

    Context "with -B flag in bundler project"
      It "skips bundler binstubs when -B flag present"
        cd "$TEST_PROJECT_DIR"
        When run rb -B __bash_complete "rb -B exec b" 12
        The status should equal 0
        The output should not include "bundler-tool"
      End

      It "skips bundler binstubs with --no-bundler flag"
        cd "$TEST_PROJECT_DIR"
        When run rb --no-bundler __bash_complete "rb --no-bundler exec b" 22
        The status should equal 0
        The output should not include "bundler-tool"
      End

      It "skips bundler binstubs with -B and x alias"
        cd "$TEST_PROJECT_DIR"
        When run rb -B __bash_complete "rb -B x b" 9
        The status should equal 0
        The output should not include "bundler-tool"
      End

      It "uses gem binstubs instead of bundler binstubs with -B"
        cd "$TEST_PROJECT_DIR"
        When run rb -B __bash_complete "rb -B x r" 9
        The status should equal 0
        The output should not include "rspec-bundler"
        The output should not include "rails"
        # Should show system gem binstubs instead (if any starting with 'r')
      End
    End

    Context "-B flag with -R flag combination"
      It "respects both -B and -R flags"
        cd "$TEST_PROJECT_DIR"
        When run rb -B -R "$RUBIES_DIR" __bash_complete "rb -B -R $RUBIES_DIR x b" 20
        The status should equal 0
        The output should not include "bundler-tool"
      End

      It "parses -R flag from command line for gem directory"
        cd "$TEST_PROJECT_DIR"
        # The -R flag should be passed as real CLI arg
        When run rb -R "$RUBIES_DIR" -B __bash_complete "rb -R $RUBIES_DIR -B x " 23
        The status should equal 0
        # Should complete but not from bundler
        The output should not include "bundler-tool"
      End
    End
  End
End
