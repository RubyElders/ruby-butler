#!/bin/bash
# ShellSpec tests for Ruby Butler run command
# Distinguished validation of rb run / rb r script execution

Describe "Ruby Butler Run Command"
  Include spec/support/helpers.sh

  setup() {
    TEST_RUN_DIR="${SHELLSPEC_TMPBASE}/run-test-$$-${RANDOM}"
    mkdir -p "$TEST_RUN_DIR"
  }

  cleanup() {
    if [ -d "$TEST_RUN_DIR" ]; then
      rm -rf "$TEST_RUN_DIR"
    fi
  }

  BeforeEach 'setup'
  AfterEach 'cleanup'

  Describe "rb run command"
    Context "listing available scripts"
      It "lists all scripts when no script name provided"
        cd "$TEST_RUN_DIR"
        cat > rbproject.toml << 'EOF'
[project]
name = "Test Project"

[scripts]
test = "echo test"
build = "echo build"
deploy = "echo deploy"
EOF
        When run rb -R "$RUBIES_DIR" run
        The status should equal 0
        The output should include "test"
        The output should include "build"
        The output should include "deploy"
      End

      It "shows project name in script list"
        cd "$TEST_RUN_DIR"
        cat > rbproject.toml << 'EOF'
[project]
name = "Sophisticated Project"

[scripts]
version = "ruby -v"
EOF
        When run rb -R "$RUBIES_DIR" run
        The status should equal 0
        The output should include "Sophisticated Project"
      End

      It "displays script descriptions when available"
        cd "$TEST_RUN_DIR"
        cat > rbproject.toml << 'EOF'
[scripts]
test = { command = "echo test", description = "Run test suite" }
build = "echo build"
EOF
        When run rb -R "$RUBIES_DIR" run
        The status should equal 0
        The output should include "Run test suite"
      End

      It "handles scripts with colon notation"
        cd "$TEST_RUN_DIR"
        cat > rbproject.toml << 'EOF'
[scripts]
"test:unit" = "echo unit"
"test:integration" = "echo integration"
EOF
        When run rb -R "$RUBIES_DIR" run
        The status should equal 0
        The output should include "test:unit"
        The output should include "test:integration"
      End
    End

    Context "executing scripts"
      It "executes simple script command"
        Skip if "Ruby not available" is_ruby_available
        cd "$TEST_RUN_DIR"
        cat > rbproject.toml << 'EOF'
[scripts]
version = "ruby -v"
EOF
        When run rb -R "$RUBIES_DIR" run version
        The status should equal 0
        The output should include "ruby"
      End

      It "executes script with detailed notation"
        Skip if "Ruby not available" is_ruby_available
        cd "$TEST_RUN_DIR"
        cat > rbproject.toml << 'EOF'
[scripts]
info = { command = "ruby -v", description = "Show version" }
EOF
        When run rb -R "$RUBIES_DIR" run info
        The status should equal 0
        The output should include "ruby"
      End

      It "passes additional arguments to script"
        Skip if "Ruby not available" is_ruby_available
        cd "$TEST_RUN_DIR"
        cat > rbproject.toml << 'EOF'
[scripts]
echo-args = "ruby -e 'puts ARGV.join(\", \")'"
EOF
        When run rb -R "$RUBIES_DIR" run echo-args arg1 arg2 arg3
        The status should equal 0
        The output should include "arg1"
        The output should include "arg2"
      End

      It "handles scripts with colons in execution"
        cd "$TEST_RUN_DIR"
        cat > rbproject.toml << 'EOF'
[scripts]
"test:version" = "echo 'test version script'"
EOF
        When run rb -R "$RUBIES_DIR" run test:version
        The status should equal 0
        The output should include "test version"
      End
    End

    Context "rb r alias"
      It "responds to 'r' alias for run"
        cd "$TEST_RUN_DIR"
        cat > rbproject.toml << 'EOF'
[scripts]
test = "echo test"
EOF
        When run rb -R "$RUBIES_DIR" r
        The status should equal 0
        The output should include "test"
      End

      It "executes scripts via 'r' alias"
        cd "$TEST_RUN_DIR"
        cat > rbproject.toml << 'EOF'
[scripts]
hello = "echo 'Hello from rb r'"
EOF
        When run rb -R "$RUBIES_DIR" r hello
        The status should equal 0
        The output should include "Hello from rb r"
      End
    End

    Context "error handling"
      It "reports when script name not found"
        cd "$TEST_RUN_DIR"
        cat > rbproject.toml << 'EOF'
[scripts]
test = "echo test"
EOF
        When run rb -R "$RUBIES_DIR" run nonexistent
        The status should not equal 0
        The stderr should include "not defined in your project configuration"
        The stderr should include "nonexistent"
      End

      It "provides helpful suggestions for similar script names"
        cd "$TEST_RUN_DIR"
        cat > rbproject.toml << 'EOF'
[scripts]
test = "echo test"
tests = "echo tests"
EOF
        When run rb -R "$RUBIES_DIR" run tset
        The status should not equal 0
        The stderr should include "not defined in your project configuration"
      End

      It "handles empty scripts section gracefully"
        cd "$TEST_RUN_DIR"
        cat > rbproject.toml << 'EOF'
[project]
name = "Empty Scripts"

[scripts]
EOF
        When run rb -R "$RUBIES_DIR" run
        The status should equal 0
        The output should include "No scripts"
      End

      It "reports missing rbproject.toml clearly"
        cd "$TEST_RUN_DIR"
        When run rb -R "$RUBIES_DIR" run test
        The status should not equal 0
        The stderr should include "No project configuration"
        The stderr should include "rbproject.toml"
      End
    End

    Context "with Ruby environment"
      It "executes gem commands within Ruby environment"
        Skip if "Ruby not available" is_ruby_available
        cd "$TEST_RUN_DIR"
        cat > rbproject.toml << 'EOF'
[scripts]
gem-version = "gem -v"
EOF
        When run rb -R "$RUBIES_DIR" run gem-version
        The status should equal 0
        The output should include "."
      End

      It "provides access to bundler if available"
        Skip if "Ruby not available" is_ruby_available
        Skip if "Bundler not available" is_bundler_available
        cd "$TEST_RUN_DIR"
        cat > rbproject.toml << 'EOF'
[scripts]
bundle-version = "bundle -v"
EOF
        When run rb -R "$RUBIES_DIR" run bundle-version
        The status should equal 0
        The output should include "Bundler"
      End
    End

    Context "environment variable support"
      It "respects RB_RUBIES_DIR environment variable"
        cd "$TEST_RUN_DIR"
        cat > rbproject.toml << 'EOF'
[scripts]
test = "echo test"
EOF
        export RB_RUBIES_DIR="$RUBIES_DIR"
        When run rb run
        The status should equal 0
        The output should include "test"
      End

      It "respects RB_RUBY_VERSION environment variable"
        Skip if "Ruby not available" is_ruby_available
        cd "$TEST_RUN_DIR"
        cat > rbproject.toml << 'EOF'
[scripts]
version = "ruby -v"
EOF
        export RB_RUBY_VERSION="$OLDER_RUBY"
        When run rb -R "$RUBIES_DIR" run version
        The status should equal 0
        The output should include "$OLDER_RUBY"
      End

      It "respects RB_PROJECT environment variable"
        cd "$TEST_RUN_DIR"
        cat > custom-project.toml << 'EOF'
[scripts]
custom = "echo custom"
EOF
        export RB_PROJECT="${TEST_RUN_DIR}/custom-project.toml"
        When run rb -R "$RUBIES_DIR" run
        The status should equal 0
        The output should include "custom"
      End

      It "allows CLI flags to override environment variables"
        cd "$TEST_RUN_DIR"
        cat > env-project.toml << 'EOF'
[scripts]
env-script = "echo env"
EOF
        cat > cli-project.toml << 'EOF'
[scripts]
cli-script = "echo cli"
EOF
        export RB_PROJECT="${TEST_RUN_DIR}/env-project.toml"
        When run rb -R "$RUBIES_DIR" -P cli-project.toml run
        The status should equal 0
        The output should include "cli-script"
      End
    End
  End
End
