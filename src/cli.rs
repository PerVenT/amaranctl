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
    #[command(about = "Set brightness: 0..100% (or 0..1000 raw)")]
    Brightness {
        #[arg(help = "Brightness value: 0..100 (percentage) or 0..1000 (raw)")]
        value: f32,
    },
    #[command(about = "Set color temperature (2700K - 6500K)")]
    Cct {
        #[arg(help = "Color temperature in Kelvin (2700 - 6500)")]
        kelvin: u16,
    },
    #[command(about = "Run background daemon with tray icon and global hotkeys")]
    Run,
}
