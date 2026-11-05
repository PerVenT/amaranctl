use anyhow::{Context, Result};
use std::path::Path;
use std::process::{Command, Stdio};
use crate::ipc::{send_ipc_command, IpcCommand};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// Checks whether an amaranctl background daemon is currently responsive via IPC.
pub async fn is_daemon_running() -> bool {
    match send_ipc_command(&IpcCommand::Ping).await {
        Ok(resp) => resp.ok,
        Err(_) => false,
    }
}

/// Spawns a detached background child daemon process running `--daemon-worker`.
///
/// On Windows, sets `CREATE_NO_WINDOW | DETACHED_PROCESS` (0x08000000 | 0x00000008)
/// with standard I/O redirected to null, preventing any console window from appearing
/// and ensuring the process runs independently of the parent terminal.
///
/// Returns the PID of the spawned child process.
pub fn spawn_detached_daemon(config_path: Option<&Path>) -> Result<u32> {
    let current_exe = std::env::current_exe().context("Failed to obtain current executable path")?;

    let mut cmd = Command::new(&current_exe);
    cmd.arg("--daemon-worker");

    if let Some(cfg) = config_path {
        cmd.arg("--config").arg(cfg);
    }

    cmd.stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        const DETACHED_PROCESS: u32 = 0x00000008;
        cmd.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);
    }

    let child = cmd.spawn().context("Failed to spawn detached daemon process")?;
    Ok(child.id())
}
