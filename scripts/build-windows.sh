#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

DOCKER="${DOCKER:-$(which podman 2>/dev/null || which docker 2>/dev/null || echo podman)}"

"${DOCKER}" run --rm \
    -v cargo-cache:/usr/local/cargo/registry:Z \
    -v cargo-git:/usr/local/cargo/git:Z \
    -v "${PROJECT_ROOT}:/workspace:Z" \
    amaran-builder \
    cargo build --release --target x86_64-pc-windows-gnu
