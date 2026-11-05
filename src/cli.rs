use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "amaranctl",
    author = "User",
    version = "0.1.0",
    about = "Lightweight BLE Mesh controller for Aputure Amaran lights",
    long_about = None
)]
pub struct Cli {
    #[arg(short, long, help = "Path to config.json")]
    pub config: Option<PathBuf>,

    #[arg(long, hide = true)]
    pub daemon_worker: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, Clone, PartialEq)]
pub enum Commands {
    #[command(about = "Turn light ON")]
    On,
    #[command(about = "Turn light OFF")]
    Off,
    #[command(about = "Toggle light power state")]
    Toggle,
    #[command(about = "Show current lamp status")]
    Status {
        #[arg(long, help = "Output status as JSON")]
        json: bool,
    },
    #[command(about = "Set brightness: 0..100% (or relative: +10, -10)")]
    Brightness {
        #[arg(allow_hyphen_values = true, help = "Brightness value: 0..100 (percentage) or relative (+10, -10)")]
        value: String,
    },
    #[command(about = "Set color temperature: 2700K - 6500K (or relative: +500, -500)")]
    Cct {
        #[arg(allow_hyphen_values = true, help = "Color temperature in Kelvin (2700 - 6500) or relative (+500, -500)")]
        value: String,
    },
    #[command(about = "Run background daemon with tray icon and global hotkeys")]
    Run {
        #[arg(short, long, help = "Run in foreground without detaching")]
        foreground: bool,
    },
}

pub fn parse_brightness_val(val_str: &str, current: f32) -> anyhow::Result<f32> {
    let s = val_str.trim();
    if let Some(rest) = s.strip_prefix('+') {
        let delta: f32 = rest.parse()?;
        Ok((current + delta).clamp(0.0, 100.0))
    } else if let Some(rest) = s.strip_prefix('-') {
        let delta: f32 = rest.parse()?;
        Ok((current - delta).clamp(0.0, 100.0))
    } else {
        let val: f32 = s.parse()?;
        Ok(val)
    }
}

pub fn parse_cct_val(val_str: &str, current: u16) -> anyhow::Result<u16> {
    let s = val_str.trim();
    if let Some(rest) = s.strip_prefix('+') {
        let delta: i32 = rest.parse()?;
        Ok(((current as i32) + delta).clamp(2700, 6500) as u16)
    } else if let Some(rest) = s.strip_prefix('-') {
        let delta: i32 = rest.parse()?;
        Ok(((current as i32) - delta).clamp(2700, 6500) as u16)
    } else {
        let val: u16 = s.parse()?;
        Ok(val.clamp(2700, 6500))
    }
}
