#!/bin/bash
# ShellSpec tests for Ruby Butler exec command - Bundler environment testing
# Distinguished validation of Bundler execution capabilities

Describe "Ruby Butler Exec Command - Bundler Environment"
  Include spec/support/helpers.sh

  Describe "exec command with Bundler environment"
    Context "bundler project execution"
      BeforeEach 'setup_test_project'
      BeforeEach 'create_bundler_project .'
      AfterEach 'cleanup_test_project'

      It "executes bundle env with appropriate ceremony"
        When run rb -R "$RUBIES_DIR" exec bundle env
        The status should equal 0
        The lines of stderr should be valid number
        The output should include "## Environment"
        The output should include "Bundler"
        The output should include "Ruby"
        The output should include "RubyGems"
        The output should include "Gem Home"
        The output should include "Gem Path"
        # Allow stderr from bundler deprecation warnings
      End

      It "shows correct Ruby version in bundle env"
        When run rb -R "$RUBIES_DIR" -r "$LATEST_RUBY" exec bundle env
        The status should equal 0
        The lines of stderr should be valid number
        The output should include "Ruby          $LATEST_RUBY"
        The output should include "Full Path   /opt/rubies/ruby-$LATEST_RUBY/bin/ruby"
        # Allow stderr from bundler deprecation warnings
      End

      It "shows correct Ruby version with older version"
        When run rb -R "$RUBIES_DIR" -r "$OLDER_RUBY" exec bundle env
        The status should equal 0
        The lines of stderr should be valid number
        The output should include "Ruby          $OLDER_RUBY"
        The output should include "Full Path   /opt/rubies/ruby-$OLDER_RUBY/bin/ruby"
        # Allow stderr from bundler deprecation warnings
      End
    End

    Context "bundler project with ruby version selection (-r, --ruby)"
      BeforeEach 'setup_test_project'
      BeforeEach 'create_bundler_project .'
      AfterEach 'cleanup_test_project'

      It "respects specific Ruby version with -r flag in bundler"
        When run rb -R "$RUBIES_DIR" -r "$OLDER_RUBY" exec bundle env
        The status should equal 0
        The lines of stderr should be valid number
        The output should include "Ruby          $OLDER_RUBY"
        The output should include "/opt/rubies/ruby-$OLDER_RUBY/bin/ruby"
        # Allow stderr from bundler deprecation warnings
      End

      It "respects specific Ruby version with --ruby flag in bundler"
        When run rb -R "$RUBIES_DIR" --ruby "$LATEST_RUBY" exec bundle env
        The status should equal 0
        The lines of stderr should be valid number
        The output should include "Ruby          $LATEST_RUBY"
        The output should include "/opt/rubies/ruby-$LATEST_RUBY/bin/ruby"
        # Allow stderr from bundler deprecation warnings
      End

      It "works with latest Ruby version variable in bundler"
        When run rb -R "$RUBIES_DIR" -r "$LATEST_RUBY" exec bundle env
        The status should equal 0
        The lines of stderr should be valid number
        The output should include "Ruby          $LATEST_RUBY"
        # Allow stderr from bundler deprecation warnings
      End

      It "works with older Ruby version variable in bundler"
        When run rb -R "$RUBIES_DIR" -r "$OLDER_RUBY" exec bundle env
        The status should equal 0
        The lines of stderr should be valid number
        The output should include "Ruby          $OLDER_RUBY"
        # Note: No stderr expectation to avoid network timeout issues
        # Allow stderr from bundler deprecation warnings
      End
    End

    Context "bundler project with rubies directory specification (-R, --rubies-dir)"
      BeforeEach 'setup_test_project'
      BeforeEach 'create_bundler_project .'
      AfterEach 'cleanup_test_project'

      It "respects custom rubies directory with -R flag in bundler"
        When run rb -R "$RUBIES_DIR" exec bundle env
        The status should equal 0
        The lines of stderr should be valid number
        The output should include "Full Path   /opt/rubies"
        # Allow stderr from bundler deprecation warnings
      End

      It "respects custom rubies directory with --rubies-dir flag in bundler"
        When run rb --rubies-dir "$RUBIES_DIR" exec bundle env
        The status should equal 0
        The lines of stderr should be valid number
        The output should include "Full Path   /opt/rubies"
        # Allow stderr from bundler deprecation warnings
      End

      It "combines rubies directory with specific Ruby version in bundler"
        When run rb -R "$RUBIES_DIR" -r "$LATEST_RUBY" exec bundle env
        The status should equal 0
        The lines of stderr should be valid number
        The output should include "Ruby          $LATEST_RUBY"
        The output should include "Full Path   /opt/rubies/ruby-$LATEST_RUBY/bin/ruby"
        # Allow stderr from bundler deprecation warnings
      End
    End

    Context "bundler project with gem home specification (-G, --gem-home)"
      BeforeEach 'setup_test_project'
      BeforeEach 'create_bundler_project .'
      AfterEach 'cleanup_test_project'

      It "respects custom gem home with -G flag in bundler"
        When run rb -R "$RUBIES_DIR" -G "/tmp/bundler-gems" exec bundle env
        The status should equal 0
        The lines of stderr should be valid number
        The output should include "Gem Home    /tmp/bundler-gems"
        The output should include "Gem Path    /tmp/bundler-gems"
        # Allow stderr from bundler deprecation warnings
      End

      It "respects custom gem home with --gem-home flag in bundler"
        When run rb -R "$RUBIES_DIR" --gem-home "/tmp/bundler-custom" exec bundle env
        The status should equal 0
        The lines of stderr should be valid number
        The output should include "Gem Home    /tmp/bundler-custom"
        # Allow stderr from bundler deprecation warnings
      End

      It "combines gem home with specific Ruby version in bundler"
        When run rb -R "$RUBIES_DIR" -r "$LATEST_RUBY" -G "/tmp/bundler-version" exec bundle env
        The status should equal 0
        The lines of stderr should be valid number
        The output should include "Ruby          $LATEST_RUBY"
        The output should include "Gem Home    /tmp/bundler-version"
        # Allow stderr from bundler deprecation warnings
      End

      It "shows correct bin directory with custom gem home in bundler"
        When run rb -R "$RUBIES_DIR" -G "/tmp/bundler-bin" exec bundle env
        The status should equal 0
        The lines of stderr should be valid number
        The output should include "Bin Dir     /tmp/bundler-bin"
        # Allow stderr from bundler deprecation warnings
      End
    End

    Context "bundler project parameter combinations"
      BeforeEach 'setup_test_project'
      BeforeEach 'create_bundler_project .'
      AfterEach 'cleanup_test_project'

      It "handles all parameters together in bundler"
        When run rb -R "$RUBIES_DIR" -r "$OLDER_RUBY" -G "/tmp/bundler-all" exec bundle env
        The status should equal 0
        The lines of stderr should be valid number
        The output should include "Ruby          $OLDER_RUBY"
        The output should include "Full Path   /opt/rubies/ruby-$OLDER_RUBY/bin/ruby"
        The output should include "Gem Home    /tmp/bundler-all"
        # Allow stderr from bundler deprecation warnings
      End

      It "handles long-form parameters together in bundler"
        When run rb --rubies-dir "$RUBIES_DIR" --ruby "$LATEST_RUBY" --gem-home "/tmp/bundler-long" exec bundle env
        The status should equal 0
        The lines of stderr should be valid number
        The output should include "Ruby          $LATEST_RUBY"
        The output should include "Gem Home    /tmp/bundler-long"
        # Allow stderr from bundler deprecation warnings
      End

      It "handles mixed short and long parameters in bundler"
        When run rb --rubies-dir "$RUBIES_DIR" --ruby "$LATEST_RUBY" -G "/tmp/bundler-mixed" exec bundle env
        The status should equal 0
        The lines of stderr should be valid number
        The output should include "Ruby          $LATEST_RUBY"
        The output should include "Gem Home    /tmp/bundler-mixed"
        # Allow stderr from bundler deprecation warnings
      End
    End

    Context "bundler project with .ruby-version detection"
      BeforeEach 'setup_test_project'
      AfterEach 'cleanup_test_project'

      It "respects .ruby-version file in bundler project"
        create_bundler_project "." "$OLDER_RUBY"

        When run rb -R "$RUBIES_DIR" exec bundle env
        The status should equal 0
        The lines of stderr should be valid number
        The output should include "Ruby          $OLDER_RUBY"
        # Allow stderr from bundler deprecation warnings
      End

      It "overrides .ruby-version with -r flag in bundler"
        create_bundler_project "." "$OLDER_RUBY"

        When run rb -R "$RUBIES_DIR" -r "$LATEST_RUBY" exec bundle env
        The status should equal 0
        The lines of stderr should be valid number
        The output should include "Ruby          $LATEST_RUBY"
        # Allow stderr from bundler deprecation warnings
      End
    End

    Context "bundler project with Gemfile ruby directive"
      BeforeEach 'setup_test_project'
      AfterEach 'cleanup_test_project'

      It "respects Gemfile ruby directive in bundler project"
        create_bundler_project "." "" "$LATEST_RUBY"

        When run rb -R "$RUBIES_DIR" exec bundle env
        The status should equal 0
        The lines of stderr should be valid number
        The output should include "Ruby          $LATEST_RUBY"
        # Allow stderr from bundler deprecation warnings
      End

      It "shows correct config directory with Gemfile ruby"
        create_bundler_project "." "" "$LATEST_RUBY"

        When run rb -R "$RUBIES_DIR" exec bundle env
        The status should equal 0
        The lines of stderr should be valid number
        The output should include "Config Dir  /opt/rubies/ruby-$LATEST_RUBY/etc"
        # Allow stderr from bundler deprecation warnings
      End
    End

    Context "bundler commands execution"
      BeforeEach 'setup_test_project'
      BeforeEach 'create_bundler_project .'
      AfterEach 'cleanup_test_project'

      It "executes bundle install successfully"
        When run rb -R "$RUBIES_DIR" exec bundle install
        The status should equal 0
        The lines of stderr should be valid number
        The output should include "Bundle complete"
        # Allow stderr from bundler deprecation warnings
      End

      It "executes bundle install with specific Ruby version"
        When run rb -R "$RUBIES_DIR" -r "$LATEST_RUBY" exec bundle install
        The status should equal 0
        The lines of stderr should be valid number
        The output should include "Bundle complete"
        # Allow stderr from bundler deprecation warnings
      End

      It "executes bundle list after install"
        # First install, then test list in separate test
        When run rb -R "$RUBIES_DIR" exec bundle list
        The status should equal 0
        The lines of stderr should be valid number
        # Bundle list may trigger install, so expect bundler output
        The output should include "Butler Notice"
        # Allow stderr from bundler deprecation warnings
      End

      It "executes bundle exec rake after install"
        # Install first then exec rake
        When run rb -R "$RUBIES_DIR" exec bundle exec rake --version
        The status should equal 0
        The lines of stderr should be valid number
        The output should include "rake"
        # Allow stderr from bundler deprecation warnings
      End
    End

    Context "lockfile update on exec"
      BeforeEach 'setup_test_project'
      AfterEach 'cleanup_test_project'

      It "updates Gemfile.lock when gem is removed before exec"
        # Create Gemfile with TWO gems
        cat > Gemfile << 'EOF'
source 'https://rubygems.org'
gem 'rake'
gem 'minitest'
EOF

        # Initial sync to install both gems
        rb -R "$RUBIES_DIR" sync >/dev/null 2>&1

        # Verify both gems are in Gemfile.lock
        grep -q "rake" Gemfile.lock || fail "rake should be in initial Gemfile.lock"
        grep -q "minitest" Gemfile.lock || fail "minitest should be in initial Gemfile.lock"

        # Remove minitest from Gemfile
        cat > Gemfile << 'EOF'
source 'https://rubygems.org'
gem 'rake'
EOF

        # Execute a ruby command - this should trigger lockfile update via check_sync
        When run rb -R "$RUBIES_DIR" exec ruby -e "puts 'test'"
        The status should equal 0
        The lines of stderr should be valid number
        The output should include "test"

        # Verify lockfile was updated: rake remains, minitest removed
        The path Gemfile.lock should be exist
        The contents of file Gemfile.lock should include "rake"
        The contents of file Gemfile.lock should not include "minitest"
      End
    End

    Context "bundler error handling"
      BeforeEach 'setup_test_project'
      # Empty directory without Gemfile for error testing
      AfterEach 'cleanup_test_project'

      It "handles bundle install gracefully without Gemfile"
        When run rb -R "$RUBIES_DIR" exec bundle install
        The status should not equal 0
        The lines of stdout should be valid number
        The lines of stderr should be valid number
        The stderr should include "Could not locate Gemfile"
        # Allow stderr from bundler deprecation warnings
      End

      It "handles bundle exec gracefully without Gemfile"
        When run rb -R "$RUBIES_DIR" exec bundle exec rake
        The status should not equal 0
        The lines of stderr should be valid number
        The stderr should include "Could not locate Gemfile"
        # Allow stderr from bundler deprecation warnings
      End
    End
  End
End
