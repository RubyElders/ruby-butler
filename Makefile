# Ruby Butler Integration Testing
# 
# This Makefile provides convenient commands for running integration tests
# in Docker containers with clean Ruby environments.

.PHONY: help build test test-unit test-integration test-docker clean debug

# Default target
help:
	@echo "Ruby Butler Testing Commands:"
	@echo ""
	@echo "  make build              - Build the rb binary in release mode"
	@echo "  make test               - Run all tests (unit + integration)"
	@echo "  make test-unit          - Run unit tests only"
	@echo "  make test-integration   - Run Docker-based integration tests (parallel)"
	@echo "  make test-docker        - Alias for test-integration"
	@echo "  make test-sequential    - Run integration tests sequentially (for debugging)"
	@echo "  make debug              - Start debug container with bash shell"
	@echo "  make clean              - Clean up Docker images and containers"
	@echo ""

# Build the release binary for Docker testing
build:
	@echo "🔨 Building rb binary in release mode..."
	cargo build --release

# Run all tests
test: test-unit test-integration

# Run unit tests (fast, no Docker)
test-unit:
	@echo "🧪 Running unit tests..."
	cargo test --lib

# Build Docker image and run integration tests (parallel)
test-integration: build
	@echo "🐳 Running integration tests in parallel..."
	docker compose build integration-tests
	docker compose run --rm integration-tests

# Run integration tests sequentially (for debugging)
test-sequential: build
	@echo "🐳 Running integration tests sequentially..."
	docker compose build test-single
	docker compose run --rm test-single bats tests/integration

# Alias for test-integration  
test-docker: test-integration

# Start a debug container for troubleshooting
debug: build
	@echo "🐳 Starting debug container..."
	docker compose build debug
	docker compose run --rm debug

# Clean up Docker resources
clean:
	@echo "🧹 Cleaning up Docker resources..."
	docker compose down --rmi all --volumes --remove-orphans
	docker system prune -f

# Run a specific test file
test-file: build
	@if [ -z "$(FILE)" ]; then \
		echo "Usage: make test-file FILE=tests/integration/basic_runtime.bats"; \
		exit 1; \
	fi
	@echo "🐳 Running specific test file: $(FILE)"
	docker compose build test-single
	docker compose run --rm test-single bats $(FILE)

# Show Docker logs for debugging
logs:
	docker compose logs integration-tests
