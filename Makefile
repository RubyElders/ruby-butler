# Ruby Butler Testing

.PHONY: test docker

# Run all tests in parallel (Docker-based)
test-commands:
	@echo "🔨 Building release binary..."
	cargo build --release
	@echo "🐳 Building Docker test image..."
	docker build -t rb-test .
	@echo "🚀 Running all tests in parallel..."
	docker run --rm -v ./target/release/rb:/app/rb:ro -v ./tests:/app/tests:ro rb-test bats --jobs 20 tests/integration/commands


# Run all tests in parallel (Docker-based)
test:
	@echo "🔨 Building release binary..."
	cargo build --release
	@echo "🐳 Building Docker test image..."
	docker build -t rb-test .
	@echo "🚀 Running all tests in parallel..."
	docker run --rm -v ./target/release/rb:/app/rb:ro -v ./tests:/app/tests:ro rb-test bats --jobs 20 tests/integration

# Build Docker image for testing
docker:
	@echo "🐳 Building Docker test image..."
	docker build -t rb-test .
