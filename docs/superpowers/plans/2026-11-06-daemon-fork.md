# Detached Daemon Fork Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a detached background fork mechanism for the single `amaranctl.exe` binary on Windows so that launching the tray daemon (`amaranctl run` or double-clicking) spawns an independent, windowless background worker (`CREATE_NO_WINDOW | DETACHED_PROCESS`) while CLI commands remain synchronous in the terminal with zero lingering CMD windows.

**Architecture:** 
1. When `amaranctl` is invoked to start the daemon (`amaranctl run` or default without subcommands), it first sends an IPC `Ping` to `\\.\pipe\amaranctl`. If a daemon is already active, it informs the user and exits.
2. If no daemon is running, the parent console process uses WinAPI `creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS)` to spawn a detached child copy of itself with a hidden internal flag (`--daemon-worker`).
3. The parent process prints `amaranctl background daemon started (PID: ...)` and exits immediately, cleanly returning the terminal prompt (`λ`).
4. The detached child process has no console attached, initializes the system tray icon, runs the Named Pipe IPC server, maintains the BLE connection to the light, and services hotkeys/IPC requests.
5. CLI subcommands (`on`, `off`, `brightness`, `cct`) remain standard console executions that communicate over IPC in ~15 ms and wait synchronously for completion.

**Tech Stack:** Rust (2021), Windows API (`CREATE_NO_WINDOW`, `DETACHED_PROCESS`, `CommandExt`), Tokio (NamedPipe, async), `clap` (derive).

**Spec:** User conversation requirements from 2026-11-05 (Variant 3: single binary with background fork for tray daemon, no persistent CMD window, synchronous CLI).

## Global Constraints
- Single binary output named `amaranctl` (`amaranctl.exe` on Windows).
- Maintain binary size under 1.2 MB.
- All git commits must use `--no-gpg-sign`.
- Cross-compile via Podman `amaran-builder` in WSL Debian without host Rust.

## Review Focus
1. Child worker daemon must not inherit parent console handles or die when the terminal is closed.
2. Double-fork / race conditions if `amaranctl run` is called rapidly multiple times (IPC ping guard prevents duplicate daemons).
3. Graceful termination: `Quit` in tray menu or task manager cleanly releases BLE connection and closes named pipe.
4. Windows autostart (`HKCU\...\Run`) must launch `amaranctl run` completely silently.
5. Unix fallback: works seamlessly on Linux/macOS with standard background spawn.

---

### Task 1: Add `--daemon-worker` Internal Flag and Daemon Check

**Files:**
- Modify: `src/cli.rs:1-35`
- Test: `tests/test_cli.rs`

**Interfaces:**
- Produces: `Cli::daemon_worker: bool`
- Consumes: `clap::Parser`

- [ ] **Step 1: Write unit test for CLI daemon-worker flag**

```rust
// In tests/test_cli.rs
#[test]
fn test_cli_daemon_worker_flag() {
    let args = Cli::parse_from(["amaranctl", "--daemon-worker"]);
    assert!(args.daemon_worker);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `wsl -d Debian podman run --rm -v /mnt/c/Users/PerVenT/.gemini/antigravity/scratch/amaran-control:/workspace:z localhost/amaran-builder cargo test --test test_cli`
Expected: FAIL (`field daemon_worker not found`).

- [ ] **Step 3: Update `src/cli.rs` with hidden `--daemon-worker` flag**

```rust
#[derive(Parser, Debug)]
#[command(name = "amaranctl", version, about = "Bluetooth Mesh controller for Aputure Amaran 60x S")]
pub struct Cli {
    #[arg(short, long, help = "Path to config.json")]
    pub config: Option<PathBuf>,

    #[arg(long, hide = true)]
    pub daemon_worker: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `wsl -d Debian podman run --rm -v /mnt/c/Users/PerVenT/.gemini/antigravity/scratch/amaran-control:/workspace:z localhost/amaran-builder cargo test --test test_cli`
Expected: PASS (4/4 tests green).

- [ ] **Step 5: Commit changes**

`git commit --no-gpg-sign -m "feat: add hidden --daemon-worker flag in CLI parser"`

---

### Task 2: Implement Detached Process Spawner (`src/daemon.rs`)

**Files:**
- Create: `src/daemon.rs`
- Modify: `src/lib.rs`
- Test: `tests/test_daemon.rs`

**Interfaces:**
- Produces: `pub fn spawn_detached_daemon(config_path: Option<&std::path::Path>) -> anyhow::Result<u32>`
- Produces: `pub async fn is_daemon_running() -> bool`

- [ ] **Step 1: Write unit test for daemon running check**

```rust
// tests/test_daemon.rs
use amaranctl::daemon::is_daemon_running;

#[tokio::test]
async fn test_is_daemon_running_when_not_started() {
    // When no pipe exists, is_daemon_running must return false promptly
    let running = is_daemon_running().await;
    assert!(!running);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `wsl -d Debian podman run --rm -v /mnt/c/Users/PerVenT/.gemini/antigravity/scratch/amaran-control:/workspace:z localhost/amaran-builder cargo test --test test_daemon`
Expected: FAIL (`daemon` module not found).

- [ ] **Step 3: Implement `src/daemon.rs`**

- `is_daemon_running()`: calls `send_ipc_command(&IpcCommand::Ping)` with short timeout. Returns `true` if `Ok(res)` where `res.ok == true`.
- `spawn_detached_daemon(config_path)`:
  - Finds current executable via `std::env::current_exe()`.
  - Configures `Command::new(exe)`:
    - Adds `--daemon-worker`.
    - If `config_path` provided, adds `--config <path>`.
    - Sets `stdin(Stdio::null())`, `stdout(Stdio::null())`, `stderr(Stdio::null())`.
    - On Windows: `.creation_flags(0x08000000 | 0x00000008)` (`CREATE_NO_WINDOW | DETACHED_PROCESS`).
  - Calls `.spawn()`, returns `child.id()`.

- [ ] **Step 4: Run test to verify it passes**

Run: `wsl -d Debian podman run --rm -v /mnt/c/Users/PerVenT/.gemini/antigravity/scratch/amaran-control:/workspace:z localhost/amaran-builder cargo test --test test_daemon`
Expected: PASS.

- [ ] **Step 5: Commit changes**

`git commit --no-gpg-sign -m "feat: implement detached daemon spawner and IPC status checker"`

---

### Task 3: Integrate Detached Fork into `src/main.rs` & Autostart

**Files:**
- Modify: `src/main.rs`
- Modify: `src/autostart.rs`

**Interfaces:**
- Consumes: `src/daemon.rs` (`is_daemon_running`, `spawn_detached_daemon`)
- Consumes: `src/tray.rs` (`run_tray_app`)

- [ ] **Step 1: Update `src/main.rs` daemon dispatch logic**

In `main()`:
- If `args.daemon_worker`:
  - Worker mode: load config and directly invoke `run_tray_app(config)`.
- If `args.command.is_none()` or `Some(Commands::Run)`:
  - Check `is_daemon_running().await`:
    - If `true`: print `amaranctl background daemon is already running.` and exit 0.
    - If `false`: call `spawn_detached_daemon(args.config.as_deref())`.
      Print `amaranctl background daemon started (PID: {pid}).`
      Exit 0.
- If subcommands (`On`, `Off`, `Brightness`, etc.):
  - Send via IPC (instant ~15 ms).
  - If daemon not running: execute direct BLE fallback.

- [ ] **Step 2: Update `src/autostart.rs` Windows Run command**

Ensure the value written to `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` is `"{exe}" run` (or `"{exe}"`) so Windows boot invokes the launcher which detaches the silent worker and exits immediately.

- [ ] **Step 3: Run full cargo test suite in Podman**

Run: `wsl -d Debian podman run --rm -v /mnt/c/Users/PerVenT/.gemini/antigravity/scratch/amaran-control:/workspace:z localhost/amaran-builder cargo test`
Expected: All tests pass.

- [ ] **Step 4: Commit changes**

`git commit --no-gpg-sign -m "feat: integrate detached background fork for tray daemon in main and autostart"`

---

### Task 4: Compilation, Binary Size Check, and Live Hardware Verification

**Files:**
- Build outputs: `target/x86_64-pc-windows-gnu/release/amaranctl.exe`
- Install paths: `amaranctl.exe`, `~/.local/bin/amaranctl.exe`, `%LOCALAPPDATA%\Programs\amaranctl\amaranctl.exe`

- [ ] **Step 1: Cross-compile Windows release binary**

Run: `wsl -d Debian bash /mnt/c/Users/PerVenT/.gemini/antigravity/scratch/amaran-control/build/build-windows.sh`
Expected: Exits with code 0. Binary size ~1.04 MB.

- [ ] **Step 2: Deploy binary to root and user paths**

Copy binary to `.\amaranctl.exe`, `~/.local/bin\amaranctl.exe`, and `%LOCALAPPDATA%\Programs\amaranctl\amaranctl.exe`.

- [ ] **Step 3: Live Hardware Verification**

- Test 1: In terminal, run `amaranctl run`.
  Verify: Prints `amaranctl background daemon started (PID: ...)` and immediately returns to the `λ` prompt (no hanging CMD window!).
- Test 2: In terminal, run `amaranctl brightness 60`.
  Verify: Executes in ~15 ms via IPC, terminal waits synchronously, prompt returns cleanly.
- Test 3: In terminal, run `amaranctl run` again.
  Verify: Prints `amaranctl background daemon is already running.` without duplicate processes.
- Test 4: Terminate daemon (`Stop-Process -Name amaranctl`), run `amaranctl on`.
  Verify: Direct BLE fallback executes cleanly.

- [ ] **Step 5: Commit changes**

`git commit --no-gpg-sign -m "chore: verify detached daemon fork on Windows hardware and update progress ledger"`
