# Docker Integration Testing Infrastructure

This directory contains Docker-based integration testing infrastructure for Ruby Butler. It provides clean, isolated environments for testing the production binary against real Ruby installations.

## Quick Start

```bash
# Build and run all integration tests
make test-integration

# Or use Docker Compose directly
docker compose run --rm integration-tests

# Run a specific test file
make test-file FILE=tests/integration/basic_runtime.bats

# Debug in container
make debug
```

## What We've Built

### 1. Optimized Dockerfile

- **Multi-Ruby Environment**: Clean Debian container with Ruby 3.2.4 (LTS) and 3.4.5 (latest)
- **Parallel Compilation**: Uses all CPU cores (`-j$(nproc)`) for fast Ruby builds
- **Performance Optimizations**: 
  - jemalloc for better memory management
  - `--disable-install-doc` for faster installation
  - `--no-document` for gem installs
- **BATS Testing**: Bash Automated Testing System for integration tests

### 2. BATS Test Suites

#### `tests/integration/basic_runtime.bats`
- Core rb command functionality
- Ruby version detection
- Custom gem home isolation
- Environment composition

#### `tests/integration/bundler_integration.bats` 
- Full bundler workflow testing
- Slow operations that were marked `#[ignore]` in unit tests
- Parallel gem home isolation
- Real bundle install operations

### 3. Docker Compose Services

- **integration-tests**: Main test runner
- **test-single**: Run individual test files
- **debug**: Interactive debugging container

### 4. Makefile Automation

- `make test-integration`: Full Docker test suite
- `make debug`: Interactive container debugging
- `make test-file FILE=path/to/test.bats`: Run specific test
- `make clean`: Clean up Docker resources

## Why This Approach?

### Performance Benefits
- **Isolated Testing**: No interference with host Ruby/gem installations
- **Parallel Execution**: Multiple gem homes allow parallel test runs
- **Real Environment**: Tests actual production binary, not development setup
- **Fast Rebuilds**: Docker layer caching for Ruby installations

### Developer Experience
- **Clean Environment**: Fresh container for each test run
- **No Host Dependencies**: Works regardless of host Ruby setup
- **Reproducible**: Same environment on any machine with Docker
- **Debugging**: Easy to drop into container for troubleshooting

## Test Structure

### Test Isolation
Each BATS test gets:
- Temporary directory (`$BATS_TEST_TMPDIR`)
- Isolated gem home
- Clean environment variables
- Automatic cleanup

### Example Test
```bash
@test "rb with custom gem home works correctly" {
    run rb --gem-home /tmp/test-gems exec -- ruby -e 'puts ENV["GEM_HOME"]'
    [ "$status" -eq 0 ]
    [[ "$output" =~ "/tmp/test-gems/ruby" ]]
}
```

## Integration with Unit Tests

This complements the existing fast unit test suite:

- **Unit Tests** (`cargo test`): Fast, sandboxed, development-focused  
- **Integration Tests** (Docker + BATS): Slow, real-world, production-focused

### Moved from Unit Tests
- `test_sync_command_with_gemfile` (marked `#[ignore]`) → `bundler_integration.bats`
- Real bundler operations requiring gem installations
- Multi-Ruby version compatibility testing

## Architecture Notes

### Custom Gem Home Support
The `--gem-home` CLI argument enables:
- Container-specific gem isolation
- Parallel test execution
- Production-like bundler workflows
- No host environment contamination

### Ruby Installation Pattern
- Ruby 3.2.4: LTS version for stability testing
- Ruby 3.4.5: Latest stable for feature testing
- Standard `/opt/rubies/ruby-X.Y.Z` layout (compatible with ruby-install)
- Bundler pre-installed in each Ruby

## Future Enhancements

- Add more Ruby versions (3.1.x, 3.3.x)
- Test matrix for different OS versions
- Performance benchmarking
- CI/CD integration
- Cross-platform testing (Alpine, Ubuntu)

## Troubleshooting

### Common Issues
1. **Build timeout**: Ruby compilation is CPU-intensive, increase timeout if needed
2. **Gem install failures**: Check network connectivity in container
3. **Permission errors**: Ensure user permissions in container

### Debugging Commands
```bash
# See what's running
docker compose ps

# View logs
docker compose logs integration-tests

# Interactive debugging
make debug

# Clean slate
make clean && make test-integration
```
