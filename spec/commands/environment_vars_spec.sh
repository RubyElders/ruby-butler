#!/bin/bash
# ShellSpec tests for Ruby Butler environment variables
# Distinguished validation of systematic RB_* environment variable support

Describe "Ruby Butler Environment Variables"
  Include spec/support/helpers.sh

  Describe "verbose flags via environment variables"
    Context "when RB_VERBOSE is set"
      It "enables informational logging"
        export RB_VERBOSE=true
        When run rb -R "$RUBIES_DIR" runtime
        The status should equal 0
        The output should include "Ruby Environment Survey"
        The stderr should include "[INFO ]"
        The stderr should include "Discovered"
      End

      It "works with any truthy value"
        export RB_VERBOSE=true
        When run rb -R "$RUBIES_DIR" version
        The status should equal 0
        The output should include "Ruby Butler"
      End
    End

    Context "when RB_VERY_VERBOSE is set"
      It "enables comprehensive diagnostic logging"
        export RB_VERY_VERBOSE=true
        When run rb -R "$RUBIES_DIR" runtime
        The status should equal 0
        The output should include "Ruby Environment Survey"
        The stderr should include "[DEBUG]"
      End
    End

    Context "when RB_LOG_LEVEL is set"
      It "respects explicit log level"
        export RB_LOG_LEVEL=info
        When run rb -R "$RUBIES_DIR" runtime
        The status should equal 0
        The output should include "Ruby Environment Survey"
        The stderr should include "[INFO ]"
      End

      It "accepts debug level"
        export RB_LOG_LEVEL=debug
        When run rb -R "$RUBIES_DIR" runtime
        The status should equal 0
        The output should include "Ruby Environment Survey"
        The stderr should include "[DEBUG]"
      End

      It "accepts none level for silence"
        export RB_LOG_LEVEL=none
        When run rb -R "$RUBIES_DIR" runtime
        The status should equal 0
        The stdout should include "Ruby Environment Survey"
        The stderr should not include "[INFO ]"
        The stderr should not include "[DEBUG]"
      End
    End

    Context "verbose flag precedence"
      It "prioritizes -V over RB_VERBOSE"
        export RB_VERBOSE=true
        When run rb -R "$RUBIES_DIR" -V runtime
        The status should equal 0
        The output should include "Ruby Environment Survey"
        The stderr should include "[DEBUG]"
      End

      It "prioritizes -v over RB_LOG_LEVEL"
        export RB_LOG_LEVEL=none
        When run rb -R "$RUBIES_DIR" -v runtime
        The status should equal 0
        The output should include "Ruby Environment Survey"
        The stderr should include "[INFO ]"
      End
    End
  End

  Describe "configuration via environment variables"
    Context "when RB_RUBIES_DIR is set"
      It "uses specified rubies directory"
        export RB_RUBIES_DIR="$RUBIES_DIR"
        When run rb runtime
        The status should equal 0
        The output should include "$LATEST_RUBY"
      End

      It "can be overridden by CLI flag"
        export RB_RUBIES_DIR="/nonexistent"
        When run rb -R "$RUBIES_DIR" runtime
        The status should equal 0
        The output should include "$LATEST_RUBY"
      End
    End

    Context "when RB_RUBY_VERSION is set"
      It "selects specified Ruby version"
        export RB_RUBY_VERSION="$OLDER_RUBY"
        When run rb -R "$RUBIES_DIR" environment
        The status should equal 0
        The output should include "$OLDER_RUBY"
      End

      It "can be overridden by CLI flag"
        export RB_RUBY_VERSION="$OLDER_RUBY"
        When run rb -R "$RUBIES_DIR" -r "$LATEST_RUBY" environment
        The status should equal 0
        The output should include "$LATEST_RUBY"
        The output should not include "$OLDER_RUBY"
      End
    End

    Context "when RB_GEM_HOME is set"
      It "uses specified gem home directory"
        export RB_GEM_HOME="/tmp/test-gems"
        When run rb -R "$RUBIES_DIR" environment
        The status should equal 0
        The output should include "/tmp/test-gems"
      End
    End

    Context "when RB_NO_BUNDLER is set"
      It "disables bundler integration"
        export RB_NO_BUNDLER=true
        When run rb -R "$RUBIES_DIR" config
        The status should equal 0
        The output should include "No Bundler: yes"
      End
    End

    Context "when RB_WORK_DIR is set"
      It "changes working directory before command execution"
        mkdir -p /tmp/rb-workdir-test
        echo "test-marker" > /tmp/rb-workdir-test/marker.txt
        export RB_WORK_DIR="/tmp/rb-workdir-test"
        export RB_RUBIES_DIR="$RUBIES_DIR"
        When run rb init
        The status should equal 0
        The stdout should include "rbproject.toml has been created"
        The file "/tmp/rb-workdir-test/rbproject.toml" should be exist
      End
    End

    Context "when RB_CONFIG is set"
      It "uses specified config file location"
        mkdir -p /tmp/rb-config-test
        cat > /tmp/rb-config-test/test.toml << 'EOF'
rubies-dir = "/custom/from/config"
EOF
        unset RB_RUBIES_DIR
        export RB_CONFIG="/tmp/rb-config-test/test.toml"
        When run rb runtime
        The status should not equal 0
        The stdout should equal ""
        The stderr should include "/custom/from/config"
      End
    End

    Context "when RB_PROJECT is set"
      It "uses specified project file location"
        mkdir -p /tmp/rb-project-test
        cat > /tmp/rb-project-test/custom.toml << 'EOF'
[scripts]
test-script = "echo test"
EOF
        export RB_PROJECT="/tmp/rb-project-test/custom.toml"
        When run rb -R "$RUBIES_DIR" run
        The status should equal 0
        The output should include "test-script"
      End
    End
  End

  Describe "environment variable display in help"
    Context "when showing help"
      It "documents RB_VERBOSE environment variable"
        When run rb help
        The status should equal 0
        The output should include "[env: RB_VERBOSE"
      End

      It "documents RB_VERY_VERBOSE environment variable"
        When run rb help
        The status should equal 0
        The output should include "[env: RB_VERY_VERBOSE"
      End

      It "documents RB_LOG_LEVEL environment variable"
        When run rb help
        The status should equal 0
        The output should include "[env: RB_LOG_LEVEL"
      End

      It "documents RB_CONFIG environment variable"
        When run rb help
        The status should equal 0
        The output should include "[env: RB_CONFIG"
      End

      It "documents RB_PROJECT environment variable"
        When run rb help
        The status should equal 0
        The output should include "[env: RB_PROJECT"
      End

      It "documents RB_RUBIES_DIR environment variable"
        When run rb help
        The status should equal 0
        The output should include "[env: RB_RUBIES_DIR"
      End

      It "documents RB_RUBY_VERSION environment variable"
        When run rb help
        The status should equal 0
        The output should include "[env: RB_RUBY_VERSION"
      End

      It "documents RB_GEM_HOME environment variable"
        When run rb help
        The status should equal 0
        The output should include "[env: RB_GEM_HOME"
      End

      It "documents RB_NO_BUNDLER environment variable"
        When run rb help
        The status should equal 0
        The output should include "[env: RB_NO_BUNDLER"
      End

      It "documents RB_WORK_DIR environment variable"
        When run rb help
        The status should equal 0
        The output should include "[env: RB_WORK_DIR"
      End
    End
  End
End

