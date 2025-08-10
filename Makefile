# Makefile for timelog project

.PHONY: all build test test-unit test-integration test-e2e test-all clean format lint check help install

# Default target
all: build test

# Build the project
build:
	@echo "Building timelog..."
	cargo build

# Build release version
build-release:
	@echo "Building timelog (release)..."
	cargo build --release

# Run all tests
test: test-unit test-integration test-e2e

# Run unit tests only
test-unit:
	@echo "Running unit tests..."
	cargo test --lib

# Run integration tests
test-integration:
	@echo "Running integration tests..."
	cargo test --test integration_tests
	cargo test --test cli_tests
	cargo test --test plugin_tests

# Run end-to-end tests
test-e2e: build
	@echo "Running end-to-end tests..."
	./tests/e2e/run_all_tests.sh

# Run comprehensive test suite
test-all:
	@echo "Running comprehensive test suite..."
	./run_tests.sh

# Run tests with verbose output
test-verbose:
	@echo "Running comprehensive test suite (verbose)..."
	./run_tests.sh

# Quick test (skip E2E tests)
test-quick:
	@echo "Running quick tests (unit + integration)..."
	./run_tests.sh --skip-e2e

# Format code
format:
	@echo "Formatting code..."
	cargo fmt --all

# Check formatting
format-check:
	@echo "Checking code formatting..."
	cargo fmt --all -- --check

# Run linter
lint:
	@echo "Running linter..."
	cargo clippy --all-targets --all-features -- -D warnings

# Run checks (format + lint)
check: format-check lint

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	cargo clean

# Install the binary
install: build-release
	@echo "Installing timelog..."
	cargo install --path .

# Install development dependencies
install-dev:
	@echo "Installing development dependencies..."
	cargo install cargo-audit

# Run security audit
audit:
	@echo "Running security audit..."
	cargo audit

# Generate documentation
docs:
	@echo "Generating documentation..."
	cargo doc --open

# Show test coverage (requires cargo-tarpaulin)
coverage:
	@echo "Generating test coverage report..."
	@if ! command -v cargo-tarpaulin &> /dev/null; then \
		echo "Installing cargo-tarpaulin..."; \
		cargo install cargo-tarpaulin; \
	fi
	cargo tarpaulin --out Html --output-dir coverage

# Watch for changes and run tests
watch:
	@echo "Watching for changes..."
	@if ! command -v cargo-watch &> /dev/null; then \
		echo "Installing cargo-watch..."; \
		cargo install cargo-watch; \
	fi
	cargo watch -x test

# Benchmark (if implemented)
bench:
	@echo "Running benchmarks..."
	cargo bench

# Setup development environment
setup-dev: install-dev
	@echo "Setting up development environment..."
	@echo "Installing pre-commit hooks..."
	@if [ -d .git ]; then \
		echo '#!/bin/bash\nmake check' > .git/hooks/pre-commit; \
		chmod +x .git/hooks/pre-commit; \
		echo "Pre-commit hook installed"; \
	fi

# Help target
help:
	@echo "Available targets:"
	@echo "  build          - Build the project"
	@echo "  build-release  - Build release version"
	@echo "  test           - Run all tests"
	@echo "  test-unit      - Run unit tests only"
	@echo "  test-integration - Run integration tests only"
	@echo "  test-e2e       - Run end-to-end tests only"
	@echo "  test-all       - Run comprehensive test suite"
	@echo "  test-verbose   - Run comprehensive test suite with verbose output"
	@echo "  test-quick     - Run quick tests (skip E2E)"
	@echo "  format         - Format code"
	@echo "  format-check   - Check code formatting"
	@echo "  lint           - Run linter"
	@echo "  check          - Run format check + linter"
	@echo "  clean          - Clean build artifacts"
	@echo "  install        - Install the binary"
	@echo "  install-dev    - Install development dependencies"
	@echo "  audit          - Run security audit"
	@echo "  docs           - Generate documentation"
	@echo "  coverage       - Generate test coverage report"
	@echo "  watch          - Watch for changes and run tests"
	@echo "  bench          - Run benchmarks"
	@echo "  setup-dev      - Setup development environment"
	@echo "  help           - Show this help message"
