#!/bin/bash

Describe "Ruby Butler Ruby installation"
  Include spec/support/helpers.sh

  setup() {
    TEST_INSTALL_DIR="${SHELLSPEC_TMPBASE}/install-ruby-$$-${RANDOM}"
    mkdir -p "$TEST_INSTALL_DIR"
  }

  cleanup() {
    rm -rf "$TEST_INSTALL_DIR"
  }

  BeforeEach 'setup'
  AfterEach 'cleanup'

  It "lists Ruby installation as a utility command"
    When run rb help
    The status should equal 0
    The output should include "Utility Commands:"
    The output should include "install"
    The stderr should equal ""
  End

  It "offers the Ruby installation target without requiring Ruby"
    When run rb -R "$TEST_INSTALL_DIR/missing" help install
    The status should equal 0
    The output should include "ruby"
    The stderr should equal ""
    The directory "$TEST_INSTALL_DIR/missing" should not be exist
  End

  It "respects the global opt-out before exec"
    When run rb --no-auto-install-ruby -R "$TEST_INSTALL_DIR/missing" -r 9.9.9 x ruby -v
    The status should not equal 0
    The stderr should include "rb install ruby 9.9.9"
    The output should equal ""
    The directory "$TEST_INSTALL_DIR/missing" should not be exist
  End

  It "respects the environment opt-out before sync"
    export RB_AUTO_INSTALL_RUBY=false
    When run rb -R "$TEST_INSTALL_DIR/missing" -r 9.9.9 sync
    The status should not equal 0
    The stderr should include "rb install ruby 9.9.9"
    The output should equal ""
    The directory "$TEST_INSTALL_DIR/missing" should not be exist
  End

  It "keeps completion free of installation side effects"
    When run rb -R "$TEST_INSTALL_DIR/missing" __bash_complete "rb install " 11
    The status should equal 0
    The output should include "ruby"
    The stderr should equal ""
    The directory "$TEST_INSTALL_DIR/missing" should not be exist
  End
End
