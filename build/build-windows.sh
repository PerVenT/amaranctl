#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

podman run --rm \
    -v "${PROJECT_ROOT}:/workspace:Z" \
    amaran-builder \
    cargo build --release --target x86_64-pc-windows-gnu
