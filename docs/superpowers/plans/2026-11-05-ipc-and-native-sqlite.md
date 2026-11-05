# IPC Client-Daemon and Native SQLite Dynamic Loading Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Shrink `amaranctl` back under 1 MB by replacing bundled `rusqlite` with native dynamic SQLite loading (`winsqlite3.dll` / `libsqlite3`), and implement a high-speed local IPC architecture (Named Pipe on Windows / Unix Domain Socket on Linux/macOS) so terminal commands execute in under 50 ms against the running tray daemon with correct synchronous terminal waiting.

**Architecture:** 
1. Remove `rusqlite` dependency. Implement zero-dependency dynamic C SQLite loader (`src/sqlite_dyn.rs`) leveraging OS libraries (`winsqlite3.dll` on Windows, `libsqlite3` on Linux/macOS) only invoked when `config.json` is missing/invalid to auto-extract credentials from `amaran.db`.
2. Add IPC module (`src/ipc.rs`): server in tray background daemon listening on `\\.\pipe\amaranctl` (Windows) / `~/.config/amaranctl/amaranctl.sock` (Unix), and client in CLI entrypoint that dispatches commands to the running daemon in ~30 ms, with fallback to direct BLE connection if daemon is not running.
3. Switch Windows subsystem to standard console so terminals (PowerShell, CMD, Clink) wait synchronously for command completion without prematurely returning the prompt `λ`, while `run_tray_app()` calls `FreeConsole()` to detach silently in tray mode.
4. Simplify `install.ps1` and `install.sh` by removing all database parsing logic, leaving clean 10-line binary installers.

**Tech Stack:** Rust (2021), Tokio (NamedPipeServer, NamedPipeClient, UnixListener, UnixStream), Win32 API (`winsqlite3.dll`, `FreeConsole`), `libc` (`dlopen`, `dlsym`).

**Spec:** User conversation requirements from 2026-11-05 (sub-1MB single binary, native OS SQLite loading, IPC daemon for instant CLI commands, synchronous console waiting).

## Global Constraints
- Single binary output named `amaranctl` (`amaranctl.exe` on Windows).
- Executable size must remain under 1.2 MB (ideally ~1 MB, eliminating rusqlite C bundle).
- Zero Rust installation on Windows host (cross-compile via Podman `amaran-builder` in WSL Debian).
- All git commits must use `--no-gpg-sign`.
- No new external crate dependencies for SQLite; use native dynamic loading.

## Review Focus
1. Named pipe / socket collision or stale sockets when daemon crashes or restarts.
2. CLI timeout when daemon pipe is unresponsive (must fail quickly and fallback or report error).
3. Dynamic SQLite loader handles missing system library gracefully without crashing the binary.
4. Windows `FreeConsole()` behavior when launched from existing console vs Explorer double-click vs Windows startup.
5. Concurrent CLI invocations while daemon is executing a BLE command.

---

### Task 1: Zero-Dependency Dynamic SQLite Loading (`src/sqlite_dyn.rs`)

**Files:**
- Create: `src/sqlite_dyn.rs`
- Modify: `Cargo.toml:17-20` (remove `rusqlite`)
- Modify: `src/config.rs:50-160` (replace `rusqlite` with `sqlite_dyn`)
- Test: `tests/test_sqlite_dyn.rs`

**Interfaces:**
- Produces: `pub fn extract_credentials_from_db(db_path: &Path) -> Result<AppConfig>`
- Consumes: `AppConfig`, `LightConfig` from `src/config.rs`

- [ ] **Step 1: Write unit test for dynamic SQLite extraction**

```rust
// tests/test_sqlite_dyn.rs
use amaranctl::config::AppConfig;
use std::path::Path;

#[test]
fn test_dynamic_sqlite_not_crashing_on_invalid_db() {
    let result = amaranctl::sqlite_dyn::extract_credentials_from_db(Path::new("non_existent.db"));
    assert!(result.is_err());
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `wsl -d Debian podman run --rm -v /mnt/c/Users/PerVenT/.gemini/antigravity/scratch/amaran-control:/workspace:z localhost/amaran-builder cargo test --test test_sqlite_dyn`
Expected: FAIL (module `sqlite_dyn` does not exist).

- [ ] **Step 3: Implement `src/sqlite_dyn.rs` using OS dynamic loading**

Implement `extract_credentials_from_db` loading:
- Windows: `winsqlite3.dll` via `LoadLibraryA` and `GetProcAddress`.
- Linux / macOS: `libsqlite3.so.0` / `libsqlite3.dylib` via `libc::dlopen` and `libc::dlsym`.
Declare function pointer types:
`sqlite3_open`, `sqlite3_close`, `sqlite3_prepare_v2`, `sqlite3_step`, `sqlite3_column_text`, `sqlite3_column_int`, `sqlite3_finalize`.
Query `mesh` and `fixtures` tables, populate `AppConfig`.

- [ ] **Step 4: Remove `rusqlite` from `Cargo.toml` and update `src/config.rs`**

Remove `rusqlite = { version = "0.31", features = ["bundled"] }`.
Point `AppConfig::try_auto_import_from_db()` to `sqlite_dyn::extract_credentials_from_db`.

- [ ] **Step 5: Run tests to verify pass**

Run: `wsl -d Debian podman run --rm -v /mnt/c/Users/PerVenT/.gemini/antigravity/scratch/amaran-control:/workspace:z localhost/amaran-builder cargo test`
Expected: PASS (all tests green).

- [ ] **Step 6: Commit changes**

`git commit --no-gpg-sign -m "feat: replace bundled rusqlite with zero-dependency dynamic OS SQLite loader"`

---

### Task 2: IPC Module for Client-Daemon Communication (`src/ipc.rs`)

**Files:**
- Create: `src/ipc.rs`
- Modify: `src/lib.rs` (export `pub mod ipc;`)
- Test: `tests/test_ipc.rs`

**Interfaces:**
- Produces:
  - `pub enum IpcCommand { On, Off, Toggle, Brightness(f32), Cct(u16), Ping }`
  - `pub struct IpcResponse { pub ok: bool, pub message: Option<String> }`
  - `pub async fn send_ipc_command(cmd: &IpcCommand) -> Result<IpcResponse>`
  - `pub struct IpcServer;`
  - `impl IpcServer { pub async fn run<F>(handler: F) -> Result<()> where F: Fn(IpcCommand) -> ... }`

- [ ] **Step 1: Write serialization & command tests for IPC protocol**

```rust
// tests/test_ipc.rs
use amaranctl::ipc::{IpcCommand, IpcResponse};

#[test]
fn test_ipc_command_json_roundtrip() {
    let cmd = IpcCommand::Brightness(75.5);
    let s = serde_json::to_string(&cmd).unwrap();
    let parsed: IpcCommand = serde_json::from_str(&s).unwrap();
    match parsed {
        IpcCommand::Brightness(v) => assert!((v - 75.5).abs() < 0.001),
        _ => panic!("wrong variant"),
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `wsl -d Debian podman run --rm -v /mnt/c/Users/PerVenT/.gemini/antigravity/scratch/amaran-control:/workspace:z localhost/amaran-builder cargo test --test test_ipc`
Expected: FAIL (`ipc` module not found).

- [ ] **Step 3: Implement `src/ipc.rs`**

- On Windows: Use `tokio::net::windows::named_pipe::{ClientOptions, ServerOptions}` on pipe `r"\\.\pipe\amaranctl"`.
- On Linux/macOS: Use `tokio::net::{UnixListener, UnixStream}` on `$HOME/.config/amaranctl/amaranctl.sock`.
- Client `send_ipc_command(cmd)` with 100ms connection timeout: sends JSON command line, reads response line.
- Server loop: accepts connections, reads command, dispatches to handler, writes response.

- [ ] **Step 4: Run tests to verify pass**

Run: `wsl -d Debian podman run --rm -v /mnt/c/Users/PerVenT/.gemini/antigravity/scratch/amaran-control:/workspace:z localhost/amaran-builder cargo test --test test_ipc`
Expected: PASS.

- [ ] **Step 5: Commit changes**

`git commit --no-gpg-sign -m "feat: add cross-platform IPC client and server module"`

---

### Task 3: Integrate IPC into Tray Daemon and CLI Entrypoint

**Files:**
- Modify: `src/tray.rs` (start IPC server loop handling client commands)
- Modify: `src/main.rs` (CLI commands try IPC first, fallback to BLE; handle console subsystem & FreeConsole)

**Interfaces:**
- Consumes: `src/ipc.rs` (`send_ipc_command`, `IpcServer`, `IpcCommand`), `src/tray.rs` (`AppAction`)

- [ ] **Step 1: Update `src/tray.rs` to run `IpcServer` concurrently with Tray UI**

In `run_tray_app(config)`:
- Spawn background async task running `IpcServer`.
- Translate `IpcCommand` to `AppAction` and send via `tx` channel to the BLE worker thread.
- On Windows: At the entry of `run_tray_app()`, invoke `FreeConsole()` to cleanly detach from terminal when starting tray mode.

- [ ] **Step 2: Update `src/main.rs` for synchronous CLI and IPC client dispatch**

- Remove `#![windows_subsystem = "windows"]` so `amaranctl.exe` is a standard console application on Windows.
- In `main()` for commands `On`, `Off`, `Toggle`, `Brightness`, `Cct`:
  - Check `send_ipc_command(&cmd).await`.
  - If `Ok(res)`: print status and exit immediately (<50ms)!
  - If `Err(_)` (daemon not running): print `[info] Background daemon not running, connecting via BLE...` and proceed to direct BLE execution as fallback.
- In `main()` for `None` or `Some(Commands::Run)`:
  - Invoke `run_tray_app(config)`.

- [ ] **Step 3: Test compilation for Windows and Linux in Podman**

Run: `wsl -d Debian bash /mnt/c/Users/PerVenT/.gemini/antigravity/scratch/amaran-control/build/build-windows.sh`
Run: `wsl -d Debian bash /mnt/c/Users/PerVenT/.gemini/antigravity/scratch/amaran-control/build/build-linux.sh`
Expected: Both targets build release binaries with zero errors and zero warnings.

- [ ] **Step 4: Verify binary size under 1 MB**

Check `amaranctl.exe` file size: must be ~950 KB - 1050 KB (down from 1.98 MB).

- [ ] **Step 5: Commit changes**

`git commit --no-gpg-sign -m "feat: integrate IPC into daemon and CLI with instant execution and console wait"`

---

### Task 4: Simplify Installer Scripts and Verify Live Hardware Integration

**Files:**
- Modify: `scripts/install.ps1`
- Modify: `scripts/install.sh`
- Modify: `README.md`

- [ ] **Step 1: Simplify `scripts/install.ps1`**

Remove all SQLite parsing logic from `install.ps1`:
- Download latest `amaranctl.exe` (or copy local if in repo).
- Place into `%LOCALAPPDATA%\Programs\amaranctl\amaranctl.exe` and `~/.local/bin/amaranctl.exe`.
- Add to user `PATH`.
- Prompt for autostart (`HKCU\...\Run`).

- [ ] **Step 2: Simplify `scripts/install.sh`**

Remove all Python/sqlite3 extraction logic from `install.sh`:
- Download Linux/macOS binary to `~/.local/bin/amaranctl`.
- Ensure executable permissions.
- Prompt for systemd autostart service on Linux.

- [ ] **Step 3: Update `README.md`**

Reflect that `amaranctl` has built-in auto-discovery of `amaran.db` via native OS libraries and instant IPC (<50ms) communication between CLI and background daemon.

- [ ] **Step 4: Live hardware verification**

- Test 1: Run `amaranctl brightness 50` in terminal while tray app is running -> verify instant execution (<50ms), no BLE scanning delay, prompt `λ` appears only after completion.
- Test 2: Kill tray app, run `amaranctl toggle` -> verify direct BLE fallback.
- Test 3: Delete `~/.config/amaranctl/config.json`, run `amaranctl on` -> verify dynamic SQLite extraction auto-creates `config.json` and turns on light.

- [ ] **Step 5: Commit changes**

`git commit --no-gpg-sign -m "docs: simplify installer scripts and document IPC instant execution"`
