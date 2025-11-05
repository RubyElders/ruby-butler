#!/bin/bash
# ShellSpec tests for Ruby Butler configuration file support
# Distinguished validation of rb.toml loading and precedence

Describe "Ruby Butler Configuration System"
  Include spec/support/helpers.sh

  setup() {
    TEST_CONFIG_DIR="${SHELLSPEC_TMPBASE}/config-test-$$-${RANDOM}"
    mkdir -p "$TEST_CONFIG_DIR"
  }

  cleanup() {
    if [ -d "$TEST_CONFIG_DIR" ]; then
      rm -rf "$TEST_CONFIG_DIR"
    fi
    unset RB_CONFIG
  }

  BeforeEach 'setup'
  AfterEach 'cleanup'

  Describe "--config flag"
    Context "when loading custom configuration file"
      It "accepts --config flag with valid TOML file"
        cd "$TEST_CONFIG_DIR"
        cat > test-config.toml << 'EOF'
ruby-version = "3.2.0"
rubies-dir = "/custom/rubies"
EOF
        When run rb --config test-config.toml --version
        The status should equal 0
        The output should include "rb"
      End

      It "applies rubies-dir from config file"
        cd "$TEST_CONFIG_DIR"
        cat > test-config.toml << 'EOF'
rubies-dir = "/nonexistent/custom/rubies"
EOF
        When run rb --config test-config.toml runtime
        The status should not equal 0
        The stderr should include "/nonexistent/custom/rubies"
      End

      It "shows --config option in help"
        When run rb --help
        The status should equal 0
        The output should include "--config"
        The output should include "configuration file"
      End
    End

    Context "with short form -c flag"
      It "accepts -c flag as alias for --config"
        cd "$TEST_CONFIG_DIR"
        cat > test-config.toml << 'EOF'
ruby-version = "3.2.0"
EOF
        When run rb -c test-config.toml --version
        The status should equal 0
        The output should include "rb"
      End
    End
  End

  Describe "RB_CONFIG environment variable"
    Context "when RB_CONFIG is set"
      It "loads configuration from RB_CONFIG path"
        cd "$TEST_CONFIG_DIR"
        cat > rb-env-config.toml << 'EOF'
rubies-dir = "/env/var/rubies"
EOF
        export RB_CONFIG="${TEST_CONFIG_DIR}/rb-env-config.toml"
        When run rb runtime
        The status should not equal 0
        The stderr should include "/env/var/rubies"
      End

      It "shows config loading with verbose flag"
        cd "$TEST_CONFIG_DIR"
        cat > rb-env-config.toml << 'EOF'
rubies-dir = "/env/var/rubies"
EOF
        export RB_CONFIG="${TEST_CONFIG_DIR}/rb-env-config.toml"
        When run rb -R "$RUBIES_DIR" -v runtime
        The status should equal 0
        The output should include "Ruby Environment Survey"
        The stderr should include "Loading configuration"
      End
    End
  End

  Describe "Configuration precedence"
    Context "when both --config and RB_CONFIG are set"
      It "prefers --config flag over RB_CONFIG"
        cd "$TEST_CONFIG_DIR"
        cat > cli-config.toml << 'EOF'
rubies-dir = "/cli/rubies"
EOF
        cat > env-config.toml << 'EOF'
rubies-dir = "/env/rubies"
EOF
        export RB_CONFIG="${TEST_CONFIG_DIR}/env-config.toml"
        When run rb --config cli-config.toml runtime
        The status should not equal 0
        The stderr should include "/cli/rubies"
      End
    End

    Context "CLI flags override config file values"
      It "uses -R flag over config file rubies-dir"
        cd "$TEST_CONFIG_DIR"
        cat > config.toml << 'EOF'
rubies-dir = "/config/rubies"
EOF
        When run rb --config config.toml -R "/override/rubies" runtime
        The status should not equal 0
        The stderr should include "/override/rubies"
      End
    End
  End
End
