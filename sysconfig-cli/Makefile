# Makefile for sysconfig-cli

# Variables
CARGO = cargo
INSTALL_DIR ?= /usr/local/bin
BINARY_NAME = sysconfig-cli
TARGET_DIR = target
RELEASE_BINARY = $(TARGET_DIR)/release/$(BINARY_NAME)
DEBUG_BINARY = $(TARGET_DIR)/debug/$(BINARY_NAME)
# Socket is auto-detected based on user (can be overridden)
SOCKET ?=

# Default target
.PHONY: all
all: build

# Build targets
.PHONY: build
build:
	@echo "Building $(BINARY_NAME) in debug mode..."
	@$(CARGO) build

.PHONY: release
release:
	@echo "Building $(BINARY_NAME) in release mode..."
	@$(CARGO) build --release

# Test targets
.PHONY: test
test:
	@echo "Running tests..."
	@$(CARGO) test

.PHONY: test-verbose
test-verbose:
	@echo "Running tests with verbose output..."
	@$(CARGO) test -- --nocapture

# Run targets
.PHONY: run
run: build
	@$(DEBUG_BINARY) --help

.PHONY: run-get
run-get: build
	@$(DEBUG_BINARY) $(if $(SOCKET),--socket $(SOCKET)) get

.PHONY: run-watch
run-watch: build
	@$(DEBUG_BINARY) $(if $(SOCKET),--socket $(SOCKET)) watch

# Installation targets
.PHONY: install
install: release
	@echo "Installing $(BINARY_NAME) to $(INSTALL_DIR)..."
	@install -d $(INSTALL_DIR)
	@install -m 755 $(RELEASE_BINARY) $(INSTALL_DIR)/$(BINARY_NAME)
	@echo "Installation complete. $(BINARY_NAME) installed to $(INSTALL_DIR)"

.PHONY: uninstall
uninstall:
	@echo "Uninstalling $(BINARY_NAME) from $(INSTALL_DIR)..."
	@rm -f $(INSTALL_DIR)/$(BINARY_NAME)
	@echo "Uninstall complete."

# Development targets
.PHONY: check
check:
	@echo "Running cargo check..."
	@$(CARGO) check

.PHONY: fmt
fmt:
	@echo "Formatting code..."
	@$(CARGO) fmt

.PHONY: fmt-check
fmt-check:
	@echo "Checking code formatting..."
	@$(CARGO) fmt -- --check

.PHONY: clippy
clippy:
	@echo "Running clippy..."
	@$(CARGO) clippy -- -W clippy::all

.PHONY: lint
lint: fmt-check clippy

# Documentation
.PHONY: doc
doc:
	@echo "Building documentation..."
	@$(CARGO) doc --no-deps

.PHONY: doc-open
doc-open:
	@echo "Building and opening documentation..."
	@$(CARGO) doc --no-deps --open

# Demo and examples
.PHONY: demo
demo: build
	@echo "Running demo script..."
	@./demo.sh

.PHONY: example-get
example-get: build
	@echo "Example: Getting current state"
	@$(DEBUG_BINARY) $(if $(SOCKET),--socket $(SOCKET)) get --format pretty

.PHONY: example-set
example-set: build
	@echo "Example: Setting a value"
	@$(DEBUG_BINARY) $(if $(SOCKET),--socket $(SOCKET)) set '$.test.example' '"Hello from Makefile"'

.PHONY: example-watch
example-watch: build
	@echo "Example: Watching for changes"
	@echo "Press Ctrl+C to stop"
	@$(DEBUG_BINARY) $(if $(SOCKET),--socket $(SOCKET)) watch --format pretty

# Cleaning
.PHONY: clean
clean:
	@echo "Cleaning build artifacts..."
	@$(CARGO) clean

.PHONY: clean-all
clean-all: clean
	@echo "Removing all generated files..."
	@rm -rf Cargo.lock

# Help target
.PHONY: help
help:
	@echo "Sysconfig CLI Makefile"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Build targets:"
	@echo "  build          Build in debug mode (default)"
	@echo "  release        Build in release mode"
	@echo ""
	@echo "Test targets:"
	@echo "  test           Run tests"
	@echo "  test-verbose   Run tests with verbose output"
	@echo ""
	@echo "Run targets:"
	@echo "  run            Show help"
	@echo "  run-get        Get current state"
	@echo "  run-watch      Watch for state changes"
	@echo ""
	@echo "Installation:"
	@echo "  install        Install to $(INSTALL_DIR) (may require sudo)"
	@echo "  uninstall      Remove from $(INSTALL_DIR) (may require sudo)"
	@echo ""
	@echo "Development:"
	@echo "  check          Run cargo check"
	@echo "  fmt            Format code"
	@echo "  fmt-check      Check code formatting"
	@echo "  clippy         Run clippy linter"
	@echo "  lint           Run all linters (fmt-check + clippy)"
	@echo ""
	@echo "Documentation:"
	@echo "  doc            Build documentation"
	@echo "  doc-open       Build and open documentation"
	@echo ""
	@echo "Examples:"
	@echo "  demo           Run interactive demo"
	@echo "  example-get    Example: get state"
	@echo "  example-set    Example: set value"
	@echo "  example-watch  Example: watch changes"
	@echo ""
	@echo "Maintenance:"
	@echo "  clean          Clean build artifacts"
	@echo "  clean-all      Clean everything including Cargo.lock"
	@echo ""
	@echo "Environment variables:"
	@echo "  INSTALL_DIR    Installation directory (default: /usr/local/bin)"
	@echo "  SOCKET         Override socket path (auto-detected based on user)"
	@echo ""
	@echo "Examples:"
	@echo "  make release"
	@echo "  sudo make install"
	@echo "  make run-get                           # Uses auto-detected socket"
	@echo "  SOCKET=/tmp/sysconfig.sock make run-get  # Override socket path"
	@echo "  INSTALL_DIR=~/.local/bin make install"

# Set default goal
.DEFAULT_GOAL := all
