#!/bin/bash
# ShellSpec tests for Ruby Butler exec command - Completion behavior
# Distinguished validation of completion with --no-bundler flag

Describe "Ruby Butler Exec Command - Completion Behavior"
  Include spec/support/helpers.sh

  Describe "completion with bundler project"
    setup_bundler_with_binstubs() {
      setup_test_project
      create_bundler_project "."

      # Create bundler binstubs directory with versioned ruby path
      BUNDLER_BIN="$TEST_PROJECT_DIR/.rb/vendor/bundler/ruby/3.3.0/bin"
      mkdir -p "$BUNDLER_BIN"

      # Create bundler-specific binstubs
      echo '#!/usr/bin/env ruby' > "$BUNDLER_BIN/bundler-tool"
      chmod +x "$BUNDLER_BIN/bundler-tool"

      echo '#!/usr/bin/env ruby' > "$BUNDLER_BIN/rails"
      chmod +x "$BUNDLER_BIN/rails"

      echo '#!/usr/bin/env ruby' > "$BUNDLER_BIN/rspec-bundler"
      chmod +x "$BUNDLER_BIN/rspec-bundler"
    }

    BeforeEach 'setup_bundler_with_binstubs'
    AfterEach 'cleanup_test_project'

    Context "without --no-bundler flag"
      It "suggests bundler binstubs for exec command"
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

    Context "with -B flag"
      It "shows bundler binstubs WITHOUT -B flag"
        cd "$TEST_PROJECT_DIR"
        When run rb __bash_complete "rb exec b" 9
        The status should equal 0
        The output should include "bundler-tool"
      End

      It "skips bundler binstubs WITH -B flag"
        cd "$TEST_PROJECT_DIR"
        When run rb __bash_complete "rb -B exec b" 12
        The status should equal 0
        The output should not include "bundler-tool"
      End

      It "skips bundler binstubs with --no-bundler flag"
        cd "$TEST_PROJECT_DIR"
        When run rb __bash_complete "rb --no-bundler exec b" 22
        The status should equal 0
        The output should not include "bundler-tool"
      End

      It "skips bundler binstubs with -B and x alias"
        cd "$TEST_PROJECT_DIR"
        When run rb __bash_complete "rb -B x b" 9
        The status should equal 0
        The output should not include "bundler-tool"
      End

      It "shows rspec-bundler WITHOUT -B flag"
        cd "$TEST_PROJECT_DIR"
        When run rb __bash_complete "rb x r" 6
        The status should equal 0
        The output should include "rspec-bundler"
      End

      It "skips rspec-bundler WITH -B flag"
        cd "$TEST_PROJECT_DIR"
        When run rb __bash_complete "rb -B x r" 9
        The status should equal 0
        The output should not include "rspec-bundler"
      End
    End

    Context "with -B and -R flags combined"
      It "respects both -B and -R flags"
        cd "$TEST_PROJECT_DIR"
        When run rb __bash_complete "rb -B -R $RUBIES_DIR x b" 27
        The status should equal 0
        The output should not include "bundler-tool"
      End

      It "parses -R flag from command line"
        cd "$TEST_PROJECT_DIR"
        When run rb __bash_complete "rb -R $RUBIES_DIR -B x b" 27
        The status should equal 0
        The output should not include "bundler-tool"
      End
    End
  End

  Describe "completion without bundler project"
    BeforeEach 'setup_test_project'
    AfterEach 'cleanup_test_project'

    Context "in directory without Gemfile"
      It "suggests gem binstubs from system"
        cd "$TEST_PROJECT_DIR"
        When run rb __bash_complete "rb exec r" 9
        The status should equal 0
        # Should complete with gem binstubs if any exist
        # No bundler project detected, so uses gem runtime
      End

      It "works with -B flag even without bundler"
        cd "$TEST_PROJECT_DIR"
        When run rb __bash_complete "rb -B exec r" 12
        The status should equal 0
        # Should still work, just uses gem binstubs
      End
    End
  End
End
