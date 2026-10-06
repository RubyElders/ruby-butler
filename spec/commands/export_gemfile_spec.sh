#!/bin/bash

Describe "Ruby Butler Gemfile export"
  Include spec/support/helpers.sh

  setup() {
    RUBIES_DIR="${RB_EXPORT_TEST_RUBIES_DIR:-$RUBIES_DIR}"
    TEST_EXPORT_DIR="${SHELLSPEC_TMPBASE}/export-gemfile-$$-${RANDOM}"
    mkdir -p "$TEST_EXPORT_DIR/tea-room"
    cp spec/fixtures/export-gemfile/Gemfile "$TEST_EXPORT_DIR/tea-room/Gemfile"
    printf '[project]\nname = "Dinner service"\n' > "$TEST_EXPORT_DIR/tea-room/settings.toml"
  }

  cleanup() {
    rm -rf "$TEST_EXPORT_DIR"
  }

  BeforeEach 'setup'
  AfterEach 'cleanup'

  It "exports compact TOML to stdout without creating files"
    When run rb -R "$RUBIES_DIR" -r "$LATEST_RUBY" -C "$TEST_EXPORT_DIR/tea-room" export-gemfile
    The status should equal 0
    The stderr should equal ""
    The output should include 'name = "tea-room"'
    The output should include 'rails = "~> 8.0"'
    The output should include 'debug = { require = false }'
    The output should include 'url = "https://gems.example.org"'
    The output should include 'private-api = "~> 2.0"'
    The file "$TEST_EXPORT_DIR/tea-room/rbproject.toml" should not be exist
    The file "$TEST_EXPORT_DIR/tea-room/Gemfile.lock" should not be exist
  End

  It "exports KDL using global project settings"
    When run rb -R "$RUBIES_DIR" -r "$LATEST_RUBY" -C "$TEST_EXPORT_DIR/tea-room" -P settings.toml export-gemfile --format kdl
    The status should equal 0
    The stderr should equal ""
    The output should include 'name "Dinner service"'
    The output should include 'gem rails "~> 8.0"'
    The output should include 'gem debug require=#false'
    The output should include 'source "https://gems.example.org"'
    The contents of file "$TEST_EXPORT_DIR/tea-room/settings.toml" should include 'name = "Dinner service"'
  End

  It "fails when the selected Gemfile is missing"
    When run rb -R "$RUBIES_DIR" -r "$LATEST_RUBY" -C "$TEST_EXPORT_DIR/tea-room" export-gemfile missing.rb
    The status should not equal 0
    The output should equal ""
    The stderr should include "missing.rb"
  End
End
