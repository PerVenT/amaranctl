# amaranctl

Ultra-lightweight native controller for Aputure Amaran studio lights. Control your lighting instantly from your system tray, global keyboard shortcuts, or terminal scripts over Bluetooth Low Energy.

---

### Special Thanks & Acknowledgements

Special thanks and sincere gratitude to **[Wes Bos](https://github.com/wesbos)** and his project **[`amaran-BLE-control`](https://github.com/wesbos/amaran-BLE-control)**. Wes deciphered and openly documented the Amaran Bluetooth Mesh protocol and packet structures. Without his pioneering reverse-engineering work, creating this native project would have been immensely harder.

---

## Why amaranctl?

If you own an Aputure Amaran light (such as the 60x, 60x S, 100d, 200x, and similar models), you probably know the friction of using the official desktop application:

* **Heavy resource footprint:** The official application is over 500 MB to download and consumes hundreds of megabytes of RAM while idle.
* **Cloud dependency:** Requires creating an account and logging in through cloud servers just to control a lamp standing on your desk.
* **Slow startup:** Takes 20 to 30 seconds to launch and connect.

**amaranctl** solves this with a clean, native approach:

* **Under 1 MB** standalone binary size (compared to 500+ MB).
* **Less than 10 MB RAM** usage in the background.
* **100% Local & Private:** Connects directly to your light via Bluetooth Low Energy (BLE Mesh). No cloud accounts, no internet connection required, zero telemetry.
* **Instant execution:** Toggle power or switch presets with zero noticeable latency.
* **Global Hotkeys:** Adjust brightness and color temperature at any time without switching away from your current game, meeting, or workspace.

---

## Quick Setup

No Python, Node.js, or compiler toolchains are required. Everything is handled automatically using tools already present in your operating system.

### How it works (The 3-Step Setup)

Amaran lights communicate over an encrypted Bluetooth connection. To send commands, `amaranctl` only needs a configuration file (`~/.config/amaranctl/config.json`) containing your local mesh key.

1. **Step 1:** Install the official **Amaran Desktop** application once, log in, and ensure your light appears in your list. (This downloads your light's credentials into your local cache).
2. **Step 2:** Run the one-line installer command below for your operating system. It parses your existing configuration or automatically extracts your keys from the local database, installs `amaranctl`, and configures autostart.
3. **Step 3:** You are all set. You can close and uninstall the official 500 MB application.

> **Configuration validation:** If a configuration file already exists at `~/.config/amaranctl/config.json`, the installer validates it. If valid, database extraction is skipped entirely. If the configuration is corrupted or invalid and a local `amaran.db` is detected, the installer offers to create a backup (`config.json.bak`) and generate a clean configuration.

---

### Installation Commands

#### Windows (PowerShell)

Open PowerShell (press `Win + X`, then select **Terminal** or **PowerShell**) and run:

```powershell
irm https://raw.githubusercontent.com/amaranctl/amaranctl/main/scripts/install.ps1 | iex
```

What this script does:
1. Checks for an existing valid configuration in `~/.config/amaranctl/config.json`.
2. If needed, reads your credentials from `amaran.db` using Windows' built-in SQLite subsystem (no Python required).
3. Installs `amaranctl.exe` to your local programs directory and adds it to your user `PATH`.
4. Prompts to enable silent background autostart on Windows login.

---

#### Linux / macOS (Bash)

Open your terminal and run:

```bash
curl -fsSL https://raw.githubusercontent.com/amaranctl/amaranctl/main/scripts/install.sh | bash
```

What this script does:
1. Validates or extracts credentials into `~/.config/amaranctl/config.json`.
2. Installs `amaranctl` into `~/.local/bin/amaranctl`.
3. On Linux, optionally sets up a lightweight `systemd --user` background service.

---

## How to Use

### 1. System Tray Mode (GUI)

Launch `amaranctl` (or double-click `amaranctl.exe`). A discreet icon appears in your system tray near the clock.

* **Tray Menu Controls:**
  * **Power:** Instant toggle on/off.
  * **Brightness Presets:** 10%, 25%, 50%, 75%, 100%.
  * **Color Temperature Presets:** 2700K (Warm), 3200K (Tungsten), 4500K (Neutral), 5600K (Daylight), 6500K (Cool White).
  * **Start on boot:** Toggle silent autostart on system login.
  * **Quit:** Exit background process.

* **Global Keyboard Shortcuts (active system-wide):**
  | Shortcut | Action |
  | :--- | :--- |
  | `Ctrl + Alt + L` | **Toggle** Light On / Off |
  | `Ctrl + Alt + Up` | **Increase Brightness** by +10% |
  | `Ctrl + Alt + Down` | **Decrease Brightness** by -10% |
  | `Ctrl + Alt + Right` | **Increase Temperature** by +200K (Cooler) |
  | `Ctrl + Alt + Left` | **Decrease Temperature** by -200K (Warmer) |

---

### 2. Terminal Mode (Automation & Streamers)

You can call `amaranctl` directly from any command line prompt, PowerShell, or shell script.

#### Commands Reference
```bash
# Power control
amaranctl on            # Turn light on
amaranctl off           # Turn light off
amaranctl toggle        # Toggle power state

# Brightness control (0 to 100%)
amaranctl brightness 50   # Set brightness to 50%
amaranctl brightness 100  # Set full brightness

# Color temperature (2700K to 6500K)
amaranctl cct 3200        # Warm studio light (3200K)
amaranctl cct 5600        # Clean daylight (5600K)
```

> **Note on precision:** You can also pass raw values between `101` and `1000` (e.g., `amaranctl brightness 555` sets 55.5% output with 0.1% hardware precision).

---

### Integration Examples

Because `amaranctl` is a fast standalone CLI tool, you can integrate it into various automation workflows:

* **OBS Studio (Scene Automation):**  
  Using OBS plugins such as *Advanced Scene Switcher*, trigger lighting adjustments automatically when changing scenes:
  * When switching to a **Camera / Talking Head** scene: trigger `amaranctl on` and `amaranctl cct 5600`.
  * When switching to a **Gaming / Screen Share** scene: trigger `amaranctl brightness 20` or `amaranctl off`.
* **Stream Deck & Macro Pads:**  
  Assign a key to execute `amaranctl toggle` to control your light with a single physical button press.
* **Scheduled Tasks:**  
  Use Windows Task Scheduler or cron to automatically turn on your desk light at the start of your workday and turn it off in the evening.

---

## Troubleshooting & FAQ

* **Light does not respond:**
  1. **Ensure the official Amaran desktop application is completely closed.** Bluetooth Low Energy allows only one active connection at a time. If the official application is running in the background, it locks the Bluetooth device.
  2. Verify that your computer's Bluetooth is enabled.
  3. Ensure the light fixture is powered on and within Bluetooth range (~10 meters / 30 feet).
* **Configuration path:**  
  `amaranctl` looks for `config.json` in the following order:
  1. File specified via `--config <path>`.
  2. Current working directory (`./config.json`).
  3. Next to the binary executable.
  4. User home directory (`~/.config/amaranctl/config.json`).
  5. System roaming app data directory (`%APPDATA%\amaranctl\config.json`).

---

## For Developers: Building from Source

To compile `amaranctl` yourself without installing Rust or native development headers on your host machine, build with Docker or Podman in two commands:

```bash
# 1. Build the compiler container image
docker build -t amaran-builder -f build/Containerfile .

# 2. Compile amaranctl into your project directory
docker run --rm -v $(pwd):/workspace amaran-builder cargo build --release
```

The compiled binary will be placed in `target/release/`.

---

## License

MIT License. Designed for creators and streamers who want fast, reliable, local lighting control.
