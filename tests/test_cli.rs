use clap::Parser;
use amaranctl::cli::{Cli, Commands};

#[test]
fn test_cli_on_off() {
    let args = vec!["amaranctl", "on"];
    let cli = Cli::parse_from(args);
    assert_eq!(cli.command, Some(Commands::On));

    let args = vec!["amaranctl", "off"];
    let cli = Cli::parse_from(args);
    assert_eq!(cli.command, Some(Commands::Off));
}

#[test]
fn test_cli_brightness() {
    let args = vec!["amaranctl", "brightness", "50"];
    let cli = Cli::parse_from(args);
    assert_eq!(cli.command, Some(Commands::Brightness { value: 50.0 }));
}

#[test]
fn test_cli_cct() {
    let args = vec!["amaranctl", "cct", "5600"];
    let cli = Cli::parse_from(args);
    assert_eq!(cli.command, Some(Commands::Cct { kelvin: 5600 }));
}

#[test]
fn test_cli_daemon_worker_flag() {
    let args = vec!["amaranctl", "--daemon-worker"];
    let cli = Cli::parse_from(args);
    assert!(cli.daemon_worker);
    assert_eq!(cli.command, None);
}

#[test]
fn test_cli_run_command() {
    let args = vec!["amaranctl", "run"];
    let cli = Cli::parse_from(args);
    assert_eq!(cli.command, Some(Commands::Run { foreground: false }));

    let args = vec!["amaranctl", "run", "--foreground"];
    let cli = Cli::parse_from(args);
    assert_eq!(cli.command, Some(Commands::Run { foreground: true }));
}
