# ==============================================================================
# Makefile for amaranctl
# Cross-platform BLE mesh controller for Amaran lights
# ==============================================================================

SHELL := /usr/bin/env bash
.SHELLFLAGS := -euo pipefail -c
.DEFAULT_GOAL := help

# Version dynamically extracted from git tag, or fallback to short commit hash
GIT_TAG    := $(shell git describe --tags --exact-match 2>/dev/null)
GIT_COMMIT := $(shell git rev-parse --short HEAD 2>/dev/null || echo "unknown")
VERSION    ?= $(if $(GIT_TAG),$(GIT_TAG),$(if $(shell git describe --tags 2>/dev/null),$(shell git describe --tags),dev-$(GIT_COMMIT)))

# Container engine detection (Podman preferred, fallback to Docker)
DOCKER     ?= $(shell command -v podman 2>/dev/null || command -v docker 2>/dev/null || echo podman)
IMAGE_NAME ?= amaran-builder
IMAGE_TAG  ?= latest

# Buildx detection
BUILDX_CMD := $(shell if $(DOCKER) buildx version >/dev/null 2>&1; then echo "$(DOCKER) buildx build"; else echo "$(DOCKER) build"; fi)

# Container runner wrapper
RUN_IN_CONTAINER = $(DOCKER) run --rm \
	-v cargo-cache:/usr/local/cargo/registry:Z \
	-v cargo-git:/usr/local/cargo/git:Z \
	-v "$$(pwd):/workspace:Z" \
	$(IMAGE_NAME):$(IMAGE_TAG)

# ------------------------------------------------------------------------------
# Targets
# ------------------------------------------------------------------------------

.PHONY: help
help: ## Show this help message
	@echo "Usage: make [target] [VERSION=vX.Y.Z]"
	@echo ""
	@echo "Current Version: $(VERSION)"
	@echo "Container Engine: $(DOCKER)"
	@echo ""
	@echo "Targets:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'

.PHONY: image
image: ## Build cross-compilation builder container image using buildx
	@echo "==> Building container image '$(IMAGE_NAME):$(IMAGE_TAG)' via $(BUILDX_CMD)..."
	$(BUILDX_CMD) -t $(IMAGE_NAME):$(IMAGE_TAG) -f Dockerfile .

.PHONY: cargo
cargo: ## Run arbitrary cargo command inside builder container (e.g. make cargo ARGS="check")
	@$(RUN_IN_CONTAINER) cargo $(ARGS)

.PHONY: test
test: ## Run unit and integration tests inside builder container
	@echo "==> Running tests..."
	$(RUN_IN_CONTAINER) cargo test $(ARGS)

.PHONY: check
check: ## Run cargo check and clippy linter inside builder container
	@echo "==> Checking code and lints..."
	$(RUN_IN_CONTAINER) cargo clippy --all-targets -- -D warnings

.PHONY: fmt
fmt: ## Format Rust codebase using rustfmt
	@echo "==> Formatting code..."
	$(RUN_IN_CONTAINER) cargo fmt --all

.PHONY: fmt-check
fmt-check: ## Check code formatting using rustfmt without modifying
	@echo "==> Checking formatting..."
	$(RUN_IN_CONTAINER) cargo fmt --all -- --check

.PHONY: build-windows
build-windows: ## Cross-compile release binary for Windows (x86_64-pc-windows-gnu)
	@echo "==> Building Windows release binary..."
	$(RUN_IN_CONTAINER) cargo build --release --target x86_64-pc-windows-gnu

.PHONY: build-linux
build-linux: ## Compile release binary for Linux (x86_64-unknown-linux-gnu)
	@echo "==> Building Linux release binary..."
	$(RUN_IN_CONTAINER) cargo build --release --target x86_64-unknown-linux-gnu

.PHONY: build-all
build-all: build-windows build-linux ## Build release binaries for all supported targets

.PHONY: release
release: build-all ## Package release binaries with SHA256 checksums into dist/
	@echo "==> Packaging release $(VERSION)..."
	@mkdir -p dist
	@rm -rf dist/*
	@$(RUN_IN_CONTAINER) bash -c '\
		set -e; \
		if [ -f target/x86_64-pc-windows-gnu/release/amaranctl.exe ]; then \
			cp target/x86_64-pc-windows-gnu/release/amaranctl.exe dist/amaranctl.exe; \
			(cd dist && zip -9 "amaranctl-$(VERSION)-x86_64-pc-windows-gnu.zip" amaranctl.exe && rm amaranctl.exe); \
			echo "    Created dist/amaranctl-$(VERSION)-x86_64-pc-windows-gnu.zip"; \
		fi; \
		if [ -f target/x86_64-unknown-linux-gnu/release/amaranctl ]; then \
			cp target/x86_64-unknown-linux-gnu/release/amaranctl dist/amaranctl; \
			tar -czf "dist/amaranctl-$(VERSION)-x86_64-unknown-linux-gnu.tar.gz" -C dist amaranctl; \
			rm dist/amaranctl; \
			echo "    Created dist/amaranctl-$(VERSION)-x86_64-unknown-linux-gnu.tar.gz"; \
		fi; \
		echo "==> Generating SHA256 checksums..."; \
		(cd dist && sha256sum amaranctl-* > SHA256SUMS.txt); \
		echo "==> Release artifacts ready in dist/:"; \
		ls -la dist/ \
	'

.PHONY: install-windows
install-windows: ## Run Windows installation script (PowerShell)
	@if command -v pwsh >/dev/null 2>&1; then \
		pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/install.ps1; \
	elif command -v powershell.exe >/dev/null 2>&1; then \
		powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/install.ps1; \
	else \
		powershell -NoProfile -ExecutionPolicy Bypass -File scripts/install.ps1; \
	fi

.PHONY: install-linux
install-linux: ## Run Linux installation script (Bash)
	@bash scripts/install.sh

.PHONY: demo
demo: ## Process raw screen recording or render demo GIF & MP4 (e.g. make demo ARGS="-InputFile raw.mp4")
	@if command -v pwsh >/dev/null 2>&1; then \
		pwsh -NoProfile -ExecutionPolicy Bypass -File scripts/render-demo.ps1 $(ARGS); \
	elif command -v powershell.exe >/dev/null 2>&1; then \
		powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/render-demo.ps1 $(ARGS); \
	else \
		powershell -NoProfile -ExecutionPolicy Bypass -File scripts/render-demo.ps1 $(ARGS); \
	fi

.PHONY: clean
clean: ## Clean build target and dist directories
	@echo "==> Cleaning build artifacts..."
	rm -rf target/ dist/
