use anyhow::Result;
use clap::Parser;
use std::env;
use std::path::{Path, PathBuf};

use amaranctl::ble::BleMeshClient;
use amaranctl::cli::{Cli, Commands};
use amaranctl::config::AppConfig;
use amaranctl::tray::run_tray_app;

fn find_config_path(custom: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = custom {
        if p.exists() {
            return Ok(p);
        }
        anyhow::bail!("Config file not found at: {:?}", p);
    }

    // 1. Current directory
    let local = Path::new("config.json");
    if local.exists() {
        return Ok(local.to_path_buf());
    }

    // 2. Next to executable
    if let Ok(exe) = env::current_exe() {
        if let Some(dir) = exe.parent() {
            let exe_cfg = dir.join("config.json");
            if exe_cfg.exists() {
                return Ok(exe_cfg);
            }
        }
    }

    // 3. User config directory
    if let Some(cfg_dir) = dirs::config_dir() {
        let app_cfg = cfg_dir.join("amaranctl/config.json");
        if app_cfg.exists() {
            return Ok(app_cfg);
        }
    }

    anyhow::bail!("Could not find config.json. Place config.json next to the executable or specify --config <path>")
}

fn main() -> Result<()> {
    let args = Cli::parse();
    let config_path = find_config_path(args.config)?;
    let config = AppConfig::load_from_path(&config_path)?;

    match args.command {
        Some(Commands::On) => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            rt.block_on(async {
                println!("Turning Amaran ON...");
                let mut client = BleMeshClient::new(config)?;
                client.connect().await?;
                client.set_power(true).await?;
                println!("Done.");
                Ok::<(), anyhow::Error>(())
            })?;
        }
        Some(Commands::Off) => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            rt.block_on(async {
                println!("Turning Amaran OFF...");
                let mut client = BleMeshClient::new(config)?;
                client.connect().await?;
                client.set_power(false).await?;
                println!("Done.");
                Ok::<(), anyhow::Error>(())
            })?;
        }
        Some(Commands::Toggle) => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            rt.block_on(async {
                println!("Toggling Amaran power...");
                let mut client = BleMeshClient::new(config)?;
                client.connect().await?;
                let new_state = !client.last_power;
                client.set_power(new_state).await?;
                println!("Done.");
                Ok::<(), anyhow::Error>(())
            })?;
        }
        Some(Commands::Brightness { value }) => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            rt.block_on(async {
                let mut client = BleMeshClient::new(config)?;
                client.connect().await?;
                if value > 100.0 {
                    let raw = value.clamp(0.0, 1000.0) as u16;
                    println!("Setting raw brightness to {} / 1000...", raw);
                    client.set_brightness_raw(raw).await?;
                } else {
                    let pct = value.clamp(0.0, 100.0);
                    println!("Setting brightness to {:.1}%...", pct);
                    client.set_brightness(pct).await?;
                }
                println!("Done.");
                Ok::<(), anyhow::Error>(())
            })?;
        }
        Some(Commands::Cct { kelvin }) => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            rt.block_on(async {
                let k = kelvin.clamp(2700, 6500);
                println!("Setting CCT to {}K...", k);
                let mut client = BleMeshClient::new(config)?;
                client.connect().await?;
                client.set_cct(k).await?;
                println!("Done.");
                Ok::<(), anyhow::Error>(())
            })?;
        }
        Some(Commands::Run) | None => {
            println!("Starting amaranctl background tray application...");
            run_tray_app(config)?;
        }
    }

    Ok(())
}
