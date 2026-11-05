# Amaranctl (Rust) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a lightweight, cross-platform Rust background utility and CLI (`amaranctl`) to control an Aputure Amaran 60x S over Bluetooth Low Energy Mesh (Telink opcode `0x26`), supporting system tray, global hotkeys, and autostart on Windows and Linux with memory footprint < 10 MB.

**Architecture:** The core application features a modular design:
- `src/mesh.rs` implements Bluetooth Mesh cryptographic key derivation (K2, K4), AES-128 CCM/CMAC encryption, network/proxy PDU framing, and Telink vendor opcode `0x26` command packing (On/Off, Brightness, CCT).
- `src/ble.rs` manages BLE discovery and stateful GATT Mesh Proxy (`0x2ADD`/`0x2ADE`) communication using `btleplug`.
- `src/tray.rs` and `src/hotkeys.rs` provide Windows system tray UI and global hotkey handling.
- `src/cli.rs` exposes direct command-line arguments for quick headless control or Linux shortcut binding.
- Containerized cross-compilation is executed via Podman in WSL using a dedicated `amaran-builder` image without touching existing images.

**Tech Stack:** Rust 1.85+, `btleplug`, `aes`, `cmac`, `ccm`, `tokio`, `tray-icon`, `muda`, `global-hotkey`, `clap`, `serde`, `serde_json`, Podman (Debian WSL), MinGW-w64 (`x86_64-pc-windows-gnu`).

**Spec:** In-conversation requirements:
- Device: Aputure Amaran 60x S (`70:3E:97:FD:81:D5`, node address `2`)
- Credentials: NetKey `58015449C4CEDAA2B9E1FA1BB580973F`, AppKey `FA1BB580973F58015449C4CEDAA2B9E1`
- Functions: Power toggle/on/off, Brightness (0–100%), CCT (2700K–6500K)
- UX: System tray with presets/slider, background process, autostart, global hotkeys, minimal RAM (< 10 MB)
- Platform: Windows 10/11 native `.exe`, Linux compatibility with `systemd --user`

## Global Constraints

- Do not install Rust directly on Windows host; perform compilation inside WSL Debian using a dedicated Podman container.
- Do not modify or delete existing Podman images (e.g. `localhost/its-fw-builder`).
- Windows autostart must use `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` (User Session, not Windows Service, preserving Session 0 UI and hotkeys).
- Linux autostart provides a `systemd --user` service unit.
- Preserve comments, reverse-engineered Telink formulas, and mathematical exactness for byte packing.

## Review Focus

1. Packet sequence numbers (`seq`): Must increment monotonically for each outgoing packet from source address `0x7001` to prevent fixture from silently dropping packets.
2. Telink CCT formula: Kelvin must be scaled as `((k + 5) / 10)` clamped to `[80, 2000]`, with proper GM tint offset `10`.
3. Brightness scaling: User 0–100% mapped to hardware 0–1000 with lower 2 bits in byte 7 and upper 8 bits in byte 8.
4. Proxy PDU segmentation: Single-segment Access PDU with length <= 15 bytes fits directly in unsegmented network PDU.
5. Windows Tray and Hotkey message loop: Must run on main thread without blocking async Tokio BLE runtime.

---

### Task 1: Dedicated Build Container for WSL Podman

**Files:**
- Create: `build/Containerfile`
- Create: `build/build-windows.sh`
- Create: `build/build-linux.sh`

**Interfaces:**
- Produces: Podman image `amaran-builder:latest` capable of compiling for `x86_64-pc-windows-gnu` and `x86_64-unknown-linux-gnu`.

- [ ] **Step 1: Write `build/Containerfile`**
Base on `docker.io/library/rust:latest`, install `mingw-w64`, add target `x86_64-pc-windows-gnu`.

- [ ] **Step 2: Write build scripts**
Create `build-windows.sh` and `build-linux.sh` running `podman run --rm -v ... amaran-builder cargo build --release`.

- [ ] **Step 3: Build container image in WSL**
Run: `wsl -d Debian podman build -t amaran-builder -f /mnt/c/Users/PerVenT/.gemini/antigravity/scratch/amaran-control/build/Containerfile /mnt/c/Users/PerVenT/.gemini/antigravity/scratch/amaran-control/build`
Expected: `amaran-builder:latest` successfully created.

- [ ] **Step 4: Commit**
```bash
git add build/
git commit -m "chore: add podman containerfile for windows cross-compilation"
```

---

### Task 2: Project Scaffolding and Configuration Loader

**Files:**
- Create: `Cargo.toml`
- Create: `src/config.rs`
- Create: `tests/test_config.rs`

**Interfaces:**
- Produces: `struct AppConfig` loading `config.json` with `net_key`, `app_key`, `mac`, and `address`.

- [ ] **Step 1: Write failing test for config loader**
In `tests/test_config.rs`, test parsing `config.json` fixture.

- [ ] **Step 2: Run test to verify it fails**
Run build in podman: `cargo test --test test_config`
Expected: FAIL (missing `Cargo.toml` / `config.rs`).

- [ ] **Step 3: Implement `Cargo.toml` and `src/config.rs`**
Add dependencies: `serde`, `serde_json`, `hex`. Implement `AppConfig::load_from_path(path: &Path) -> Result<AppConfig>`.

- [ ] **Step 4: Run test to verify it passes**
Run: `cargo test --test test_config`
Expected: PASS.

- [ ] **Step 5: Commit**
```bash
git add Cargo.toml src/config.rs tests/test_config.rs
git commit -m "feat: add cargo project scaffolding and config loader"
```

---

### Task 3: Telink Mesh Protocol & Cryptography

**Files:**
- Create: `src/mesh.rs`
- Create: `tests/test_mesh.rs`

**Interfaces:**
- Consumes: `AppConfig` (`net_key: [u8; 16]`, `app_key: [u8; 16]`, `dst_address: u16`).
- Produces: 
  - `encode_onoff(on: bool) -> [u8; 10]`
  - `encode_brightness(brightness_0_1000: u16) -> [u8; 10]`
  - `encode_cct(kelvin: u16, brightness_0_1000: u16, gm_offset: i8) -> [u8; 10]`
  - `build_proxy_pdu(keys: &MeshKeys, seq: u32, src: u16, dst: u16, iv_index: u32, opcode: u8, params: &[u8]) -> Vec<u8>`

- [ ] **Step 1: Write unit tests for Telink encoding and crypto derivations**
In `tests/test_mesh.rs`, verify:
- Checksum algorithm (`sum & 0xff`).
- Power On payload equals `[0x8D, 0, 0, 0, 0, 0, 0, 0, 0x01, 0x8C]`.
- Brightness payload bit packing for 500 (50%).
- CCT payload bit packing for 4500K.
- K2 and K4 derivation against standard Bluetooth Mesh test vectors.

- [ ] **Step 2: Run tests to verify failure**
Expected: FAIL.

- [ ] **Step 3: Implement `src/mesh.rs`**
Port validated Telink encoding and AES-CMAC / AES-CCM / K2 / K4 / PECB functions using pure Rust crypto crates (`aes`, `cmac`, `ccm`).

- [ ] **Step 4: Run tests to verify passes**
Run: `cargo test --test test_mesh`
Expected: All tests PASS.

- [ ] **Step 5: Commit**
```bash
git add src/mesh.rs tests/test_mesh.rs
git commit -m "feat: implement telink opcode 0x26 encoding and bluetooth mesh cryptography"
```

---

### Task 4: BLE GATT Mesh Proxy Client

**Files:**
- Create: `src/ble.rs`

**Interfaces:**
- Consumes: `AppConfig`, `build_proxy_pdu` from `src/mesh.rs`.
- Produces: `BleMeshClient`:
  - `connect(mac: &str) -> Result<Self>`
  - `send_command(&mut self, payload_10bytes: [u8; 10]) -> Result<()>`
  - Uses Mesh Proxy In characteristic UUID `00002ADD-0000-1000-8000-00805F9B34FB`.

- [ ] **Step 1: Implement `BleMeshClient` with `btleplug`**
Scan for device by MAC address `70:3E:97:FD:81:D5`, connect, discover services, locate `2add`, write proxy PDUs with sequence number tracking.

- [ ] **Step 2: Add mock/unit tests for sequence number incrementing**
Verify sequence numbers increment monotonically and persist across calls.

- [ ] **Step 3: Commit**
```bash
git add src/ble.rs
git commit -m "feat: implement ble mesh proxy client using btleplug"
```

---

### Task 5: Command Line Interface (CLI)

**Files:**
- Create: `src/cli.rs`
- Modify: `src/main.rs`

**Interfaces:**
- Produces: CLI parser with subcommands:
  - `on`, `off`, `toggle`
  - `brightness <PERCENT>`
  - `cct <KELVIN>`

- [ ] **Step 1: Implement CLI argument parser using `clap`**
Support headless one-shot commands (`amaranctl --toggle`, `amaranctl --cct 4500`).

- [ ] **Step 2: Verify CLI dispatch**
Test parsing and dispatching in dry-run mode.

- [ ] **Step 3: Commit**
```bash
git add src/cli.rs src/main.rs
git commit -m "feat: implement command line interface"
```

---

### Task 6: System Tray UI & Global Hotkeys (Windows / Linux)

**Files:**
- Create: `src/tray.rs`
- Create: `src/hotkeys.rs`
- Modify: `src/main.rs`

**Interfaces:**
- Produces:
  - System Tray icon with menu (Power Toggle, Brightness presets 10%, 25%, 50%, 75%, 100%, CCT presets 2700K, 3200K, 4500K, 5600K, 6500K, Autostart toggle, Exit).
  - Global Hotkeys listener: `Ctrl+Alt+L` (Toggle), `Ctrl+Alt+Up` (Bright +10%), `Ctrl+Alt+Down` (Bright -10%), `Ctrl+Alt+Right` (CCT +500K), `Ctrl+Alt+Left` (CCT -500K).

- [ ] **Step 1: Implement System Tray with `tray-icon` and `muda`**
Create clean icon and menu event loop.

- [ ] **Step 2: Implement Global Hotkeys with `global-hotkey`**
Register default shortcut combinations and route events to BLE mesh client.

- [ ] **Step 3: Commit**
```bash
git add src/tray.rs src/hotkeys.rs src/main.rs
git commit -m "feat: implement system tray menu and global hotkeys"
```

---

### Task 7: Autostart Configuration (Windows Registry & Linux systemd)

**Files:**
- Create: `src/autostart.rs`
- Create: `dist/amaran.service`

**Interfaces:**
- Produces:
  - `autostart::is_enabled() -> bool`
  - `autostart::set_enabled(enable: bool) -> Result<()>`
  - Windows: Writes `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
  - Linux: Provides `amaran.service` for `systemd --user`.

- [ ] **Step 1: Implement Windows Registry autostart toggle**
Read/write registry key under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.

- [ ] **Step 2: Create Linux user systemd service template**
Write `dist/amaran.service` with `ExecStart=%h/.local/bin/amaranctl`.

- [ ] **Step 3: Commit**
```bash
git add src/autostart.rs dist/amaran.service
git commit -m "feat: add windows registry and linux systemd autostart support"
```

---

### Task 8: Cross-Compilation, Packaging, and End-to-End Verification

**Files:**
- Modify: `build/build-windows.sh`
- Modify: `build/build-linux.sh`

**Interfaces:**
- Produces:
  - `target/x86_64-pc-windows-gnu/release/amaranctl.exe` (< 10 MB)
  - `target/x86_64-unknown-linux-gnu/release/amaranctl` (< 10 MB)

- [ ] **Step 1: Compile Windows `.exe` via Podman**
Run container build targeting `x86_64-pc-windows-gnu`.
Expected: `amaranctl.exe` built successfully.

- [ ] **Step 2: Test executable on Windows host**
Run `amaranctl.exe --help` and verify prompt.

- [ ] **Step 3: Live Verification with Amaran 60x S**
Run `amaranctl.exe on` and `amaranctl.exe cct 4500` to confirm physical light response.

- [ ] **Step 4: Commit & Tag**
```bash
git commit -m "chore: release v1.0.0 windows and linux binaries"
```

