# amaranctl

[![CI](https://github.com/amaranctl/amaranctl/actions/workflows/ci.yml/badge.svg)](https://github.com/amaranctl/amaranctl/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**amaranctl** is an ultra-lightweight, high-performance native controller and background tray daemon for Aputure Amaran studio lights (such as the 60x, 60x S, 100d, 200x, and similar models).

Control your lighting instantly from your **system tray**, with **global hotkeys**, or through lightning-fast **terminal CLI commands** over Bluetooth Low Energy (BLE Mesh).

---

### Special Thanks & Acknowledgements

Special thanks and sincere gratitude to **[Wes Bos](https://github.com/wesbos)** and his project **[`amaran-BLE-control`](https://github.com/wesbos/amaran-BLE-control)**. Wes deciphered and openly documented the Amaran Bluetooth Mesh protocol and packet structures. Without his pioneering reverse-engineering work, creating this native project would have been immensely harder.

---

## Why amaranctl?

The official Amaran Desktop application is built on heavy web frameworks:
* **Massive footprint:** Over 500 MB download size and hundreds of megabytes of RAM consumption.
* **Mandatory Cloud Login:** Requires an account, password, and online connection just to control a lamp on your desk.
* **Sluggish startup:** Takes 20–30 seconds to launch.

**amaranctl** replaces this with a clean, native solution:
* **Standalone binary:** Under 1 MB (release build).
* **Minimal memory:** Less than 10 MB RAM in the background.
* **100% Local & Private:** Direct BLE Mesh connection. Zero cloud accounts, zero telemetry, no internet connection needed.
* **Bi-directional Status Polling:** Regularly syncs actual lamp power, brightness, and color temperature directly from hardware.
* **Adaptive Tray Icon:** Dynamically renders the live status (connecting, off, or glowing with the actual CCT Kelvin color gradient from 2700K warm tungsten to 6500K daylight).
* **Instant IPC Execution (<50 ms):** Terminal CLI commands communicate with the background tray daemon over native local IPC (Windows Named Pipes or UNIX domain sockets).
* **Global Hotkeys:** Adjust brightness and color temperature at any time without switching windows or minimizing your game or video call.

---

## Architecture & Paths

To ensure clean system integration without clutter, `amaranctl` uses standard user locations:

| Component | Windows Path | Linux Path |
| :--- | :--- | :--- |
| **Canonical Binary** | `~/.local/bin/amaranctl.exe` | `~/.local/bin/amaranctl` |
| **Configuration** | `~/.config/amaranctl/config.json` | `~/.config/amaranctl/config.json` |
| **Autostart** | Windows Registry (`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`) | Systemd user unit (`~/.config/systemd/user/amaranctl.service`) |
| **IPC Endpoint** | `\\.\pipe\amaranctl` (Named Pipe) | `/tmp/amaranctl.sock` (UNIX Domain Socket) |

---

## Configuration

`amaranctl` requires your local mesh network keys to securely authenticate with your lamps.

### Automatic Import (Zero Manual Configuration)
On first launch, if no config is found at `~/.config/amaranctl/config.json`, `amaranctl` automatically locates your local Amaran Desktop SQLite database (`amaran.db`), extracts your mesh UUID, network key, application key, and paired lights, and writes a clean config file to `~/.config/amaranctl/config.json`.

### Manual Configuration
An example template is provided in [`config.json.example`](config.json.example):

```json
{
  "meshUuid": "00112233445566778899aabbccddeeff",
  "netKey": "00112233445566778899AABBCCDDEEFF",
  "appKey": "FFEEDDCCBBAA99887766554433221100",
  "lights": [
    {
      "name": "Amaran 60x S",
      "mac": "AA:BB:CC:DD:EE:FF",
      "address": 2,
      "deviceKey": "0123456789ABCDEF0123456789ABCDEF",
      "deviceUuid": "00112233445566778899AABBCCDDEEFF",
      "code": "400M5"
    }
  ]
}
```

Copy this file to `~/.config/amaranctl/config.json` and fill in your device credentials.

---

## Installation

All installation and maintenance scripts reside in [`scripts/`](scripts/).

### Windows (PowerShell)

Run the automated installer script:

```powershell
pwsh -ExecutionPolicy Bypass -File scripts/install.ps1
```

Or pass `-StartDaemon` to immediately start the background tray daemon:
```powershell
pwsh -ExecutionPolicy Bypass -File scripts/install.ps1 -StartDaemon
```

What the installer does:
1. Builds the release binary via container (or downloads from GitHub releases if `-SkipBuild` is passed).
2. Stops any running daemon instance cleanly.
3. Installs the canonical binary into `~/.local/bin/amaranctl.exe`.
4. Adds `~/.local/bin` to your User `PATH` if not already present.
5. Cleans up any legacy binary locations.
6. Configures Windows Autostart in the registry (`"~/.local/bin/amaranctl.exe" run`).

### Linux / macOS (Bash)

```bash
bash scripts/install.sh
```

What the script does:
1. Installs the executable into `~/.local/bin/amaranctl`.
2. Adds `~/.local/bin` to your `PATH` advice.
3. Optionally registers and enables a `systemd --user` background service for seamless autostart.

---

## How to Use

### 1. System Tray Daemon

Run the daemon:
```bash
amaranctl run
```

* **Adaptive Tray Icon:**
  * 🟡 **Pulsing Yellow:** Connecting to BLE mesh.
  * 🔴 **Red:** No connection / lamp power disconnected.
  * ⚪ **Dim Gray:** Lamp connected, but powered OFF.
  * 💡 **Adaptive Color:** Lamp is ON; icon dynamically renders the exact color temperature (from warm 2700K to daylight 6500K) with a brightness intensity halo.
* **Tray Menu Controls:**
  * **Status:** Displays real-time status and values (e.g. `Amaran 60x S — 🟢 ON | 65% | 4500K`).
  * **Power:** Instant toggle on/off.
  * **Brightness Presets:** 10%, 25%, 50%, 75%, 100%.
  * **Color Temperature Presets:** 2700K (Warm), 3200K (Tungsten), 4500K (Neutral), 5600K (Daylight), 6500K (Cool White).
  * **Start on boot:** Toggle silent autostart on system login.
  * **Quit:** Exit background daemon.

* **Global Keyboard Shortcuts (active system-wide):**
  | Shortcut | Action |
  | :--- | :--- |
  | `Ctrl + Alt + L` | **Toggle** Light On / Off |
  | `Ctrl + Alt + Up` | **Increase Brightness** by +10% |
  | `Ctrl + Alt + Down` | **Decrease Brightness** by -10% |
  | `Ctrl + Alt + Right` | **Increase Temperature** by +200K (Cooler) |
  | `Ctrl + Alt + Left` | **Decrease Temperature** by -200K (Warmer) |

---

### 2. Command-Line Interface (CLI)

When the tray daemon is running, CLI commands execute in **under 50 milliseconds** via local IPC. If the daemon is not running, the CLI seamlessly connects directly via Bluetooth, executes the command, and exits.

```bash
# Check current lamp status
amaranctl status
# Output: Amaran 60x S: ON | Brightness: 65.0% | CCT: 4500K

# Machine-readable JSON output
amaranctl status --json
# Output: {"connected":true,"connecting":false,"power":true,"brightness":65.0,"cct":4500,"gm":0}

# Power control
amaranctl on            # Turn light on
amaranctl off           # Turn light off
amaranctl toggle        # Toggle power state

# Brightness control (0 to 100% or relative adjustment)
amaranctl brightness 50     # Set to 50%
amaranctl brightness +10    # Increase brightness by 10%
amaranctl brightness -- -5  # Decrease brightness by 5%

# Color temperature (2700K to 6500K or relative adjustment)
amaranctl cct 3200          # Set to 3200K
amaranctl cct +200          # Shift 200K cooler
amaranctl cct -- -200       # Shift 200K warmer
```

---

## Developer Guide & Makefile

The project uses a standard `Makefile` with automatic container engine detection (`podman` or `docker buildx`) and dynamic git tag versioning.

```bash
make help
```

### Common Targets

* **`make image`**: Builds the builder container (`Dockerfile`) using `buildx`.
* **`make test`**: Runs the entire test suite (`cargo test`) inside the container.
* **`make check`**: Runs `cargo clippy --all-targets -- -D warnings`.
* **`make fmt`**: Automatically formats the codebase with `rustfmt`.
* **`make fmt-check`**: Validates formatting without modifying files (used in CI).
* **`make build-windows`**: Cross-compiles `amaranctl.exe` for Windows (`x86_64-pc-windows-gnu`).
* **`make build-linux`**: Compiles `amaranctl` for Linux (`x86_64-unknown-linux-gnu`).
* **`make release`**: Packages binaries into `dist/` as `.zip` and `.tar.gz` with version from git tag and computes `SHA256SUMS.txt`.
* **`make clean`**: Cleans `target/` and `dist/` directories.

---

## CI / CD & Multi-Arch Releases

The repository includes GitHub Actions workflows configured in [`.github/workflows/`](.github/workflows/):

1. **Continuous Integration (`ci.yml`):**
   * Triggered on pull requests and pushes to `main`.
   * Checks code formatting (`rustfmt`).
   * Runs Clippy linter with zero warnings (`cargo clippy`).
   * Executes full unit and integration test suite.

2. **Automated Release (`release.yml`):**
   * Triggered on pushing a release tag (`v*`, e.g. `v0.1.0`).
   * Builds release binaries for:
     * **Windows x86_64** (`x86_64-pc-windows-msvc`)
     * **Linux x86_64** (`x86_64-unknown-linux-gnu`)
     * **Linux ARM64** (`aarch64-unknown-linux-gnu` via native ARM runner)
   * Packages archives with SHA256 checksums (`SHA256SUMS.txt`).
   * Automatically publishes the GitHub Release with generated release notes and downloadable assets.

---

## License

MIT License. Designed with care for creators, developers, and streamers.
