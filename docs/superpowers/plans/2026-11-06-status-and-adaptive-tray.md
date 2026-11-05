# Live Status Reporting, Adaptive Tray Icon & amaranctl status Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement bi-directional Bluetooth Mesh communication for `amaranctl` to capture live lamp status, display an adaptive tray icon and tooltip reflecting connection/power/CCT state, support the `amaranctl status [--json]` command, and enable relative adjustments (`+`/`-`).

**Architecture:** Inbound BLE notifications on characteristic `0x2A9E` are decrypted via AES-CCM (Network + Access layers) and decoded into `LampStatus`. The background daemon polls status every 2 seconds, maintaining an `Arc<RwLock<LampStatus>>` cache that renders dynamic 32x32 RGBA tray icons (CCT tinted when ON, pulsing when connecting, gray when OFF, red dot when disconnected) and services CLI `status` requests via IPC in <20 ms.

**Tech Stack:** Rust (2021), AES-CCM (`ccm`, `aes`), Tokio, `tray-icon`, Windows API (`CREATE_NO_WINDOW`, `DETACHED_PROCESS`).

**Spec:** `docs/superpowers/specs/2026-11-06-status-and-adaptive-tray-design.md`

## Global Constraints
- Single binary output named `amaranctl` (`amaranctl.exe` on Windows).
- Maintain binary size strictly under 1.2 MB (currently 1.05 MB).
- Cross-compile via Podman container `localhost/amaran-builder` without host Rust.
- All git commits must use `--no-gpg-sign`.

## Review Focus
1. Inbound packet MIC verification failure handling: corrupt or duplicate packets must not panic or desync the sequence table.
2. Disconnection resilience: if the lamp is powered off from the wall, status transitions cleanly to Disconnected (red dot icon) within 5 seconds without freezing the IPC server.
3. Rapid clicking / Debounce: user clicking the tray icon or running CLI commands while Bluetooth is connecting must queue the last desired state without spawning parallel BLE sessions.
4. Relative adjustments edge cases: `brightness +10` at 95% clamps cleanly to 100%; `cct -500` at 2800K clamps cleanly to 2700K.
5. IPC latency: `amaranctl status` must respond in under 30 ms when the background daemon is running.

---

### Task 1: Inbound Mesh Decryption & Status Decoder (`src/mesh.rs`)

**Files:**
- Modify: `src/mesh.rs`
- Test: `tests/test_mesh.rs`

**Interfaces:**
- Produces: `pub fn aes_ccm_decrypt(key: &[u8; 16], nonce: &[u8; 13], ciphertext: &[u8]) -> Result<Vec<u8>>`
- Produces: `pub fn decrypt_proxy_pdu(ctx: &MeshSecurityContext, iv_index: u32, pdu: &[u8]) -> Result<(u16, u16, Vec<u8>)>`
- Produces: `pub fn decode_telink_status(payload: &[u8]) -> Option<LampStatusData>`
- Produces: `pub struct LampStatusData { pub power: bool, pub brightness: f32, pub cct: u16, pub gm: i8 }`

- [ ] **Step 1: Write unit tests for inbound decryption and status decoding**

```rust
// tests/test_mesh.rs
#[test]
fn test_mesh_encryption_decryption_roundtrip() {
    let ctx = MeshSecurityContext::new(&[1u8; 16], &[2u8; 16]).unwrap();
    let pdu = build_proxy_pdu(&ctx, 100, 0x0001, 0x0002, 0, 0x8202, &[0xAA, 0xBB]).unwrap();
    let (src, dst, decrypted) = decrypt_proxy_pdu(&ctx, 0, &pdu).unwrap();
    assert_eq!(src, 0x0001);
    assert_eq!(dst, 0x0002);
    assert_eq!(&decrypted[..2], &opcode_bytes(0x8202)[..]);
    assert_eq!(&decrypted[2..], &[0xAA, 0xBB]);
}

#[test]
fn test_decode_telink_status_cct_packet() {
    // Valid 10-byte CCT status packet
    let mut packet = [0u8; 10];
    packet[9] = 0x02; // CCT command type
    // power on (bit 8)
    packet[1] = 0x01;
    // checksum
    let mut sum: u16 = 0;
    for i in 1..10 { sum += packet[i] as u16; }
    packet[0] = (sum & 0xff) as u8;

    let decoded = decode_telink_status(&packet).expect("valid decode");
    assert!(decoded.power);
}
```

- [ ] **Step 2: Run test to verify it fails (RED)**

Run: `wsl -d Debian podman run --rm -v /mnt/c/Users/PerVenT/.gemini/antigravity/scratch/amaran-control:/workspace:z localhost/amaran-builder cargo test --test test_mesh`
Expected: FAIL (`decrypt_proxy_pdu` not found).

- [ ] **Step 3: Implement AES-CCM decryption, PDU de-obfuscation and Telink status decoding in `src/mesh.rs`**

- Implement `aes_ccm_decrypt`: uses `AesCcm4::new(key.into()).decrypt(nonce.into(), ciphertext)`.
- Implement `decrypt_proxy_pdu`: reverses `build_proxy_pdu` (extracts `obfuscated`, calculates `pecb`, de-obfuscates `ctl/ttl/seq/src`, decrypts Network payload with `enc_key`, extracts `lower_transport`, decrypts Access payload with `app_key`).
- Implement `decode_telink_status`: validates checksum (`p[0] == sum(p[1..9]) & 0xff`), decodes `power = (low64 >> 8) & 1`, `cct = telink_cct * 10`, `intensity = raw / 10.0`.

- [ ] **Step 4: Run test to verify it passes (GREEN)**

Run: `wsl -d Debian podman run --rm -v /mnt/c/Users/PerVenT/.gemini/antigravity/scratch/amaran-control:/workspace:z localhost/amaran-builder cargo test --test test_mesh`
Expected: PASS (all tests green).

- [ ] **Step 5: Commit changes**

`git commit --no-gpg-sign -m "feat: implement BLE Mesh inbound PDU decryption and status decoder"`

---

### Task 2: Notification Subscription & Status Poller in `BleMeshClient` (`src/ble.rs`)

**Files:**
- Modify: `src/ble.rs`
- Test: `tests/test_ble.rs`

**Interfaces:**
- Consumes: `src/mesh.rs` (`decrypt_proxy_pdu`, `decode_telink_status`, `LampStatusData`)
- Produces: `pub struct LampStatus { pub connected: bool, pub connecting: bool, pub power: bool, pub brightness: f32, pub cct: u16 }`
- Produces: `BleMeshClient::current_status(&self) -> LampStatus`
- Produces: `BleMeshClient::subscribe_notifications(&mut self) -> Result<()>`
- Produces: 2-second periodic status polling loop.

- [ ] **Step 1: Write unit test for `LampStatus` tracking in `tests/test_ble.rs`**

```rust
// tests/test_ble.rs
#[test]
fn test_lamp_status_defaults_and_formatting() {
    let s = LampStatus {
        connected: true,
        connecting: false,
        power: true,
        brightness: 75.0,
        cct: 5600,
    };
    assert_eq!(s.tooltip_text(), "Amaran 60x S — 🟢 ON | 75% | 5600K");
}
```

- [ ] **Step 2: Run test to verify it fails (RED)**

Run: `wsl -d Debian podman run --rm -v /mnt/c/Users/PerVenT/.gemini/antigravity/scratch/amaran-control:/workspace:z localhost/amaran-builder cargo test --test test_ble`
Expected: FAIL (`LampStatus` / `tooltip_text` not found).

- [ ] **Step 3: Implement notification subscription and inbound status updates in `src/ble.rs`**

- Add `LampStatus` struct with `tooltip_text()` method.
- In `BleMeshClient::ensure_connected()`:
  - Find `0x2A9E` (Proxy Data Out).
  - Call `peripheral.subscribe(&char_out).await`.
  - Listen for notifications stream, decrypt with `decrypt_proxy_pdu`, and decode with `decode_telink_status`.
  - Update `current_status: Arc<RwLock<LampStatus>>`.
- Implement `poll_status(&mut self) -> Result<()>` sending `status_request`.

- [ ] **Step 4: Run test to verify it passes (GREEN)**

Run: `wsl -d Debian podman run --rm -v /mnt/c/Users/PerVenT/.gemini/antigravity/scratch/amaran-control:/workspace:z localhost/amaran-builder cargo test --test test_ble`
Expected: PASS.

- [ ] **Step 5: Commit changes**

`git commit --no-gpg-sign -m "feat: add notification subscriber and status poller in BleMeshClient"`

---

### Task 3: Adaptive System Tray Icon & Dynamic Tooltip (`src/tray.rs`)

**Files:**
- Modify: `src/tray.rs`

**Interfaces:**
- Consumes: `BleMeshClient` / `LampStatus`
- Produces: `pub fn create_adaptive_icon(status: &LampStatus, frame: u32) -> Icon`
- Produces: 400ms animation timer for connecting state.
- Produces: 2s status polling loop.

- [ ] **Step 1: Write helper function & test for CCT color interpolation**

- When `status.connecting`: alternate between gray and white ring.
- When `!status.connected`: gray bulb with red status dot.
- When `status.connected && !status.power`: dim gray bulb (`#444444`).
- When `status.connected && status.power`:
  - 2700K: RGB `(255, 170, 51)`
  - 4500K: RGB `(255, 244, 224)`
  - 6500K: RGB `(204, 230, 255)`
  - Linearly interpolate between color stops based on `status.cct`.

- [ ] **Step 2: Update `run_tray_app` in `src/tray.rs`**

- Replace static icon creation with reactive updater channel.
- Every 2 seconds, send `AppAction::PollStatus` to BLE worker.
- When `StatusUpdate(status)` is received:
  - Update `tray_icon.set_icon(create_adaptive_icon(&status, frame))`.
  - Update `tray_icon.set_tooltip(status.tooltip_text())`.
- Add click debounce: if already connecting, queue single action instead of duplicate connects.

- [ ] **Step 3: Run cargo test to verify no regressions**

Run: `wsl -d Debian podman run --rm -v /mnt/c/Users/PerVenT/.gemini/antigravity/scratch/amaran-control:/workspace:z localhost/amaran-builder cargo test`
Expected: PASS.

- [ ] **Step 4: Commit changes**

`git commit --no-gpg-sign -m "feat: implement adaptive CCT tray icon and live connection tooltip"`

---

### Task 4: IPC Status Command & CLI Subcommands (`src/ipc.rs`, `src/cli.rs`, `src/main.rs`)

**Files:**
- Modify: `src/ipc.rs`
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Test: `tests/test_cli.rs`

**Interfaces:**
- Produces: `IpcCommand::Status`
- Produces: `Commands::Status { json: bool }`
- Produces: Relative adjustment parsing in `Commands::Brightness` and `Commands::Cct`.

- [ ] **Step 1: Write unit tests for status CLI command and relative adjustments in `tests/test_cli.rs`**

```rust
// tests/test_cli.rs
#[test]
fn test_cli_status_command() {
    let args = vec!["amaranctl", "status"];
    let cli = Cli::parse_from(args);
    assert_eq!(cli.command, Some(Commands::Status { json: false }));

    let args = vec!["amaranctl", "status", "--json"];
    let cli = Cli::parse_from(args);
    assert_eq!(cli.command, Some(Commands::Status { json: true }));
}
```

- [ ] **Step 2: Run test to verify it fails (RED)**

Run: `wsl -d Debian podman run --rm -v /mnt/c/Users/PerVenT/.gemini/antigravity/scratch/amaran-control:/workspace:z localhost/amaran-builder cargo test --test test_cli`
Expected: FAIL (`Commands::Status` not found).

- [ ] **Step 3: Implement `IpcCommand::Status`, `Commands::Status`, and relative brightness/CCT in CLI**

- Add `IpcCommand::Status` to `src/ipc.rs`.
- In `src/tray.rs`: IPC server handler for `IpcCommand::Status` returns current `LampStatus` as JSON.
- Add `Commands::Status { #[arg(long)] json: bool }` to `src/cli.rs`.
- In `src/main.rs`:
  - When `Commands::Status` is run: query IPC. If JSON requested, print raw JSON. If text, print human-readable summary.
  - In `Commands::Brightness` and `Commands::Cct`: allow relative delta (e.g. string with `+` or `-`), querying current status to apply delta.

- [ ] **Step 4: Run test to verify it passes (GREEN)**

Run: `wsl -d Debian podman run --rm -v /mnt/c/Users/PerVenT/.gemini/antigravity/scratch/amaran-control:/workspace:z localhost/amaran-builder cargo test`
Expected: PASS (all tests pass).

- [ ] **Step 5: Commit changes**

`git commit --no-gpg-sign -m "feat: add amaranctl status command and relative adjustments"`

---

### Task 5: Compilation, Binary Size Check & Live Hardware Verification

**Files:**
- Build outputs: `target/x86_64-pc-windows-gnu/release/amaranctl.exe`
- Install paths: `amaranctl.exe`, `~/.local/bin/amaranctl.exe`, `%LOCALAPPDATA%\Programs\amaranctl\amaranctl.exe`

- [ ] **Step 1: Cross-compile Windows release binary**

Run: `wsl -d Debian bash /mnt/c/Users/PerVenT/.gemini/antigravity/scratch/amaran-control/build/build-windows.sh`
Expected: Exit code 0, binary size < 1.2 MB.

- [ ] **Step 2: Deploy binary to system paths**

Copy to `.\amaranctl.exe`, `~/.local/bin\amaranctl.exe`, and `%LOCALAPPDATA%\Programs\amaranctl\amaranctl.exe`.

- [ ] **Step 3: Live Hardware Verification on Windows**

1. Launch background daemon: `amaranctl run`.
2. Check initial tooltip: verify `Amaran 60x S — ⏳ Пошук та підключення...` during connection.
3. Check connected tooltip: hover over tray icon, verify `Amaran 60x S — 🟢 ON | {brightness}% | {cct}K`.
4. Test CLI status:
   - `amaranctl status` -> prints `Amaran 60x S: ON | Brightness: ... | CCT: ... | Connected` in <25 ms.
   - `amaranctl status --json` -> outputs valid JSON.
5. Test relative adjustment:
   - `amaranctl brightness +10` -> brightness increases by 10%.
6. Test physical knob sync:
   - Physically rotate brightness knob on the Amaran 60x S.
   - Wait 4 seconds, run `amaranctl status`.
   - Verify CLI and tray tooltip reflect the newly rotated physical value.

- [ ] **Step 4: Commit changes & update progress ledger**

`git commit --no-gpg-sign -m "chore: verify live status reporting and adaptive tray on Windows hardware"`
