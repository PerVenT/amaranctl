#!/usr/bin/env bash
# scripts/install.sh - Automated installer for amaranctl on Linux and macOS
set -euo pipefail

echo "=========================================="
echo "        amaranctl UNIX Installer          "
echo "=========================================="
echo ""

# 1. Binary installation
echo "[1/2] Installing amaranctl binary..."
BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"
TARGET_BIN="$BIN_DIR/amaranctl"

OS_TYPE="$(uname -s)"
ARCH="$(uname -m)"
REPO="amaranctl/amaranctl"

LOCAL_BIN=""
if [ -f "./target/release/amaranctl" ]; then
    LOCAL_BIN="./target/release/amaranctl"
elif [ -f "./target/x86_64-unknown-linux-gnu/release/amaranctl" ]; then
    LOCAL_BIN="./target/x86_64-unknown-linux-gnu/release/amaranctl"
elif [ -f "./target/aarch64-unknown-linux-gnu/release/amaranctl" ]; then
    LOCAL_BIN="./target/aarch64-unknown-linux-gnu/release/amaranctl"
fi

if [ -n "$LOCAL_BIN" ]; then
    echo "    Found local build: $LOCAL_BIN"
    cp "$LOCAL_BIN" "$TARGET_BIN"
    chmod +x "$TARGET_BIN"
    echo "    Installed local binary to: $TARGET_BIN"
else
    DOWNLOAD_TAG="latest"
    if [ "$OS_TYPE" = "Linux" ]; then
        if [ "$ARCH" = "aarch64" ] || [ "$ARCH" = "arm64" ]; then
            DOWNLOAD_URL="https://github.com/$REPO/releases/$DOWNLOAD_TAG/download/amaranctl-aarch64-unknown-linux-gnu.tar.gz"
        else
            DOWNLOAD_URL="https://github.com/$REPO/releases/$DOWNLOAD_TAG/download/amaranctl-x86_64-unknown-linux-gnu.tar.gz"
        fi
    elif [ "$OS_TYPE" = "Darwin" ]; then
        DOWNLOAD_URL="https://github.com/$REPO/releases/$DOWNLOAD_TAG/download/amaranctl-apple-darwin.tar.gz"
    fi

    echo "    Local build not found. Downloading from $DOWNLOAD_URL..."
    TMP_DIR="$(mktemp -d)"
    if command -v curl &>/dev/null; then
        curl -fsSL "$DOWNLOAD_URL" -o "$TMP_DIR/amaranctl.tar.gz" || true
    elif command -v wget &>/dev/null; then
        wget -qO "$TMP_DIR/amaranctl.tar.gz" "$DOWNLOAD_URL" || true
    fi

    if [ -f "$TMP_DIR/amaranctl.tar.gz" ]; then
        tar -xzf "$TMP_DIR/amaranctl.tar.gz" -C "$TMP_DIR"
        if [ -f "$TMP_DIR/amaranctl" ]; then
            mv "$TMP_DIR/amaranctl" "$TARGET_BIN"
            chmod +x "$TARGET_BIN"
            echo "    Installed binary to: $TARGET_BIN"
        fi
    fi
    rm -rf "$TMP_DIR"

    if [ ! -f "$TARGET_BIN" ]; then
        echo "[!] Could not download binary automatically. Please build using 'make build-linux' and run this script again."
    fi
fi

if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo "    Note: Add ~/.local/bin to your PATH in ~/.bashrc or ~/.zshrc:"
    echo "          export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

# 2. Autostart setup via systemd user unit
echo "[2/2] Configuring Autostart..."
if [ "$OS_TYPE" = "Linux" ] && command -v systemctl &>/dev/null; then
    read -r -p "Enable systemd user service for amaranctl background tray? (Y/n): " AUTOSTART_CHOICE
    AUTOSTART_CHOICE=${AUTOSTART_CHOICE:-Y}
    if [[ "$AUTOSTART_CHOICE" =~ ^[Yy]$ ]]; then
        SYSTEMD_DIR="$HOME/.config/systemd/user"
        mkdir -p "$SYSTEMD_DIR"
        cat > "$SYSTEMD_DIR/amaranctl.service" << EOF
[Unit]
Description=Amaran Lights Bluetooth Mesh Controller
After=bluetooth.target

[Service]
ExecStart=$TARGET_BIN run
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
EOF
        systemctl --user daemon-reload
        systemctl --user enable amaranctl.service
        echo "    systemd user service enabled!"
    fi
fi

echo ""
echo "=========================================="
echo "        Installation Successful!          "
echo "=========================================="
echo "Binary:  $TARGET_BIN"
echo ""
echo "Usage:"
echo "  Tray app:  amaranctl run"
echo "  CLI:       amaranctl status"
echo "             amaranctl on | off | toggle | brightness 50 | cct 4500"
echo "  Hotkeys:   Ctrl+Alt+L (Toggle), Ctrl+Alt+Up/Down (Brightness), Ctrl+Alt+Left/Right (CCT)"
echo ""
