#!/usr/bin/env bash
# scripts/install.sh - Automated installer for amaranctl on Linux and macOS
set -e

echo "=========================================="
echo "        amaranctl UNIX Installer          "
echo "=========================================="
echo ""

# 1. Binary installation
echo "[1/2] Installing amaranctl binary..."
BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"
TARGET_BIN="$BIN_DIR/amaranctl"

OS_TYPE=$(uname -s)
REPO="amaranctl/amaranctl"

if [ "$OS_TYPE" = "Linux" ]; then
    DOWNLOAD_URL="https://github.com/$REPO/releases/latest/download/amaranctl-linux-x86_64"
elif [ "$OS_TYPE" = "Darwin" ]; then
    DOWNLOAD_URL="https://github.com/$REPO/releases/latest/download/amaranctl-macos-universal"
fi

if [ -f "./target/release/amaranctl" ]; then
    cp "./target/release/amaranctl" "$TARGET_BIN"
    chmod +x "$TARGET_BIN"
    echo "    Installed local binary to: $TARGET_BIN"
else
    echo "    Downloading $DOWNLOAD_URL..."
    if command -v curl &>/dev/null; then
        curl -fsSL "$DOWNLOAD_URL" -o "$TARGET_BIN" || true
    elif command -v wget &>/dev/null; then
        wget -qO "$TARGET_BIN" "$DOWNLOAD_URL" || true
    fi
    if [ -f "$TARGET_BIN" ]; then
        chmod +x "$TARGET_BIN"
        echo "    Installed binary to: $TARGET_BIN"
    else
        echo "[!] Could not download binary automatically. Please place 'amaranctl' into $BIN_DIR"
    fi
fi

if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo "    Note: Add ~/.local/bin to your PATH in ~/.bashrc or ~/.zshrc:"
    echo "          export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

# 2. Autostart setup
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
echo "  CLI:       amaranctl on | off | toggle | brightness 50 | cct 4500"
echo "  Hotkeys:   Ctrl+Alt+L (Toggle), Ctrl+Alt+Up/Down (Brightness), Ctrl+Alt+Left/Right (CCT)"
echo ""
