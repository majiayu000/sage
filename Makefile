# Sage Agent Makefile

.PHONY: help build test clean install dev check fmt clippy docs examples doc-check doc-status guard guard-strict

# Default target
help:
	@echo "Sage Agent - Development Commands"
	@echo "================================="
	@echo ""
	@echo "Building:"
	@echo "  build      - Build the project in debug mode"
	@echo "  release    - Build the project in release mode"
	@echo "  install    - Install sage CLI globally"
	@echo ""
	@echo "Testing:"
	@echo "  test       - Run all tests"
	@echo "  test-unit  - Run unit tests only"
	@echo "  test-int   - Run integration tests only"
	@echo ""
	@echo "Development:"
	@echo "  dev        - Run in development mode"
	@echo "  check      - Check code without building"
	@echo "  fmt        - Format code"
	@echo "  clippy     - Run clippy linter"
	@echo "  clean      - Clean build artifacts"
	@echo ""
	@echo "Documentation:"
	@echo "  docs       - Generate and open documentation"
	@echo "  examples   - Run all examples"
	@echo "  doc-check  - Check documentation consistency"
	@echo "  doc-status - Show documentation status"
	@echo ""
	@echo "Guards:"
	@echo "  guard        - Run VibeGuard checks (report only)"
	@echo "  guard-strict - Run VibeGuard checks (fail on violations)"
	@echo ""
	@echo "Usage:"
	@echo "  run        - Run sage with arguments (e.g., make run ARGS='--help')"

# Building
build:
	cargo build

release:
	cargo build --release

install:
	cargo install --path crates/sage-cli

# Testing
test:
	cargo test

test-unit:
	cargo test --lib

test-int:
	cargo test --test integration_test

# Development
dev:
	cargo run --bin sage

check:
	cargo check

fmt:
	cargo fmt

clippy:
	cargo clippy -- -D warnings

clean:
	cargo clean
	rm -f trajectory_*.json
	rm -rf target/

# Documentation
docs:
	cargo doc --open

examples:
	@echo "Running examples..."
	cargo run --example basic_usage
	cargo run --example markdown_demo
	cargo run --example ui_demo

# Run with arguments
run:
	cargo run --bin sage -- $(ARGS)

# VibeGuard checks
VIBEGUARD_DIR ?= $(HOME)/Desktop/code/AI/tools/vibeguard

guard:
	@echo "Running VibeGuard Rust guards..."
	@bash $(VIBEGUARD_DIR)/guards/rust/check_duplicate_types.sh .
	@bash $(VIBEGUARD_DIR)/guards/rust/check_nested_locks.sh .
	@bash $(VIBEGUARD_DIR)/guards/rust/check_unwrap_in_prod.sh .

guard-strict:
	@echo "Running VibeGuard Rust guards (strict)..."
	bash $(VIBEGUARD_DIR)/guards/rust/check_duplicate_types.sh --strict .
	bash $(VIBEGUARD_DIR)/guards/rust/check_nested_locks.sh --strict .

# Quick development cycle
quick: fmt clippy test

# Full CI check
ci: fmt clippy test build guard

# Documentation consistency
doc-check:
	@echo "🔍 Checking documentation consistency..."
	@python3 scripts/check_doc_consistency.py

doc-status:
	@echo "📊 Documentation Status:"
	@echo "English README: $$(wc -l < README.md) lines"
	@echo "Chinese README: $$(wc -l < README_zh.md) lines"
	@echo "Last modified:"
	@ls -la README*.md | awk '{print "  " $$9 ": " $$6 " " $$7 " " $$8}'

# Setup development environment
setup:
	rustup update
	rustup component add rustfmt clippy
	@echo "Development environment ready!"
