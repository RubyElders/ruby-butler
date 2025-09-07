#!/bin/bash
# ShellSpec helper configuration for Ruby Butler
# Distinguished setup for sophisticated testing

# Load shared helper functions
. "$SHELLSPEC_PROJECT_ROOT/spec/support/helpers.sh"

# Configure environment for distinguished testing
export RUBIES_DIR="/opt/rubies"
export GEM_HOME="/home/testuser/.gem"

# Ensure ruby butler binary is accessible
export PATH="/app:$PATH"

# Global setup for systematic test isolation
spec_helper_configure() {
    # Ensure all tests start with clean environment
    setup_test_project
}

# Global cleanup for systematic test isolation
spec_helper_cleanup() {
    # Ensure all tests end with clean environment
    cleanup_test_project
}

# Note: Individual specs handle cleanup with BeforeEach/AfterEach
# No global AfterRun needed with the new isolated approach
