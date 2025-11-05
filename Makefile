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

# Format shell scripts (trailing whitespace + final newline only)
fmt-shell:
	@echo "✨ Formatting shell scripts..."
	@find spec -name "*.sh" -type f -exec sed -i 's/[[:space:]]*$$//' {} \;
	@find spec -name "*.sh" -type f -exec sh -c 'tail -c1 "$$1" | read -r _ || echo >> "$$1"' _ {} \;
	@echo "✅ Shell script formatting complete"

# Check shell scripts for common issues
lint-shell:
	@echo "🔍 Linting shell scripts..."
	@shellcheck spec/**/*.sh

# Format YAML files (trailing whitespace + final newline only)
fmt-yaml:
	@echo "✨ Formatting YAML files..."
	@find .github -name "*.yml" -type f -exec sed -i 's/[[:space:]]*$$//' {} \;
	@find .github -name "*.yml" -type f -exec sh -c 'tail -c1 "$$1" | read -r _ || echo >> "$$1"' _ {} \;
	@echo "✅ YAML formatting complete"

# Check YAML files
lint-yaml:
	@echo "🔍 Linting YAML files..."
	@yamllint .github/workflows/
