use std::path::Path;
use amaranctl::config::AppConfig;

#[test]
fn test_load_real_config_file() {
    let config = AppConfig::load_from_path(Path::new("config.json")).expect("Failed to load config.json");
    assert_eq!(config.mesh_uuid, "c4e7e379d8ad11f1b2433ec5dd894707");
    assert_eq!(config.net_key_bytes().len(), 16);
    assert_eq!(config.app_key_bytes().len(), 16);
    assert_eq!(config.lights.len(), 1);
    
    let light = &config.lights[0];
    assert_eq!(light.name, "amaran 60x S-FD81D5");
    assert_eq!(light.mac, "70:3E:97:FD:81:D5");
    assert_eq!(light.address, 2);
    assert_eq!(light.device_key_bytes().unwrap().len(), 16);
}

#[test]
fn test_parse_config_str() {
    let json = r#"{
      "meshUuid": "c4e7e379d8ad11f1b2433ec5dd894707",
      "netKey": "58015449C4CEDAA2B9E1FA1BB580973F",
      "appKey": "FA1BB580973F58015449C4CEDAA2B9E1",
      "lights": [
        {
          "name": "test light",
          "mac": "AA:BB:CC:DD:EE:FF",
          "address": 2
        }
      ]
    }"#;
    let config = AppConfig::from_json_str(json).expect("valid parse");
    assert_eq!(config.lights[0].name, "test light");
    assert_eq!(config.lights[0].address, 2);
}

#[test]
fn test_parse_config_with_utf8_bom() {
    // UTF-8 BOM is \u{feff} (bytes EF BB BF)
    let json_with_bom = "\u{feff}{\"meshUuid\":\"1234\",\"netKey\":\"00112233445566778899aabbccddeeff\",\"appKey\":\"ffeeddccbbaa99887766554433221100\",\"lights\":[]}";
    let config = AppConfig::from_json_str(json_with_bom).expect("BOM should be stripped and parsed cleanly");
    assert_eq!(config.mesh_uuid, "1234");
    assert_eq!(config.net_key_bytes().len(), 16);
}

#[test]
fn test_empty_config_fails_cleanly() {
    let empty = "";
    assert!(AppConfig::from_json_str(empty).is_err());
    let whitespace = "   \n\t  ";
    assert!(AppConfig::from_json_str(whitespace).is_err());
}
