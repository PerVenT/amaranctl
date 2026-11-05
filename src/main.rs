#![windows_subsystem = "windows"]

use anyhow::Result;
use clap::Parser;
use std::env;
use std::path::PathBuf;

use amaranctl::ble::BleMeshClient;
use amaranctl::cli::{Cli, Commands};
use amaranctl::config::AppConfig;
use amaranctl::tray::run_tray_app;

fn attach_parent_console_if_cli() {
    #[cfg(windows)]
    {
        if std::env::args().len() > 1 {
            unsafe {
                windows_sys::Win32::System::Console::AttachConsole(
                    windows_sys::Win32::System::Console::ATTACH_PARENT_PROCESS,
                );
            }
        }
    }
}

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
    attach_parent_console_if_cli();

    let args = Cli::parse();
    let config = load_or_auto_import_config(args.config)?;

    match args.command {
        Some(Commands::On) => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            rt.block_on(async {
                println!("Connecting to Amaran...");
                let mut client = BleMeshClient::new(config)?;
                client.set_power(true).await?;
                println!("Amaran 60x S turned ON.");
                let _ = client.disconnect().await;
                Ok::<(), anyhow::Error>(())
            })?;
        }
        Some(Commands::Off) => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            rt.block_on(async {
                println!("Connecting to Amaran...");
                let mut client = BleMeshClient::new(config)?;
                client.set_power(false).await?;
                println!("Amaran 60x S turned OFF.");
                let _ = client.disconnect().await;
                Ok::<(), anyhow::Error>(())
            })?;
        }
        Some(Commands::Toggle) => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            rt.block_on(async {
                println!("Connecting to Amaran...");
                let mut client = BleMeshClient::new(config)?;
                let new_state = !client.last_power;
                client.set_power(new_state).await?;
                println!("Power toggled to: {}.", if new_state { "ON" } else { "OFF" });
                let _ = client.disconnect().await;
                Ok::<(), anyhow::Error>(())
            })?;
        }
        Some(Commands::Brightness { value }) => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            rt.block_on(async {
                println!("Connecting to Amaran...");
                let mut client = BleMeshClient::new(config)?;
                if value > 100.0 {
                    let raw = value.clamp(0.0, 1000.0) as u16;
                    client.set_brightness_raw(raw).await?;
                    println!("Raw brightness set to {} / 1000.", raw);
                } else {
                    let pct = value.clamp(0.0, 100.0);
                    client.set_brightness(pct).await?;
                    println!("Brightness set to {:.1}%.", pct);
                }
                let _ = client.disconnect().await;
                Ok::<(), anyhow::Error>(())
            })?;
        }
        Some(Commands::Cct { kelvin }) => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            rt.block_on(async {
                println!("Connecting to Amaran...");
                let mut client = BleMeshClient::new(config)?;
                let k = kelvin.clamp(2700, 6500);
                client.set_cct(k).await?;
                println!("CCT set to {}K.", k);
                let _ = client.disconnect().await;
                Ok::<(), anyhow::Error>(())
            })?;
        }
        Some(Commands::Run) | None => {
            run_tray_app(config)?;
        }
    }

    Ok(())
}
