use amaranctl::autostart::{get_linux_service_content, is_autostart_enabled};

#[test]
fn test_linux_service_template() {
    let content = get_linux_service_content("/usr/local/bin/amaranctl");
    assert!(content.contains("ExecStart=/usr/local/bin/amaranctl run"));
    assert!(content.contains("WantedBy=default.target"));
}

#[test]
fn test_autostart_check_does_not_panic() {
    let _ = is_autostart_enabled();
}
