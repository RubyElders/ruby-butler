#!/bin/bash
# ShellSpec tests for Ruby Butler init command
# Distinguished validation of project initialization capabilities

Describe "Ruby Butler Init Command"
  Include spec/support/helpers.sh

  # Use a temporary directory for each test
  setup() {
    TEST_INIT_DIR="${SHELLSPEC_TMPBASE}/init-test-$$-${RANDOM}"
    mkdir -p "$TEST_INIT_DIR"
  }

  cleanup() {
    if [ -d "$TEST_INIT_DIR" ]; then
      rm -rf "$TEST_INIT_DIR"
    fi
  }

  BeforeEach 'setup'
  AfterEach 'cleanup'

  Describe "rb init command"
    Context "when creating a new rbproject.toml"
      It "creates rbproject.toml in current directory"
        cd "$TEST_INIT_DIR"
        When run rb init
        The status should equal 0
        The output should include "Splendid"
        The file "rbproject.toml" should be exist
      End

      It "displays success message with ceremony"
        cd "$TEST_INIT_DIR"
        When run rb init
        The status should equal 0
        The output should include "Splendid!"
        The output should include "rbproject.toml has been created"
      End

      It "creates valid TOML file"
        cd "$TEST_INIT_DIR"
        When run rb init
        The status should equal 0
        The output should include "Splendid"
        The contents of file "rbproject.toml" should include "[project]"
        The contents of file "rbproject.toml" should include "[scripts]"
      End

      It "includes project metadata section"
        cd "$TEST_INIT_DIR"
        When run rb init
        The status should equal 0
        The output should include "Splendid"
        The contents of file "rbproject.toml" should include 'name = "Butler project template"'
        The contents of file "rbproject.toml" should include 'description = "Please fill in"'
      End

      It "includes sample ruby-version script"
        cd "$TEST_INIT_DIR"
        When run rb init
        The status should equal 0
        The output should include "Splendid"
        The contents of file "rbproject.toml" should include 'ruby-version = "ruby -v"'
      End

      It "provides helpful next steps"
        cd "$TEST_INIT_DIR"
        When run rb init
        The status should equal 0
        The output should include "You may now"
        The output should include "rb run"
      End

      It "references example documentation"
        cd "$TEST_INIT_DIR"
        When run rb init
        The status should equal 0
        The output should include "examples/rbproject.toml"
      End
    End

    Context "when rbproject.toml already exists"
      It "gracefully refuses to overwrite existing file"
        cd "$TEST_INIT_DIR"
        echo "existing content" > rbproject.toml
        When run rb init
        The status should not equal 0
        The stderr should include "already graces"
        The stderr should include "this directory"
      End

      It "provides proper guidance for resolution"
        cd "$TEST_INIT_DIR"
        echo "existing content" > rbproject.toml
        When run rb init
        The status should not equal 0
        The stderr should include "delete the existing one first"
      End

      It "preserves existing file content"
        cd "$TEST_INIT_DIR"
        echo "my precious content" > rbproject.toml
        When run rb init
        The status should not equal 0
        The stderr should include "already graces"
        The contents of file "rbproject.toml" should equal "my precious content"
      End
    End

    Context "working with generated rbproject.toml"
      It "can list scripts from generated file"
        cd "$TEST_INIT_DIR"
        rb init >/dev/null 2>&1
        When run rb -R "$RUBIES_DIR" run
        The status should equal 0
        The output should include "ruby-version"
      End

      It "can execute generated script"
        Skip if "Ruby not available" is_ruby_available
        cd "$TEST_INIT_DIR"
        rb init >/dev/null 2>&1
        When run rb -R "$RUBIES_DIR" run ruby-version
        The status should equal 0
        The output should include "ruby"
      End
    End

    Context "environment variable support"
      It "respects RB_RUBIES_DIR environment variable"
        cd "$TEST_INIT_DIR"
        export RB_RUBIES_DIR="$RUBIES_DIR"
        When run rb init
        The status should equal 0
        The output should include "Splendid"
      End

      It "works with RB_WORK_DIR to init in different directory"
        export RB_WORK_DIR="$TEST_INIT_DIR"
        When run rb init
        The status should equal 0
        The output should include "Splendid"
        The file "$TEST_INIT_DIR/rbproject.toml" should be exist
      End
    End
  End
End
