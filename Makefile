# Sage Agent Makefile

.PHONY: help build test clean install dev check fmt clippy docs examples doc-check doc-status guard guard-strict guard-bash-check arch-guard

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
	@echo "  examples   - Run smoke-test examples"
	@echo "  doc-check  - Check documentation consistency"
	@echo "  doc-status - Show documentation status"
	@echo ""
	@echo "Guards:"
	@echo "  guard        - Run VibeGuard checks (report only)"
	@echo "  guard-strict - Run VibeGuard checks (fail on violations)"
	@echo "  arch-guard   - Run architecture guard tests"
	@echo ""
	@echo "Usage:"
	@echo "  run        - Run sage with arguments (e.g., make run ARGS='--help')"

# Building
build:
	cargo build --workspace

release:
	cargo build --workspace --release

install:
	cargo install --path crates/sage-cli

# Testing
test:
	cargo test --workspace --all-targets

test-unit:
	cargo test --workspace --lib

test-int:
	cargo test --workspace --tests

# Development
dev:
	cargo run --bin sage

check:
	cargo check --workspace --all-targets --all-features

fmt:
	cargo fmt

clippy:
	cargo clippy --workspace --all-targets --all-features -- -D warnings

clean:
	cargo clean
	rm -f trajectory_*.json
	rm -rf target/

# Documentation
docs:
	cargo doc --workspace --open

examples:
	@echo "Running examples..."
	cargo run -p sage --example read_tool_demo
	cargo run -p sage --example grep_demo
	cargo run -p sage --example planning_demo

# Run with arguments
run:
	cargo run --bin sage -- $(ARGS)

# VibeGuard checks
VIBEGUARD_DIR ?= $(HOME)/Desktop/code/AI/tool/vibeguard
VIBEGUARD_BASH ?= bash

guard-bash-check:
	@$(VIBEGUARD_BASH) -c 'if [ "$${BASH_VERSINFO[0]:-0}" -lt 4 ]; then echo "VibeGuard Rust guards require Bash >= 4. Set VIBEGUARD_BASH to a modern bash, for example /opt/homebrew/bin/bash. Current: $${BASH_VERSION:-unknown}" >&2; exit 2; fi'

guard: guard-bash-check
	@echo "Running VibeGuard Rust guards..."
	@$(VIBEGUARD_BASH) $(VIBEGUARD_DIR)/guards/rust/check_duplicate_types.sh .
	@$(VIBEGUARD_BASH) $(VIBEGUARD_DIR)/guards/rust/check_nested_locks.sh .
	@$(VIBEGUARD_BASH) $(VIBEGUARD_DIR)/guards/rust/check_unwrap_in_prod.sh .

guard-strict: guard-bash-check
	@echo "Running VibeGuard Rust guards (strict)..."
	$(VIBEGUARD_BASH) $(VIBEGUARD_DIR)/guards/rust/check_duplicate_types.sh --strict .
	$(VIBEGUARD_BASH) $(VIBEGUARD_DIR)/guards/rust/check_nested_locks.sh --strict .

arch-guard:
	@echo "Running architecture guard tests..."
	cargo test --package sage-core --test architecture_guards -- --nocapture

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
