# Ruby Butler Testing
# Distinguished testing orchestration

.PHONY: spec cargo docker

# Build release binary
cargo:
	@echo "🔨 Building release binary..."
	cargo build --release

# Build Docker image for testing
docker:
	@echo "🐳 Building Docker test image..."
	docker compose build

# Run ShellSpec tests using Docker Compose
spec: cargo docker
	@echo "🚀 Running ShellSpec tests with distinguished parallel execution..."
	./shellspec
