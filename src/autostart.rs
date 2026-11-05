use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::process::Command;

pub fn get_linux_service_content(exe_path: &str) -> String {
    format!(
        r#"[Unit]
Description=Amaran 60x S BLE Mesh Controller
After=bluetooth.target

[Service]
Type=simple
ExecStart={} run
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
"#,
        exe_path
    )
}

#[cfg(windows)]
pub fn is_autostart_enabled() -> bool {
    let output = Command::new("reg")
        .args(["query", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run", "/v", "amaranctl"])
        .output();

    match output {
        Ok(out) => out.status.success(),
        Err(_) => false,
    }
}

#[cfg(not(windows))]
pub fn is_autostart_enabled() -> bool {
    if let Some(home) = dirs::home_dir() {
        let service_path = home.join(".config/systemd/user/amaranctl.service");
        service_path.exists()
    } else {
        false
    }
}

#[cfg(windows)]
pub fn set_autostart(enable: bool) -> Result<()> {
    if enable {
        let exe = env::current_exe().context("failed to get current executable path")?;
        let exe_str = exe.to_string_lossy().to_string();
        let val = format!("\"{}\"", exe_str);

        let status = Command::new("reg")
            .args([
                "add",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                "amaranctl",
                "/t",
                "REG_SZ",
                "/d",
                &val,
                "/f",
            ])
            .status()
            .context("failed to execute reg.exe add")?;

        if !status.success() {
            anyhow::bail!("reg add command exited with status: {}", status);
        }
    } else {
        let _ = Command::new("reg")
            .args([
                "delete",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run",
                "/v",
                "amaranctl",
                "/f",
            ])
            .status();
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn set_autostart(enable: bool) -> Result<()> {
    let home = dirs::home_dir().context("failed to get home directory")?;
    let unit_dir = home.join(".config/systemd/user");
    let unit_file = unit_dir.join("amaranctl.service");

    if enable {
        fs::create_dir_all(&unit_dir).context("failed to create systemd user dir")?;
        let exe = env::current_exe().context("failed to get current executable path")?;
        let content = get_linux_service_content(&exe.to_string_lossy());
        fs::write(&unit_file, content).context("failed to write systemd unit file")?;

        let _ = Command::new("systemctl").args(["--user", "daemon-reload"]).status();
        let _ = Command::new("systemctl").args(["--user", "enable", "amaranctl"]).status();
    } else {
        let _ = Command::new("systemctl").args(["--user", "disable", "amaranctl"]).status();
        let _ = fs::remove_file(&unit_file);
        let _ = Command::new("systemctl").args(["--user", "daemon-reload"]).status();
    }
    Ok(())
}
