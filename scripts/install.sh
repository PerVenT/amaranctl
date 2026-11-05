#!/usr/bin/env bash
# scripts/install.sh - Automated installer for amaranctl on Linux and macOS
set -e

echo "=========================================="
echo "        amaranctl UNIX Installer          "
echo "=========================================="
echo ""

CONFIG_DIR="$HOME/.config/amaranctl"
CONFIG_FILE="$CONFIG_DIR/config.json"
mkdir -p "$CONFIG_DIR"

# 1. Locate amaran.db helper
find_amaran_db() {
    local search_dirs=(
        "$HOME/Library/Application Support/amaran Desktop"
        "$HOME/.config/amaran Desktop"
        "$HOME/.wine/drive_c/users/$USER/AppData/Roaming/amaran Desktop"
        "$HOME/.var/app"
    )
    for dir in "${search_dirs[@]}"; do
        if [ -d "$dir" ]; then
            local found
            found=$(find "$dir" -name "amaran.db" 2>/dev/null | head -n 1)
            if [ -n "$found" ]; then
                echo "$found"
                return 0
            fi
        fi
    done
    return 1
}

# 2. Check existing configuration ("Parse, don't validate")
echo "[1/4] Checking configuration ($CONFIG_FILE)..."
NEEDS_EXTRACTION=1

if [ -f "$CONFIG_FILE" ]; then
    IS_VALID=0
    if command -v python3 &>/dev/null; then
        if python3 -c '
import sys, json
try:
    with open(sys.argv[1], "r", encoding="utf-8") as f:
        d = json.load(f)
    assert d.get("meshUuid") and isinstance(d["meshUuid"], str)
    assert d.get("netKey") and len(d["netKey"]) == 32
    assert d.get("appKey") and len(d["appKey"]) == 32
    assert isinstance(d.get("lights"), list) and len(d["lights"]) > 0
    for l in d["lights"]:
        assert l.get("mac") and l.get("address")
    sys.exit(0)
except Exception as e:
    sys.exit(1)
' "$CONFIG_FILE" 2>/dev/null; then
            IS_VALID=1
        fi
    else
        # Basic check if python3 is unavailable
        if grep -q "meshUuid" "$CONFIG_FILE" && grep -q "netKey" "$CONFIG_FILE" && grep -q "appKey" "$CONFIG_FILE"; then
            IS_VALID=1
        fi
    fi

    if [ "$IS_VALID" -eq 1 ]; then
        echo "    Found existing valid configuration at: $CONFIG_FILE"
        echo "    Skipping database extraction."
        NEEDS_EXTRACTION=0
    else
        echo "    [!] Existing config file is invalid or corrupted."
        DB_PATH=$(find_amaran_db || true)
        if [ -n "$DB_PATH" ] && [ -f "$DB_PATH" ]; then
            echo "    Found local database: $DB_PATH"
            read -r -p "    Backup invalid config ($CONFIG_FILE.bak) and extract fresh keys? (Y/n): " REPLACE_CHOICE
            REPLACE_CHOICE=${REPLACE_CHOICE:-Y}
            if [[ "$REPLACE_CHOICE" =~ ^[Yy]$ ]]; then
                cp "$CONFIG_FILE" "$CONFIG_FILE.bak"
                echo "    Backed up corrupted config to: $CONFIG_FILE.bak"
                NEEDS_EXTRACTION=1
            else
                echo "    Keeping current configuration as requested."
                NEEDS_EXTRACTION=0
            fi
        else
            echo "    [!] Could not locate amaran.db to regenerate configuration."
            echo "        Please fix $CONFIG_FILE manually or install Amaran Desktop."
            exit 1
        fi
    fi
fi

# 3. Extraction if needed
if [ "$NEEDS_EXTRACTION" -eq 1 ]; then
    echo "[2/4] Extracting Bluetooth Mesh credentials from database..."
    DB_PATH=$(find_amaran_db || true)
    if [ -z "$DB_PATH" ]; then
        echo "    [!] Could not automatically locate amaran.db."
        read -r -p "    Enter path to amaran.db manually (or leave blank to abort): " MANUAL_PATH
        if [ -n "$MANUAL_PATH" ] && [ -f "$MANUAL_PATH" ]; then
            DB_PATH="$MANUAL_PATH"
        else
            echo "    Aborted. No valid configuration or database found."
            exit 1
        fi
    fi

    echo "    Found database: $DB_PATH"
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
        echo "[!] Error: Neither python3 nor sqlite3 found. Cannot extract from SQLite database."
        exit 1
    fi
else
    echo "[2/4] Configuration ready (skipped database extraction)."
fi

# 4. Binary installation
echo "[3/4] Installing amaranctl binary..."
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

# 5. Autostart setup
echo "[4/4] Configuring Autostart..."
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
echo "Config:  $CONFIG_FILE"
echo "Binary:  $TARGET_BIN"
echo ""
echo "Usage:"
echo "  Tray app:  amaranctl run"
echo "  CLI:       amaranctl on | off | toggle | brightness 50 | cct 4500"
echo "  Hotkeys:   Ctrl+Alt+L (Toggle), Ctrl+Alt+Up/Down (Brightness), Ctrl+Alt+Left/Right (CCT)"
echo ""
