# amaranctl

> **Ultra-lightweight native controller for Aputure Amaran studio lights.**  
> Control your lighting instantly from your system tray, global hotkeys, or terminal scripts over Bluetooth Low Energy.

---

### Special Thanks & Acknowledgements

Huge thanks and heartfelt gratitude to **[Wes Bos](https://github.com/wesbos)** and his project **[`amaran-BLE-control`](https://github.com/wesbos/amaran-BLE-control)**. Wes deciphered and openly documented the Amaran Bluetooth Mesh packet structures and encryption protocol. Without his pioneering reverse-engineering work, creating this native project would have been immensely harder.

---

## Why amaranctl?

If you own an Aputure Amaran light (like the 60x, 60x S, 100d, 200x, etc.), you probably know the pain of using the official desktop app:

* 🐘 **Bloated:** The official app is over **500 MB** to download and consumes hundreds of megabytes of RAM just sitting idle.
* 🌐 **Cloud-dependent:** Requires registering an account and signing in through the cloud just to adjust a lamp on your desk.
* ⏳ **Slow:** Takes half a minute to launch and connect.

**amaranctl** replaces all of that with a clean, native tool:
* 🪶 **Under 1 MB** executable size (vs 500+ MB).
* ⚡ **Less than 10 MB RAM** usage in the background.
* 🔒 **100% Local & Private:** Talks directly to your light via Bluetooth Low Energy (BLE Mesh). No cloud accounts, no internet required, zero telemetry.
* ⏱️ **Instant Response:** Change brightness or color temperature in milliseconds.
* ⌨️ **Global Shortcuts:** Adjust lighting anytime without leaving your game or video call.

---

## 1-Minute Quick Setup

No Python, Node.js, or technical background required. Everything is handled automatically using tools already built into your operating system.

### How it works (The 3-Step Concept)
Amaran lights talk using encrypted Bluetooth. To send commands, `amaranctl` only needs a small configuration file (`config.json`) containing your room's local encryption key.

1. **Step 1:** Download and install the official **Amaran Desktop** app once, log in, and make sure your light appears in it. (This downloads your light's secret key to your computer's local database).
2. **Step 2:** Run the one-line installer command below for your operating system. It automatically finds your local database, extracts the keys, downloads `amaranctl`, and sets it up.
3. **Step 3:** That’s it! You can now close and uninstall the 500 MB official app. You won't need it ever again.

---

### Installation Commands

#### 🪟 Windows (PowerShell)
Open PowerShell (press `Win + X`, then choose **Terminal** or **PowerShell**) and paste:

```powershell
irm https://raw.githubusercontent.com/amaranctl/amaranctl/main/scripts/install.ps1 | iex
```

*What this script does:*
1. Automatically locates your light's local database (`amaran.db`) inside `%APPDATA%`.
2. Extracts your keys using Windows' built-in SQLite engine and saves them to `%APPDATA%\amaranctl\config.json`.
3. Places `amaranctl.exe` into your user programs folder and adds it to your system `PATH`.
4. Asks if you want `amaranctl` to start automatically when your computer turns on.

---

#### 🐧 Linux / 🍎 macOS (Bash)
Open your terminal and paste:

```bash
curl -fsSL https://raw.githubusercontent.com/amaranctl/amaranctl/main/scripts/install.sh | bash
```

*What this script does:*
1. Locates `amaran.db` in standard application support folders.
2. Saves your keys to `~/.config/amaranctl/config.json`.
3. Installs `amaranctl` to `~/.local/bin/amaranctl`.
4. On Linux, optionally sets up a lightweight `systemd` user service for autostart.

---

## How to Use

### 1. System Tray Mode (Simple & Visual)

Launch `amaranctl` (or double-click `amaranctl.exe`). An icon will appear in your taskbar tray near the clock.

* **Tray Menu Controls:**
  * **Turn On / Off** with a single click.
  * **Quick Brightness Presets:** 10%, 25%, 50%, 75%, 100%.
  * **Color Temperature Presets:** 2700K (Warm), 3200K (Tungsten), 4500K (Neutral), 5600K (Daylight), 6500K (Cool White).
  * **Start on boot:** Checkbox to quietly launch on Windows login.
  * **Quit:** Exit the background daemon.

* **Global Hotkeys (Work from anywhere, even inside full-screen games or apps):**
  | Shortcut | Action |
  | :--- | :--- |
  | `Ctrl + Alt + L` | **Toggle** Light On / Off |
  | `Ctrl + Alt + Up` | **Increase Brightness** by +10% |
  | `Ctrl + Alt + Down` | **Decrease Brightness** by -10% |
  | `Ctrl + Alt + Right` | **Increase Temperature** by +200K (Cooler) |
  | `Ctrl + Alt + Left` | **Decrease Temperature** by -200K (Warmer) |

---

### 2. Terminal Mode (For Automation & Streamers)

You can call `amaranctl` directly from any command prompt, PowerShell, or bash terminal.

#### Basic Commands
```bash
# Power
amaranctl on            # Turn light on
amaranctl off           # Turn light off
amaranctl toggle        # Toggle power state

# Brightness (0 to 100%)
amaranctl brightness 50   # Set to 50%
amaranctl brightness 100  # Full brightness

# Color Temperature (2700K to 6500K)
amaranctl cct 3200        # Warm studio light (3200K)
amaranctl cct 5600        # Clean daylight (5600K)
```

> **Tip:** For ultra-fine brightness control, you can pass numbers between `101` and `1000` (e.g. `amaranctl brightness 555` sets exactly 55.5% output).

---

### Creative Automation Examples

Because `amaranctl` runs as a fast command-line tool, you can easily integrate it into your streaming setup and daily workflows:

* 🎥 **OBS Studio (Scene Automation):**  
  Using free OBS plugins like *Advanced Scene Switcher*, you can automatically trigger lighting presets:
  * When switching to your **"Talking Head"** camera scene: run `amaranctl on` and `amaranctl cct 5600`.
  * When switching to a **"Gaming / Movie"** scene: run `amaranctl brightness 15` or `amaranctl off`.
* 🎛️ **Stream Deck / Macro Keys:**  
  Add a button with the "System > Open / Run" action pointing to `amaranctl toggle` to turn your studio lights on and off with a physical key press.
* 🌅 **Workday Routines:**  
  Set up Windows Task Scheduler or cron to automatically turn your desk light on at 9:00 AM and dim it down in the evening.

---

## Troubleshooting & Tips

* **The light is not responding:**
  1. **Make sure the official Amaran app is completely closed.** Bluetooth only allows one connection at a time. If the official app is open in the background, it locks the Bluetooth connection.
  2. Ensure your computer's Bluetooth is turned on.
  3. Ensure the light is plugged in and within Bluetooth range (~10 meters / 30 feet).
* **Where is `config.json` stored?**
  * **Windows:** `%APPDATA%\amaranctl\config.json`
  * **Linux / macOS:** `~/.config/amaranctl/config.json`
  * Or directly in the same folder as `amaranctl.exe`.

---

## For Developers: Building from Source

If you want to compile `amaranctl` yourself without installing Rust on your host machine, you can build it in 2 simple commands using Docker or Podman:

```bash
# 1. Build the lightweight compiler container
docker build -t amaran-builder -f build/Containerfile .

# 2. Compile amaranctl directly into your current folder
docker run --rm -v $(pwd):/workspace amaran-builder cargo build --release
```

The compiled binary will be placed right in your `target/release/` directory.

---

## License

MIT License. Crafted for creators and streamers who want fast, reliable lighting control.
