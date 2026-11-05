#!/usr/bin/env bash
# scripts/install.sh - Automated installer for amaranctl on Linux and macOS
set -e

echo "=========================================="
echo "        amaranctl UNIX Installer          "
echo "=========================================="
echo ""

# 1. Locate amaran.db
echo "[1/5] Searching for Amaran Desktop database..."
DB_PATH=""

# Search common paths
SEARCH_DIRS=(
    "$HOME/Library/Application Support/amaran Desktop"
    "$HOME/.config/amaran Desktop"
    "$HOME/.wine/drive_c/users/$USER/AppData/Roaming/amaran Desktop"
    "$HOME/.var/app"
)

for dir in "${SEARCH_DIRS[@]}"; do
    if [ -d "$dir" ]; then
        FOUND=$(find "$dir" -name "amaran.db" 2>/dev/null | head -n 1)
        if [ -n "$FOUND" ]; then
            DB_PATH="$FOUND"
            break
        fi
    fi
done

if [ -z "$DB_PATH" ]; then
    echo "[!] Could not automatically locate amaran.db."
    echo "    Make sure Amaran Desktop is installed, or enter path manually:"
    read -r -p "    Path to amaran.db (or leave blank to skip key extraction): " MANUAL_PATH
    if [ -n "$MANUAL_PATH" ] && [ -f "$MANUAL_PATH" ]; then
        DB_PATH="$MANUAL_PATH"
    fi
fi

CONFIG_DIR="$HOME/.config/amaranctl"
mkdir -p "$CONFIG_DIR"
CONFIG_FILE="$CONFIG_DIR/config.json"

# 2. Extract keys if DB_PATH found
if [ -n "$DB_PATH" ]; then
    echo "    Found database: $DB_PATH"
    echo "[2/5] Extracting Bluetooth Mesh credentials..."
    if command -v python3 &>/dev/null; then
        python3 - "$DB_PATH" "$CONFIG_FILE" << 'EOF'
import sys, sqlite3, json

db_path = sys.argv[1]
out_path = sys.argv[2]

conn = sqlite3.connect(db_path)
cur = conn.cursor()

mesh_row = cur.execute("SELECT uuid, net_key, app_key FROM mesh LIMIT 1;").fetchone()
if not mesh_row:
    sys.exit("Error: No mesh found in database.")

uuid, net_key, app_key = mesh_row

fixtures = []
for row in cur.execute("SELECT name, mac_address, node_address, device_key, device_uuid, code FROM fixtures;").fetchall():
    name, mac, node_addr, dev_key, dev_uuid, code = row
    fixtures.append({
        "name": name,
        "mac": mac,
        "address": node_addr,
        "deviceKey": dev_key,
        "deviceUuid": dev_uuid,
        "code": code
    })

config = {
    "meshUuid": uuid,
    "netKey": net_key,
    "appKey": app_key,
    "lights": fixtures
}

with open(out_path, "w", encoding="utf-8") as f:
    json.dump(config, f, indent=2)

print(f"    Saved configuration to: {out_path}")
EOF
    elif command -v sqlite3 &>/dev/null; then
        UUID=$(sqlite3 "$DB_PATH" "SELECT uuid FROM mesh LIMIT 1;")
        NET_KEY=$(sqlite3 "$DB_PATH" "SELECT net_key FROM mesh LIMIT 1;")
        APP_KEY=$(sqlite3 "$DB_PATH" "SELECT app_key FROM mesh LIMIT 1;")
        echo "{\"meshUuid\":\"$UUID\",\"netKey\":\"$NET_KEY\",\"appKey\":\"$APP_KEY\",\"lights\":[]}" > "$CONFIG_FILE"
        echo "    Saved basic configuration to: $CONFIG_FILE"
    else
        echo "[!] Warning: Neither python3 nor sqlite3 found. Please extract config.json manually."
    fi
else
    echo "[2/5] Skipping key extraction (no database found)."
fi

# 3. Binary installation
echo "[3/5] Installing amaranctl binary..."
BIN_DIR="$HOME/.local/bin"
mkdir -p "$BIN_DIR"
TARGET_BIN="$BIN_DIR/amaranctl"

OS_TYPE=$(uname -s)
ARCH_TYPE=$(uname -m)

REPO="amaranctl/amaranctl"
if [ "$OS_TYPE" = "Linux" ]; then
    DOWNLOAD_URL="https://github.com/$REPO/releases/latest/download/amaranctl-linux-x86_64"
elif [ "$OS_TYPE" = "Darwin" ]; then
    DOWNLOAD_URL="https://github.com/$REPO/releases/latest/download/amaranctl-macos-universal"
fi

if [ -f "./target/release/amaranctl" ]; then
    cp "./target/release/amaranctl" "$TARGET_BIN"
    chmod +x "$TARGET_BIN"
    echo "    Installed local binary to $TARGET_BIN"
else
    echo "    Downloading $DOWNLOAD_URL..."
    if command -v curl &>/dev/null; then
        curl -fsSL "$DOWNLOAD_URL" -o "$TARGET_BIN" || true
    elif command -v wget &>/dev/null; then
        wget -qO "$TARGET_BIN" "$DOWNLOAD_URL" || true
    fi
    if [ -f "$TARGET_BIN" ]; then
        chmod +x "$TARGET_BIN"
        echo "    Installed binary to $TARGET_BIN"
    else
        echo "[!] Could not download binary automatically. Please place 'amaranctl' into $BIN_DIR"
    fi
fi

# Ensure ~/.local/bin is in PATH
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo "    Note: Add ~/.local/bin to your PATH in ~/.bashrc or ~/.zshrc:"
    echo "          export PATH=\"\$HOME/.local/bin:\$PATH\""
fi

# 4. Autostart setup
echo "[4/5] Configuring Autostart..."
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
echo "Usage:"
echo "  Tray app:  amaranctl run"
echo "  CLI:       amaranctl on | off | toggle | brightness 50 | cct 4500"
echo "  Hotkeys:   Ctrl+Alt+L (Toggle), Ctrl+Alt+Up/Down (Brightness), Ctrl+Alt+Left/Right (CCT)"
echo ""
