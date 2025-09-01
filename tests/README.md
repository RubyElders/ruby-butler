# Integration Testing with Docker

This directory contains Docker-based integration tests for Ruby Butler using BATS (Bash Automated Testing System).

## Overview

The integration tests run in clean Docker containers with:
- Multiple Ruby versions (3.2.4, 3.4.5) installed via ruby-install
- Isolated gem environments for parallel testing
- BATS test framework for reliable bash-based testing
- Production-ready binary testing

## Quick Start

```bash
# Run all integration tests
make test-integration

# Run just unit tests
make test-unit

# Run all tests (unit + integration)
make test

# Debug with interactive container
make debug

# Run specific test file
make test-file FILE=tests/integration/basic_runtime.bats
```

## Test Structure

- `tests/integration/basic_runtime.bats` - Core functionality tests
- `tests/integration/bundler_integration.bats` - Bundler workflow tests
- `Dockerfile` - Clean test environment with Ruby versions
- `docker-compose.yml` - Test orchestration
- `Makefile` - Convenient test commands

## Why Docker Integration Tests?

1. **Clean Environment**: Each test runs in a pristine container
2. **Real Ruby Installations**: Tests against actual ruby-install builds
3. **Parallel Isolation**: Custom gem homes prevent test interference
4. **Production Testing**: Tests the actual release binary
5. **Reproducible**: Same environment across development machines

## Test Philosophy

- **Unit tests**: Fast, isolated, mocked dependencies
- **Integration tests**: Real Ruby installations, real gem operations
- **Docker tests**: Full end-to-end workflows in clean environments

## BATS vs TAP

We chose BATS over TAP because:
- Better bash integration and helpers
- Cleaner test syntax and organization  
- Built-in setup/teardown functions
- Better error reporting and debugging
- Standard in the bash testing ecosystem

## Custom Gem Home Testing

The `--gem-home` argument is extensively tested to ensure:
- Parallel test execution without interference
- Container-based gem isolation
- Correct environment variable propagation
- Bundler compatibility with custom gem paths

## Debugging Tests

```bash
# Start debug container
make debug

# Inside container, run tests manually:
bats tests/integration/basic_runtime.bats

# Run with verbose output:
bats -t tests/integration/basic_runtime.bats

# Test specific function:
bats -f "rb binary is executable" tests/integration/basic_runtime.bats
```

## Adding New Tests

1. Create new `.bats` file in `tests/integration/`
2. Use BATS test functions with descriptive names
3. Include setup/teardown for test isolation
4. Test both success and failure scenarios
5. Use custom gem homes for isolation

Example:
```bash
#!/usr/bin/env bats

setup() {
    export TEST_GEM_HOME="$(mktemp -d)"
}

teardown() {
    rm -rf "$TEST_GEM_HOME"
}

@test "new functionality works correctly" {
    run rb --gem-home "$TEST_GEM_HOME" new-command
    [ "$status" -eq 0 ]
    [[ "$output" =~ "expected pattern" ]]
}
```
