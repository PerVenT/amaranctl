use amaranctl::daemon::is_daemon_running;

#[tokio::test]
async fn test_is_daemon_running_when_not_started() {
    let running = is_daemon_running().await;
    assert!(!running);
}
