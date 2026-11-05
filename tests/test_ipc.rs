use amaranctl::ipc::{IpcCommand, IpcResponse};

#[test]
fn test_ipc_command_json_roundtrip() {
    let cmd = IpcCommand::Brightness(75.5);
    let s = serde_json::to_string(&cmd).expect("serialize command");
    let parsed: IpcCommand = serde_json::from_str(&s).expect("deserialize command");
    match parsed {
        IpcCommand::Brightness(v) => assert!((v - 75.5).abs() < 0.001),
        _ => panic!("wrong variant"),
    }
}

#[test]
fn test_ipc_response_json_roundtrip() {
    let res = IpcResponse {
        ok: true,
        message: Some("Light toggled".to_string()),
    };
    let s = serde_json::to_string(&res).expect("serialize response");
    let parsed: IpcResponse = serde_json::from_str(&s).expect("deserialize response");
    assert!(parsed.ok);
    assert_eq!(parsed.message.as_deref(), Some("Light toggled"));
}
