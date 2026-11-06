use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", content = "val", rename_all = "snake_case")]
pub enum IpcCommand {
    On,
    Off,
    Toggle,
    Brightness(f32),
    Cct(u16),
    Status,
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IpcResponse {
    pub ok: bool,
    pub message: Option<String>,
}

pub const WINDOWS_PIPE_NAME: &str = r"\\.\pipe\amaranctl";

#[cfg(windows)]
pub async fn send_ipc_command(cmd: &IpcCommand) -> Result<IpcResponse> {
    use tokio::net::windows::named_pipe::ClientOptions;

    let client = tokio::time::timeout(
        Duration::from_millis(200),
        async {
            loop {
                match ClientOptions::new().open(WINDOWS_PIPE_NAME) {
                    Ok(c) => return Ok(c),
                    Err(e) if e.raw_os_error() == Some(2) || e.raw_os_error() == Some(231) => {
                        // ERROR_FILE_NOT_FOUND (pipe starting up) or ERROR_PIPE_BUSY: retry
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                    Err(e) => return Err(anyhow::Error::from(e)),
                }
            }
        },
    )
    .await
    .context("Timed out connecting to daemon pipe")??;

    let (reader, mut writer) = tokio::io::split(client);
    let mut buf_reader = BufReader::new(reader);

    let mut line = serde_json::to_string(cmd)?;
    line.push('\n');
    writer.write_all(line.as_bytes()).await?;
    writer.flush().await?;

    let mut resp_line = String::new();
    tokio::time::timeout(Duration::from_secs(4), buf_reader.read_line(&mut resp_line))
        .await
        .context("Timed out waiting for daemon response")??;

    let resp: IpcResponse = serde_json::from_str(resp_line.trim())?;
    Ok(resp)
}

#[cfg(not(windows))]
pub fn get_unix_socket_path() -> std::path::PathBuf {
    if let Some(home) = dirs::home_dir() {
        home.join(".config").join("amaranctl").join("amaranctl.sock")
    } else {
        std::path::PathBuf::from("/tmp/amaranctl.sock")
    }
}

#[cfg(not(windows))]
pub async fn send_ipc_command(cmd: &IpcCommand) -> Result<IpcResponse> {
    use tokio::net::UnixStream;

    let sock_path = get_unix_socket_path();
    let stream = tokio::time::timeout(Duration::from_millis(200), UnixStream::connect(&sock_path))
        .await
        .context("Timed out connecting to UNIX domain socket")?
        .context("Failed to connect to UNIX domain socket")?;

    let (reader, mut writer) = stream.into_split();
    let mut buf_reader = BufReader::new(reader);

    let mut line = serde_json::to_string(cmd)?;
    line.push('\n');
    writer.write_all(line.as_bytes()).await?;
    writer.flush().await?;

    let mut resp_line = String::new();
    tokio::time::timeout(Duration::from_secs(4), buf_reader.read_line(&mut resp_line))
        .await
        .context("Timed out waiting for daemon response")??;

    let resp: IpcResponse = serde_json::from_str(resp_line.trim())?;
    Ok(resp)
}

#[cfg(windows)]
pub struct IpcServer;

#[cfg(windows)]
impl IpcServer {
    pub async fn run<F, Fut>(handler: F) -> Result<()>
    where
        F: Fn(IpcCommand) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = IpcResponse> + Send + 'static,
    {
        use std::sync::Arc;
        use tokio::net::windows::named_pipe::ServerOptions;

        let handler = Arc::new(handler);
        let mut server = ServerOptions::new()
            .first_pipe_instance(true)
            .create(WINDOWS_PIPE_NAME)
            .context("Failed to create first named pipe instance")?;

        loop {
            if server.connect().await.is_err() {
                continue;
            }
            let connected_client = server;

            // Create next instance ready to accept incoming clients
            server = match ServerOptions::new().create(WINDOWS_PIPE_NAME) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("[ipc] Error creating next pipe instance: {:?}", e);
                    break;
                }
            };

            let h = handler.clone();
            tokio::spawn(async move {
                let (reader, mut writer) = tokio::io::split(connected_client);
                let mut buf_reader = BufReader::new(reader);
                let mut line = String::new();

                if let Ok(n) = buf_reader.read_line(&mut line).await {
                    if n > 0 {
                        if let Ok(cmd) = serde_json::from_str::<IpcCommand>(line.trim()) {
                            let resp = h(cmd).await;
                            if let Ok(mut resp_json) = serde_json::to_string(&resp) {
                                resp_json.push('\n');
                                let _ = writer.write_all(resp_json.as_bytes()).await;
                                let _ = writer.flush().await;
                            }
                        }
                    }
                }
            });
        }
        Ok(())
    }
}

#[cfg(not(windows))]
pub struct IpcServer;

#[cfg(not(windows))]
impl IpcServer {
    pub async fn run<F, Fut>(handler: F) -> Result<()>
    where
        F: Fn(IpcCommand) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = IpcResponse> + Send + 'static,
    {
        use std::sync::Arc;
        use tokio::net::UnixListener;

        let sock_path = get_unix_socket_path();
        if let Some(parent) = sock_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::remove_file(&sock_path);

        let listener = UnixListener::bind(&sock_path)
            .with_context(|| format!("Failed to bind UNIX domain socket at {:?}", sock_path))?;

        let handler = Arc::new(handler);

        loop {
            let (stream, _) = match listener.accept().await {
                Ok(res) => res,
                Err(e) => {
                    eprintln!("[ipc] Accept error: {:?}", e);
                    continue;
                }
            };

            let h = handler.clone();
            tokio::spawn(async move {
                let (reader, mut writer) = stream.into_split();
                let mut buf_reader = BufReader::new(reader);
                let mut line = String::new();

                if let Ok(n) = buf_reader.read_line(&mut line).await {
                    if n > 0 {
                        if let Ok(cmd) = serde_json::from_str::<IpcCommand>(line.trim()) {
                            let resp = h(cmd).await;
                            if let Ok(mut resp_json) = serde_json::to_string(&resp) {
                                resp_json.push('\n');
                                let _ = writer.write_all(resp_json.as_bytes()).await;
                                let _ = writer.flush().await;
                            }
                        }
                    }
                }
            });
        }
    }
}
