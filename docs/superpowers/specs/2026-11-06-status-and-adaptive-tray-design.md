# Specification: Live Status Reporting, Adaptive Tray Icon & amaranctl status

**Author:** Agent & User  
**Date:** 2026-11-06  
**Status:** Draft (Awaiting Approval)  
**Branch:** `feature/daemon-fork` (or `feature/status-adaptive-tray`)

---

## 1. Intent and Why

Currently, `amaranctl` operates primarily in TX-only mode: commands are sent to the light blindly without knowing its actual state, and the tray icon remains a static yellow bulb regardless of whether the light is on, off, or disconnected. Furthermore, users cannot tell if a connection is currently being established, leading to repeated attempts ("clicking 10 times blindly").

This feature implements:
1. **Bi-directional BLE Mesh communication (RX Decryption):** Decrypting inbound BLE GATT notifications from the Amaran light to capture its real-time state (`power`, `brightness`, `cct`, `gm_tint`).
2. **Adaptive System Tray Icon & Tooltip:** Visually communicating connection progress (pulsating search), actual power state, CCT color tint, and connection loss.
3. **`amaranctl status` CLI Command:** Providing instant (<20ms) status output via IPC (with `--json` support for scripts and automation) and direct BLE fallback.
4. **Relative Adjustments:** Enabling commands like `amaranctl brightness +10`, `-15`, `amaranctl cct +500`, `-500` based on live lamp status.
5. **Connection Debounce & Feedback:** Preventing duplicate/overlapping connection attempts when toggled during connection setup.

---

## 2. Requirements & Scope

### Goals
- **G1 (Inbound Decryption):** Implement Bluetooth Mesh Network & Access layer AES-CCM decryption for incoming Proxy Data Out PDUs (`0x2A9E`) using existing `NetKey` and `AppKey`.
- **G2 (Telink Status Decoding):** Decode the 10-byte Telink payload (command type `0x02` for CCT) into `LampState`:
  - `power: bool` (ON/OFF)
  - `brightness: f32` (0.0% .. 100.0%, from raw 0..1000)
  - `cct: u16` (2700K .. 6500K)
  - `connected: bool`
- **G3 (Periodic Background Polling):** Background worker polls `status_request` (`0x0E`) every 4 seconds to sync physical knob tweaks and phone app adjustments.
- **G4 (Adaptive Tray Icon):**
  - **Connecting:** Pulsating animation / alternating frames (gray ↔ white) indicating active Bluetooth connection.
  - **Connected ON:** Icon tinted with CCT color (amber 2700K `#FFAA33`, neutral 4500K `#FFF4E0`, daylight 6500K `#CCE6FF`).
  - **Connected OFF:** Dim gray bulb (`#444444`).
  - **Disconnected:** Dim gray bulb with red status indicator dot (`#E74C3C`).
- **G5 (Dynamic Tooltip):**
  - Connecting: `Amaran 60x S — ⏳ Пошук та підключення...`
  - Connected ON: `Amaran 60x S — 🟢 ON | {brightness:.1}% | {cct}K`
  - Connected OFF: `Amaran 60x S — ⚪ OFF (Готовий)`
  - Disconnected: `Amaran 60x S — 🔴 Немає зв'язку (перевірте живлення лампи)`
- **G6 (`amaranctl status` CLI):**
  - Text format: `Amaran 60x S: ON | Brightness: 60.0% | CCT: 5600K | Connected`
  - JSON format (`--json`): `{"power":true,"brightness":60.0,"cct":5600,"connected":true}`
  - IPC latency: <20 ms via background daemon.
- **G7 (Relative Steps):** Parse leading `+` or `-` in `amaranctl brightness` and `amaranctl cct`.

### Non-Goals
- Full RGB color wheel GUI (Amaran 60x S is bi-color CCT).
- Multiple simultaneous mesh light grouping (remains single target device per config).

---

## 3. Approaches Considered

### Approach 1: Integrated RX-Decoder & 4s Polling in `BleMeshClient` (Chosen)
- The background daemon's BLE client subscribes to GATT notifications on `0x2A9E`.
- Every 4 seconds (and immediately after commands), it sends a 10-byte `status_request`.
- Received packets are decrypted via AES-CCM and update `Arc<RwLock<LampState>>`.
- Tray icon and tooltip update reactively whenever state changes.
- **Tradeoffs:** Requires writing the inbound AES-CCM decryption routine for Network & Transport layers, but provides seamless, conflict-free, real-time synchronization.

### Approach 2: Passive Notification Listening Without Active Requests (Rejected)
- Listening for spontaneous notifications without sending `status_request`.
- **Tradeoffs:** Telink BLE Mesh firmware in Amaran lights only broadcasts to the mesh and does not consistently forward updates to GATT proxy clients without an explicit request. Physical knob tweaks would be missed.

### Approach 3: On-Demand Polling Only (Rejected)
- Only querying status when `amaranctl status` is invoked.
- **Tradeoffs:** The tray icon would remain static, defeating the primary visual feedback requirement.

---

## 4. Technical Architecture

### 4.1 Inbound Mesh Decryption (`src/mesh.rs`)
1. Inbound GATT notification arrives from characteristic `0x2A9E` (Proxy PDU).
2. Validate Proxy PDU Type (`0x00` = Network PDU).
3. Network Layer Decryption:
   - Use `privKey` to de-obfuscate CTL, TTL, Sequence Number, and Source Address.
   - Use `encKey` with AES-CCM to decrypt Network Payload and verify 32-bit MIC.
4. Transport/Access Layer Decryption:
   - Use `AppKey` with AES-CCM to decrypt Access Payload and verify 32-bit TransMIC.
5. Extract 10-byte Telink packet and decode:
   ```rust
   let on = (low64 >> 8) & 0x01 == 1;
   let cct_raw = ((low64 >> 52) & 0x3ff) as u16;
   let cct_flag = ((low64 >> 42) & 0x01) == 1;
   let telink_cct = if cct_flag { cct_raw + 1000 } else { cct_raw };
   let cct_kelvin = telink_cct * 10;
   let raw_intensity = (((high16 as u32) << 2) | ((low64 >> 62) & 0x03) as u32) & 0x3ff;
   let brightness_pct = (raw_intensity as f32) / 10.0;
   ```

### 4.2 State Sharing & Tray Updates (`src/tray.rs`)
- Shared state:
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct LampStatus {
      pub connected: bool,
      pub connecting: bool,
      pub power: bool,
      pub brightness: f32,
      pub cct: u16,
  }
  ```
- Tray loop receives `StatusUpdate(LampStatus)` events and:
  - Generates 32x32 RGBA icon dynamically via KaTeX/KaTeX-free math:
    - Connecting: Alternates ring opacity every 400ms.
    - Connected ON: Interpolates RGB between 2700K `(255, 170, 51)` and 6500K `(204, 230, 255)`.
    - Connected OFF: Dark gray `(60, 60, 60)` with subtle outline.
    - Disconnected: Dark gray with red dot at bottom-right `(231, 76, 60)`.
  - Calls `tray_icon.set_icon(...)` and `tray_icon.set_tooltip(...)`.

### 4.3 IPC & CLI (`src/ipc.rs`, `src/cli.rs`)
- `IpcCommand::Status`
- Response: `IpcResponse { ok: true, message: Some(json_str) }`
- CLI subcommands:
  - `amaranctl status [--json]`
  - `amaranctl brightness [+|-]<value>`
  - `amaranctl cct [+|-]<value>`

---

## 5. Global Constraints & Verification
- Binary size must stay strictly under 1.2 MB (currently 1.05 MB).
- Cross-compile via Podman container (`localhost/amaran-builder`).
- Commits must use `--no-gpg-sign`.
- Full hardware verification on Windows 11 with live Amaran 60x S.
