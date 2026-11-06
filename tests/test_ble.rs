use amaranctl::config::AppConfig;
use amaranctl::ble::{BleMeshClient, LampStatus};

#[test]
fn test_ble_client_initialization_and_seq() {
    let json = r#"{
      "meshUuid": "c4e7e379d8ad11f1b2433ec5dd894707",
      "netKey": "58015449C4CEDAA2B9E1FA1BB580973F",
      "appKey": "FA1BB580973F58015449C4CEDAA2B9E1",
      "lights": [
        {
          "name": "amaran 60x S-FD81D5",
          "mac": "70:3E:97:FD:81:D5",
          "address": 2
        }
      ]
    }"#;
    let config = AppConfig::from_json_str(json).unwrap();
    let mut client = BleMeshClient::new(config).expect("client creation");

    let s1 = client.next_seq();
    let s2 = client.next_seq();
    assert_eq!(s2, s1 + 1);

    let pdu_on = client.build_onoff_pdu(2, true).expect("build onoff pdu");
    assert_eq!(pdu_on[0], 0x00); // Proxy Network PDU
}

#[test]
fn test_lamp_status_defaults_and_formatting() {
    let mut s = LampStatus {
        connected: true,
        connecting: false,
        power: true,
        brightness: 75.0,
        cct: 5600,
    };
    assert_eq!(s.tooltip_text(), "Amaran 60x S — 🟢 ON | 75% | 5600K");

    s.power = false;
    assert_eq!(s.tooltip_text(), "Amaran 60x S — ⚪ OFF (Готовий)");

    s.connecting = true;
    assert_eq!(s.tooltip_text(), "Amaran 60x S — ⏳ Пошук та підключення...");

    s.connecting = false;
    s.connected = false;
    assert_eq!(s.tooltip_text(), "Amaran 60x S — 🔴 Немає зв'язку (перевірте живлення лампи)");
}

#[tokio::test]
async fn test_disconnect_resets_status() {
    let json = r#"{
      "meshUuid": "c4e7e379d8ad11f1b2433ec5dd894707",
      "netKey": "58015449C4CEDAA2B9E1FA1BB580973F",
      "appKey": "FA1BB580973F58015449C4CEDAA2B9E1",
      "lights": [
        {
          "name": "amaran 60x S-FD81D5",
          "mac": "70:3E:97:FD:81:D5",
          "address": 2
        }
      ]
    }"#;
    let config = AppConfig::from_json_str(json).unwrap();
    let mut client = BleMeshClient::new(config).expect("client creation");
    {
        let mut s = client.status.write().await;
        s.connected = true;
        s.connecting = true;
    }
    client.disconnect().await.expect("disconnect");
    let st = client.current_status().await;
    assert!(!st.connected);
    assert!(!st.connecting);
}
