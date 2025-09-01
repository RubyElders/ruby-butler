#!/bin/bash
# Quick validation script for Docker integration test setup

set -e

echo "🔍 Validating Docker integration test setup..."

# Check if required files exist
echo "✅ Checking required files..."
test -f Dockerfile && echo "  ✓ Dockerfile exists"
test -f docker-compose.yml && echo "  ✓ docker-compose.yml exists"
test -f Makefile && echo "  ✓ Makefile exists"
test -f tests/integration/basic_runtime.bats && echo "  ✓ basic_runtime.bats exists"
test -f tests/integration/bundler_integration.bats && echo "  ✓ bundler_integration.bats exists"

# Check if release binary exists
if [ -f target/release/rb ]; then
    echo "  ✓ Release binary exists"
    echo "    Binary size: $(du -h target/release/rb | cut -f1)"
else
    echo "  ❌ Release binary missing - run 'make build' first"
    exit 1
fi

# Validate BATS syntax (basic check)
echo "✅ Validating BATS test syntax..."
for test_file in tests/integration/*.bats; do
    if grep -q "#!/usr/bin/env bats" "$test_file" && \
       grep -q "@test" "$test_file"; then
        echo "  ✓ $(basename "$test_file") has valid BATS syntax"
    else
        echo "  ❌ $(basename "$test_file") has invalid BATS syntax"
        exit 1
    fi
done

# Check Docker availability
echo "✅ Checking Docker environment..."
docker --version > /dev/null && echo "  ✓ Docker is available"
docker compose version > /dev/null && echo "  ✓ docker compose is available"

echo ""
echo "🎉 Setup validation complete! Ready to run integration tests."
echo ""
echo "Next steps:"
echo "  make test-integration    # Run all integration tests"
echo "  make debug              # Debug with interactive container"
echo "  make test-file FILE=... # Run specific test file"
