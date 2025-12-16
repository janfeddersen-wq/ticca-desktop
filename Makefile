# Makefile for Ticca Desktop
#
# Common development tasks for the Ticca Desktop application.
# This provides convenient shortcuts for building, testing, and
# managing the project.
#
# Usage:
#   make build      - Build debug binaries
#   make release    - Build release binaries
#   make test       - Run all tests
#   make lint       - Run all linters
#   make check      - Run lint and test
#   make clean      - Clean build artifacts
#   make run        - Run the application (debug)
#   make dev        - Run with hot reloading
#   make help       - Show this help message

.PHONY: build release test lint fmt check clean run dev help
.PHONY: rust-build rust-release rust-test rust-lint rust-fmt rust-check
.PHONY: python-test python-lint python-fmt python-check
.PHONY: setup install-deps docs

# Default target
.DEFAULT_GOAL := help

# Colors for output
COLOR_RESET   := \033[0m
COLOR_GREEN   := \033[32m
COLOR_YELLOW  := \033[33m
COLOR_BLUE    := \033[34m

# ============================================================================
# Help
# ============================================================================

help:
	@echo "$(COLOR_BLUE)Ticca Desktop - Development Commands$(COLOR_RESET)"
	@echo ""
	@echo "$(COLOR_GREEN)Build Commands:$(COLOR_RESET)"
	@echo "  make build         Build debug binaries (Rust)"
	@echo "  make release       Build release binaries (Rust)"
	@echo ""
	@echo "$(COLOR_GREEN)Test Commands:$(COLOR_RESET)"
	@echo "  make test          Run all tests (Rust + Python)"
	@echo "  make rust-test     Run Rust tests only"
	@echo "  make python-test   Run Python tests only"
	@echo ""
	@echo "$(COLOR_GREEN)Lint Commands:$(COLOR_RESET)"
	@echo "  make lint          Run all linters (Rust + Python)"
	@echo "  make rust-lint     Run Rust linters (clippy)"
	@echo "  make python-lint   Run Python linters (ruff, mypy)"
	@echo "  make fmt           Format all code"
	@echo ""
	@echo "$(COLOR_GREEN)Quality Commands:$(COLOR_RESET)"
	@echo "  make check         Run lint + test (full quality check)"
	@echo "  make docs          Build documentation"
	@echo ""
	@echo "$(COLOR_GREEN)Run Commands:$(COLOR_RESET)"
	@echo "  make run           Run the application (debug build)"
	@echo "  make dev           Run with cargo-watch (hot reload)"
	@echo ""
	@echo "$(COLOR_GREEN)Setup Commands:$(COLOR_RESET)"
	@echo "  make setup         Initial project setup"
	@echo "  make install-deps  Install development dependencies"
	@echo "  make clean         Clean all build artifacts"

# ============================================================================
# Build Commands
# ============================================================================

build: rust-build
	@echo "$(COLOR_GREEN)✓ Build complete$(COLOR_RESET)"

rust-build:
	@echo "$(COLOR_BLUE)Building Rust workspace (debug)...$(COLOR_RESET)"
	cargo build --workspace

release: rust-release
	@echo "$(COLOR_GREEN)✓ Release build complete$(COLOR_RESET)"

rust-release:
	@echo "$(COLOR_BLUE)Building Rust workspace (release)...$(COLOR_RESET)"
	cargo build --workspace --release

# ============================================================================
# Test Commands
# ============================================================================

test: rust-test python-test
	@echo "$(COLOR_GREEN)✓ All tests passed$(COLOR_RESET)"

rust-test:
	@echo "$(COLOR_BLUE)Running Rust tests...$(COLOR_RESET)"
	cargo test --workspace

python-test:
	@echo "$(COLOR_BLUE)Running Python tests...$(COLOR_RESET)"
	cd python && pytest tests/ -v

# ============================================================================
# Lint Commands
# ============================================================================

lint: rust-lint python-lint
	@echo "$(COLOR_GREEN)✓ All linting passed$(COLOR_RESET)"

rust-lint:
	@echo "$(COLOR_BLUE)Running Rust linters...$(COLOR_RESET)"
	cargo clippy --workspace --all-targets -- -D warnings

python-lint:
	@echo "$(COLOR_BLUE)Running Python linters...$(COLOR_RESET)"
	cd python && ruff check .
	cd python && mypy ticca_agent/ --ignore-missing-imports

# ============================================================================
# Format Commands
# ============================================================================

fmt: rust-fmt python-fmt
	@echo "$(COLOR_GREEN)✓ All formatting complete$(COLOR_RESET)"

rust-fmt:
	@echo "$(COLOR_BLUE)Formatting Rust code...$(COLOR_RESET)"
	cargo fmt --all

python-fmt:
	@echo "$(COLOR_BLUE)Formatting Python code...$(COLOR_RESET)"
	cd python && ruff format .

# ============================================================================
# Quality Check
# ============================================================================

check: lint test
	@echo "$(COLOR_GREEN)✓ All quality checks passed$(COLOR_RESET)"

rust-check: rust-lint rust-test
	@echo "$(COLOR_GREEN)✓ Rust quality checks passed$(COLOR_RESET)"

python-check: python-lint python-test
	@echo "$(COLOR_GREEN)✓ Python quality checks passed$(COLOR_RESET)"

# ============================================================================
# Documentation
# ============================================================================

docs:
	@echo "$(COLOR_BLUE)Building documentation...$(COLOR_RESET)"
	cargo doc --workspace --no-deps --open

# ============================================================================
# Run Commands
# ============================================================================

run:
	@echo "$(COLOR_BLUE)Running Ticca Desktop...$(COLOR_RESET)"
	cargo run -p ticca-ui

dev:
	@echo "$(COLOR_BLUE)Running Ticca Desktop with hot reload...$(COLOR_RESET)"
	@command -v cargo-watch >/dev/null 2>&1 || { echo "$(COLOR_YELLOW)Installing cargo-watch...$(COLOR_RESET)"; cargo install cargo-watch; }
	cargo watch -x 'run -p ticca-ui'

# ============================================================================
# Setup Commands
# ============================================================================

setup: install-deps
	@echo "$(COLOR_GREEN)✓ Setup complete$(COLOR_RESET)"
	@echo ""
	@echo "Next steps:"
	@echo "  1. Configure API keys in ~/.config/ticca/config.toml"
	@echo "  2. Run 'make run' to start the application"

install-deps:
	@echo "$(COLOR_BLUE)Installing Rust dependencies...$(COLOR_RESET)"
	rustup component add clippy rustfmt
	@command -v cargo-watch >/dev/null 2>&1 || cargo install cargo-watch
	@command -v cargo-audit >/dev/null 2>&1 || cargo install cargo-audit
	@echo "$(COLOR_BLUE)Installing Python dependencies...$(COLOR_RESET)"
	pip install -e ./python[dev]
	pip install ruff mypy

# ============================================================================
# Clean Commands
# ============================================================================

clean:
	@echo "$(COLOR_BLUE)Cleaning build artifacts...$(COLOR_RESET)"
	cargo clean
	rm -rf python/__pycache__
	rm -rf python/.pytest_cache
	rm -rf python/.mypy_cache
	rm -rf python/.ruff_cache
	rm -rf python/ticca_agent/__pycache__
	rm -rf python/tests/__pycache__
	rm -rf dist build
	find python -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null || true
	@echo "$(COLOR_GREEN)✓ Clean complete$(COLOR_RESET)"

# ============================================================================
# CI Commands (for GitHub Actions)
# ============================================================================

.PHONY: ci-check ci-test ci-lint

ci-check:
	cargo check --workspace --all-targets

ci-test:
	cargo test --workspace
	cd python && pytest tests/ -v --tb=short

ci-lint:
	cargo clippy --workspace --all-targets -- -D warnings
	cargo fmt --all -- --check
	cd python && ruff check .
	cd python && ruff format --check .
