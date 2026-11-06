use amaranctl::cli::{parse_brightness_val, parse_cct_val, Cli, Commands};
use clap::Parser;

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
    assert_eq!(
        cli.command,
        Some(Commands::Brightness { value: "50".into() })
    );

    let args = vec!["amaranctl", "brightness", "+10"];
    let cli = Cli::parse_from(args);
    assert_eq!(
        cli.command,
        Some(Commands::Brightness {
            value: "+10".into()
        })
    );

    let args = vec!["amaranctl", "brightness", "--", "-15"];
    let cli = Cli::parse_from(args);
    assert_eq!(
        cli.command,
        Some(Commands::Brightness {
            value: "-15".into()
        })
    );
}

#[test]
fn test_cli_cct() {
    let args = vec!["amaranctl", "cct", "5600"];
    let cli = Cli::parse_from(args);
    assert_eq!(
        cli.command,
        Some(Commands::Cct {
            value: "5600".into()
        })
    );

    let args = vec!["amaranctl", "cct", "+500"];
    let cli = Cli::parse_from(args);
    assert_eq!(
        cli.command,
        Some(Commands::Cct {
            value: "+500".into()
        })
    );
}

#[test]
fn test_cli_status_command() {
    let args = vec!["amaranctl", "status"];
    let cli = Cli::parse_from(args);
    assert_eq!(cli.command, Some(Commands::Status { json: false }));

    let args = vec!["amaranctl", "status", "--json"];
    let cli = Cli::parse_from(args);
    assert_eq!(cli.command, Some(Commands::Status { json: true }));
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

#[test]
fn test_parse_relative_adjustments() {
    // Brightness absolute
    assert_eq!(parse_brightness_val("50", 100.0).unwrap(), 50.0);

    // Brightness relative +
    assert_eq!(parse_brightness_val("+10", 75.0).unwrap(), 85.0);
    // Clamping at 100%
    assert_eq!(parse_brightness_val("+10", 95.0).unwrap(), 100.0);

    // Brightness relative -
    assert_eq!(parse_brightness_val("-15", 50.0).unwrap(), 35.0);
    // Clamping at 0%
    assert_eq!(parse_brightness_val("-15", 10.0).unwrap(), 0.0);

    // CCT absolute
    assert_eq!(parse_cct_val("5600", 3200).unwrap(), 5600);

    // CCT relative +
    assert_eq!(parse_cct_val("+500", 4500).unwrap(), 5000);
    // Clamping at 6500K
    assert_eq!(parse_cct_val("+500", 6300).unwrap(), 6500);

    // CCT relative -
    assert_eq!(parse_cct_val("-500", 4500).unwrap(), 4000);
    // Clamping at 2700K
    assert_eq!(parse_cct_val("-500", 2800).unwrap(), 2700);
}
