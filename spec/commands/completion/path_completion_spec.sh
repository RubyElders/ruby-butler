#!/bin/bash
# ShellSpec tests for path-based completion
# Tests directory and file completion for path-based flags
#
# Note: These tests verify the completion OUTPUT (what rb __bash_complete returns).
# The bash completion function behavior (adding/not adding space) is controlled by
# the generated bash script which uses compopt -o nospace when $cur ends with /

Describe "Ruby Butler Path Completion"
  Include spec/support/helpers.sh

  Describe "directory flag completion with custom completers"
    Context "-R flag (rubies-dir) completion"
      It "suggests directories from current directory (not commands)"
        When run rb __bash_complete "rb -R " 7
        The status should equal 0
        # Should NOT suggest subcommands like "runtime"
        # In Docker container, /app has spec/ directory mounted
        The first line of output should not equal "runtime"
      End

      It "completes partial directory paths"
        When run rb __bash_complete "rb -R sp" 9
        The status should equal 0
        # Should complete to spec/ directory
        The output should include "spec/"
      End
    End

    Context "-C flag (work-dir) completion"
      It "suggests directories from current directory (not commands)"
        When run rb __bash_complete "rb -C " 7
        The status should equal 0
        The first line of output should not equal "runtime"
      End

      It "completes partial directory path and suggests subdirectories"
        When run rb __bash_complete "rb -C sp" 9
        The status should equal 0
        The output should include "spec/"
      End

      It "suggests subdirectories after completing a directory"
        When run rb __bash_complete "rb -C spec/" 13
        The status should equal 0
        The output should include "spec/behaviour/"
        The output should include "spec/commands/"
        The output should include "spec/support/"
      End
    End

    Context "-G flag (gem-home) completion"
      It "suggests directories from current directory (not commands)"
        When run rb __bash_complete "rb -G " 7
        The status should equal 0
        The first line of output should not equal "runtime"
      End
    End

    Context "-c flag (config file) completion"
      It "suggests files and directories from current directory (not commands)"
        When run rb __bash_complete "rb -c " 7
        The status should equal 0
        # Should see files like rb (binary) and .shellspec file
        The first line of output should not equal "runtime"
      End
    End

    Context "-P flag (project file) completion"
      It "suggests files and directories from current directory (not commands)"
        When run rb __bash_complete "rb -P " 7
        The status should equal 0
        The first line of output should not equal "runtime"
      End
    End
  End

  Describe "environment variable isolation"
    Context "completion without env vars"
      # CURRENT LIMITATION: CLAP completers run before CLI parsing
      # They cannot see flag values from the command line being completed
      # They only see environment variables and default paths

      It "completer uses default rubies directory (cannot see --rubies-dir in line)"
        Skip "CLAP completers cannot access --rubies-dir from command line yet"
        unset RB_RUBIES_DIR

        # This documents desired behavior - currently not supported
        When run rb __bash_complete "rb --rubies-dir $RUBIES_DIR -r " 35
        The status should equal 0
        The output should include "$LATEST_RUBY"
      End

      It "completer uses default path (cannot see -R in line)"
        Skip "CLAP completers cannot access -R flag from command line yet"
        unset RB_RUBIES_DIR

        When run rb __bash_complete "rb -R $RUBIES_DIR -r " 30
        The status should equal 0
        The output should include "$LATEST_RUBY"
      End

      It "uses RB_RUBIES_DIR environment variable if set"
        export RB_RUBIES_DIR="$RUBIES_DIR"

        # Completers CAN see environment variables
        When run rb __bash_complete "rb -r " 7
        The status should equal 0
        The output should include "$LATEST_RUBY"
        The output should include "$OLDER_RUBY"

        unset RB_RUBIES_DIR
      End

      It "uses default ~/.rubies path when no env var set"
        unset RB_RUBIES_DIR

        When run rb __bash_complete "rb -r " 7
        The status should equal 0
        # Should complete if ~/.rubies exists, empty if not
      End
    End
  End
End
