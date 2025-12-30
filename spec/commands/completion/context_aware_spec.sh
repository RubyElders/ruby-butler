#!/bin/bash
# ShellSpec tests for context-aware completion
# Tests that completion respects flags that modify runtime behavior

Describe "Ruby Butler Context-Aware Completion"
  Include spec/support/helpers.sh

  Describe "Ruby version completion with custom rubies-dir"
    setup_custom_rubies() {
      CUSTOM_RUBIES_DIR="$SHELLSPEC_TMPBASE/custom-rubies"
      mkdir -p "$CUSTOM_RUBIES_DIR/ruby-2.7.8/bin"
      mkdir -p "$CUSTOM_RUBIES_DIR/ruby-9.9.9/bin"

      # Create mock ruby executables
      echo '#!/bin/bash' > "$CUSTOM_RUBIES_DIR/ruby-2.7.8/bin/ruby"
      echo 'echo "ruby 2.7.8"' >> "$CUSTOM_RUBIES_DIR/ruby-2.7.8/bin/ruby"
      chmod +x "$CUSTOM_RUBIES_DIR/ruby-2.7.8/bin/ruby"

      echo '#!/bin/bash' > "$CUSTOM_RUBIES_DIR/ruby-9.9.9/bin/ruby"
      echo 'echo "ruby 9.9.9"' >> "$CUSTOM_RUBIES_DIR/ruby-9.9.9/bin/ruby"
      chmod +x "$CUSTOM_RUBIES_DIR/ruby-9.9.9/bin/ruby"
    }

    cleanup_custom_rubies() {
      rm -rf "$CUSTOM_RUBIES_DIR"
    }

    BeforeEach 'setup_custom_rubies'
    AfterEach 'cleanup_custom_rubies'

    Context "-R flag affects -r completion"
      # CURRENT LIMITATION: CLAP completers don't have access to -R flag value
      # This test documents the desired behavior for future implementation

      It "should complete Ruby versions from custom rubies-dir (TODO)"
        Skip "CLAP completers cannot access parsed flag values yet"
        When run rb __bash_complete "rb -R $CUSTOM_RUBIES_DIR -r " 7
        The status should equal 0
        The output should include "2.7.8"
        The output should include "9.9.9"
        The output should not include "$LATEST_RUBY"
      End

      It "uses default rubies-dir when -R not provided"
        When run rb __bash_complete "rb -r " 7 --rubies-dir "$RUBIES_DIR"
        The status should equal 0
        The output should include "$LATEST_RUBY"
        The output should not include "2.7.8"
        The output should not include "9.9.9"
      End
    End

    Context "long form --rubies-dir affects completion"
      It "should complete Ruby versions from --rubies-dir path (TODO)"
        Skip "CLAP completers cannot access parsed flag values yet"
        When run rb __bash_complete "rb --rubies-dir $CUSTOM_RUBIES_DIR --ruby " 10
        The status should equal 0
        The output should include "2.7.8"
        The output should include "9.9.9"
      End
    End
  End

  Describe "Exec completion with -C flag (work-dir)"
    setup_custom_workdir() {
      CUSTOM_WORKDIR="$SHELLSPEC_TMPBASE/custom-work"
      mkdir -p "$CUSTOM_WORKDIR"

      # Create Gemfile for bundler project
      echo 'source "https://rubygems.org"' > "$CUSTOM_WORKDIR/Gemfile"

      # Create bundler binstubs using actual Ruby ABI
      ruby_abi=$(get_ruby_abi_version "$LATEST_RUBY")
      CUSTOM_BUNDLER_BIN="$CUSTOM_WORKDIR/.rb/vendor/bundler/ruby/$ruby_abi/bin"
      mkdir -p "$CUSTOM_BUNDLER_BIN"

      echo '#!/usr/bin/env ruby' > "$CUSTOM_BUNDLER_BIN/custom-tool"
      chmod +x "$CUSTOM_BUNDLER_BIN/custom-tool"

      echo '#!/usr/bin/env ruby' > "$CUSTOM_BUNDLER_BIN/special-script"
      chmod +x "$CUSTOM_BUNDLER_BIN/special-script"
    }

    cleanup_custom_workdir() {
      rm -rf "$CUSTOM_WORKDIR"
    }

    BeforeEach 'setup_custom_workdir'
    AfterEach 'cleanup_custom_workdir'

    Context "-C flag affects exec completion"
      # CURRENT LIMITATION: CLAP completers run in current directory
      # They don't have access to -C flag value

      It "should discover binstubs relative to -C directory (TODO)"
        Skip "CLAP completers cannot access -C flag value yet"
        When run rb __bash_complete "rb -C $CUSTOM_WORKDIR exec cu" 9
        The status should equal 0
        The output should include "custom-tool"
        The output should not include "bundle"
      End

      It "discovers binstubs in current directory without -C"
        setup_test_project
        create_bundler_project "."

        ruby_abi=$(get_ruby_abi_version "$LATEST_RUBY")
        BUNDLER_BIN="$TEST_PROJECT_DIR/.rb/vendor/bundler/ruby/$ruby_abi/bin"
        mkdir -p "$BUNDLER_BIN"

        echo '#!/usr/bin/env ruby' > "$BUNDLER_BIN/current-dir-tool"
        chmod +x "$BUNDLER_BIN/current-dir-tool"

        cd "$TEST_PROJECT_DIR"
        When run rb __bash_complete "rb exec cu" 9
        The status should equal 0
        The output should include "current-dir-tool"

        cleanup_test_project
      End
    End
  End

  Describe "Combined flag context"
    setup_combined_context() {
      CUSTOM_RUBIES_DIR="$SHELLSPEC_TMPBASE/ctx-rubies"
      CUSTOM_WORKDIR="$SHELLSPEC_TMPBASE/ctx-work"

      mkdir -p "$CUSTOM_RUBIES_DIR/ruby-8.8.8/bin"
      mkdir -p "$CUSTOM_WORKDIR"

      echo '#!/bin/bash' > "$CUSTOM_RUBIES_DIR/ruby-8.8.8/bin/ruby"
      echo 'echo "ruby 8.8.8"' >> "$CUSTOM_RUBIES_DIR/ruby-8.8.8/bin/ruby"
      chmod +x "$CUSTOM_RUBIES_DIR/ruby-8.8.8/bin/ruby"

      echo 'source "https://rubygems.org"' > "$CUSTOM_WORKDIR/Gemfile"
    }

    cleanup_combined_context() {
      rm -rf "$CUSTOM_RUBIES_DIR" "$CUSTOM_WORKDIR"
    }

    BeforeEach 'setup_combined_context'
    AfterEach 'cleanup_combined_context'

    Context "multiple flags affecting completion"
      It "should use both -R and -C for exec completion (TODO)"
        Skip "CLAP completers cannot access multiple flag context yet"
        When run rb __bash_complete "rb -R $CUSTOM_RUBIES_DIR -C $CUSTOM_WORKDIR -r " 7
        The status should equal 0
        The output should include "8.8.8"
      End
    End
  End

  Describe "Dockerfile rubies path testing"
    Context "non-standard rubies location"
      It "completes rubies from /opt/rubies when explicitly specified"
        # Test with Docker-style path (Docker has 3.2.4 and 3.4.5)
        When run rb __bash_complete "rb -r " 7 --rubies-dir "/opt/rubies"
        The status should equal 0
        The output should include "3.4.5"
        The output should include "3.2.4"
      End
    End
  End
End
