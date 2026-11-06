use amaranctl::ble::LampStatus;
use amaranctl::tray::{cct_to_rgb, create_adaptive_icon};

#[test]
fn test_cct_to_rgb_interpolation() {
    // 2700K Warm
    let (r, g, b) = cct_to_rgb(2700);
    assert_eq!((r, g, b), (255, 170, 51));

    // 4500K Neutral
    let (r, g, b) = cct_to_rgb(4500);
    assert_eq!((r, g, b), (255, 244, 224));

    // 6500K Cool / Daylight
    let (r, g, b) = cct_to_rgb(6500);
    assert_eq!((r, g, b), (204, 230, 255));

    // 5600K
    let (r, g, b) = cct_to_rgb(5600);
    assert!(r >= 204);
    assert!((230..=244).contains(&g));
    assert!(b >= 224);
}

#[test]
fn test_create_adaptive_icon_states() {
    // Connected ON
    let status_on = LampStatus {
        connected: true,
        connecting: false,
        power: true,
        brightness: 80.0,
        cct: 3200,
    };
    let _icon_on = create_adaptive_icon(&status_on, 0);

    // Connected OFF
    let status_off = LampStatus {
        connected: true,
        connecting: false,
        power: false,
        brightness: 80.0,
        cct: 3200,
    };
    let _icon_off = create_adaptive_icon(&status_off, 0);

    // Connecting
    let status_connecting = LampStatus {
        connected: false,
        connecting: true,
        power: false,
        brightness: 80.0,
        cct: 3200,
    };
    let _icon_conn = create_adaptive_icon(&status_connecting, 1);

    // Disconnected
    let status_disconnected = LampStatus {
        connected: false,
        connecting: false,
        power: false,
        brightness: 0.0,
        cct: 5600,
    };
    let _icon_disc = create_adaptive_icon(&status_disconnected, 0);
}
