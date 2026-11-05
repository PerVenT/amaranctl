use anyhow::Result;
use clap::Parser;
use std::env;
use std::path::PathBuf;

use amaranctl::ble::BleMeshClient;
use amaranctl::cli::{Cli, Commands};
use amaranctl::config::AppConfig;
use amaranctl::daemon::is_daemon_running;
#[cfg(windows)]
use amaranctl::daemon::spawn_detached_daemon;
use amaranctl::ipc::{send_ipc_command, IpcCommand};
use amaranctl::tray::run_tray_app;

fn load_or_auto_import_config(custom: Option<PathBuf>) -> Result<AppConfig> {
    if let Some(p) = custom {
        if p.exists() {
            return AppConfig::load_from_path(&p);
        }
        anyhow::bail!("Config file not found at: {:?}", p);
    }

    let mut candidates = Vec::new();

    // 1. Current directory
    candidates.push(PathBuf::from("config.json"));

    // 2. Next to executable
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("config.json"));
        }
    }

    // 3. ~/.config/amaranctl/config.json (cross-platform home directory)
    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".config").join("amaranctl").join("config.json"));
    }

    // 4. User standard config directory (e.g. %APPDATA% on Windows, ~/.config on Linux)
    if let Some(cfg_dir) = dirs::config_dir() {
        candidates.push(cfg_dir.join("amaranctl").join("config.json"));
    }

    for path in &candidates {
        if path.exists() {
            match AppConfig::load_from_path(path) {
                Ok(cfg) => return Ok(cfg),
                Err(e) => {
                    eprintln!("[!] Warning: Could not parse config at {:?}: {}", path, e);
                }
            }
        }
    }

    // 5. If no valid config file found anywhere, attempt auto-import from amaran.db!
    match AppConfig::try_auto_import_from_db() {
        Ok(cfg) => return Ok(cfg),
        Err(_) => {}
    }

    anyhow::bail!(
        "Could not find valid config.json or amaran.db.\n\
         Please make sure Amaran Desktop is installed and logged in, or place config.json in ~/.config/amaranctl/config.json"
    )
}

fn main() -> Result<()> {
    let args = Cli::parse();

    // 1. If invoked as the detached background worker or with --foreground, run the tray app directly
    if args.daemon_worker {
        let config = load_or_auto_import_config(args.config)?;
        return run_tray_app(config);
    }

    if let Some(Commands::Run { foreground: true }) = args.command {
        let config = load_or_auto_import_config(args.config)?;
        return run_tray_app(config);
    }

    // 2. Check if command can be dispatched to running background daemon via IPC
    let maybe_ipc = match &args.command {
        Some(Commands::On) => Some(IpcCommand::On),
        Some(Commands::Off) => Some(IpcCommand::Off),
        Some(Commands::Toggle) => Some(IpcCommand::Toggle),
        Some(Commands::Brightness { value }) => Some(IpcCommand::Brightness(*value)),
        Some(Commands::Cct { kelvin }) => Some(IpcCommand::Cct(*kelvin)),
        _ => None,
    };

    if let Some(cmd) = maybe_ipc {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;

        // A. Try sending to background daemon (instant: ~15ms)
        if let Ok(res) = rt.block_on(send_ipc_command(&cmd)) {
            if let Some(msg) = res.message {
                println!("{}", msg);
            } else {
                println!("OK");
            }
            return Ok(());
        }

        // B. Fallback: daemon not running, connect directly via BLE
        println!("Background daemon not running, connecting to Amaran directly...");
        let config = load_or_auto_import_config(args.config)?;
        rt.block_on(async {
            let mut client = BleMeshClient::new(config)?;
            client.ensure_connected().await?;
            match cmd {
                IpcCommand::On => {
                    client.set_power(true).await?;
                    println!("Amaran 60x S turned ON.");
                }
                IpcCommand::Off => {
                    client.set_power(false).await?;
                    println!("Amaran 60x S turned OFF.");
                }
                IpcCommand::Toggle => {
                    let cur = client.last_power;
                    client.set_power(!cur).await?;
                    println!("Power toggled to: {}.", if !cur { "ON" } else { "OFF" });
                }
                IpcCommand::Brightness(val) => {
                    if val > 100.0 {
                        let raw = val.clamp(0.0, 1000.0) as u16;
                        client.set_brightness_raw(raw).await?;
                        println!("Raw brightness set to {} / 1000.", raw);
                    } else {
                        let pct = val.clamp(0.0, 100.0);
                        client.set_brightness(pct).await?;
                        println!("Brightness set to {:.1}%.", pct);
                    }
                }
                IpcCommand::Cct(kelvin) => {
                    let k = kelvin.clamp(2700, 6500);
                    client.set_cct(k).await?;
                    println!("CCT set to {}K.", k);
                }
                _ => {}
            }
            let _ = client.disconnect().await;
            Ok::<(), anyhow::Error>(())
        })?;
        return Ok(());
    }

    // 3. User invoked `amaranctl run` or `amaranctl` (no subcommands):
    // Check if background daemon is already running via IPC ping
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;

    if rt.block_on(is_daemon_running()) {
        println!("amaranctl background daemon is already running.");
        return Ok(());
    }

    #[cfg(windows)]
    {
        // On Windows: Daemon is not running, spawn detached background child process
        let config_abs = args
            .config
            .as_ref()
            .map(|p| std::fs::canonicalize(p).unwrap_or_else(|_| p.clone()));

        match spawn_detached_daemon(config_abs.as_deref()) {
            Ok(pid) => {
                println!("amaranctl background daemon started (PID: {}).", pid);
                return Ok(());
            }
            Err(e) => {
                eprintln!("[!] Warning: Failed to spawn detached background daemon: {}. Falling back to foreground.", e);
            }
        }
    }

    let config = load_or_auto_import_config(args.config)?;
    run_tray_app(config)?;
    Ok(())
}
