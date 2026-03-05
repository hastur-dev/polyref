use anyhow::Result;
use clap::Parser;
use crate_ref_daemon::{
    daemon::{Daemon, DaemonConfig},
    hook_types::{HookAction, HookEvent, HookResponse},
    ipc::socket_path,
    watcher::RefWatcher,
};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "crate-ref-daemon",
    about = "Persistent hook daemon for crate-ref-check"
)]
struct Args {
    /// Directory containing lib_*.rs reference files
    #[arg(long, env = "CRATE_REF_DIR")]
    ref_dir: Option<PathBuf>,

    /// Path to persist the flat index
    #[arg(long, env = "CRATE_REF_INDEX")]
    index_path: Option<PathBuf>,

    /// Hamming distance threshold (default: 8)
    #[arg(long, default_value_t = 8)]
    threshold: u32,

    /// Project directory to scan for dependencies (auto-generates missing refs on startup)
    #[arg(long, env = "CRATE_REF_PROJECT_DIR")]
    project_dir: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let mut config = DaemonConfig::default();
    if let Some(d) = args.ref_dir {
        config.ref_dir = d;
    }
    if let Some(p) = args.index_path {
        config.index_path = p;
    }
    config.threshold = args.threshold;
    config.project_dir = args.project_dir;

    let sock = socket_path();
    eprintln!("[crate-ref-daemon] will listen on {:?}", sock);

    // Build daemon state (index load or rebuild)
    let ref_dir = config.ref_dir.clone();
    let mut daemon = Daemon::new(config)?;

    // Start file watcher if ref_dir exists
    let watcher = if ref_dir.exists() {
        Some(RefWatcher::watch(&ref_dir)?)
    } else {
        None
    };

    // Platform-specific server loop
    serve(&mut daemon, watcher.as_ref()).await
}

#[cfg(unix)]
async fn serve(daemon: &mut Daemon, watcher: Option<&RefWatcher>) -> Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::UnixListener;

    let sock = socket_path();

    // Remove stale socket from a previous crash
    if sock.exists() {
        std::fs::remove_file(&sock)?;
    }

    let listener = UnixListener::bind(&sock)?;
    eprintln!("[crate-ref-daemon] listening on {:?}", sock);

    loop {
        if let Some(w) = watcher {
            daemon.apply_watcher_events(w)?;
        }

        let (mut stream, _) = listener.accept().await?;
        handle_connection_async(&mut stream, daemon).await?;
    }
}

#[cfg(windows)]
async fn serve(daemon: &mut Daemon, watcher: Option<&RefWatcher>) -> Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::windows::named_pipe::{PipeMode, ServerOptions};

    let pipe_name = socket_path();
    let pipe_str = pipe_name.to_string_lossy().to_string();
    eprintln!("[crate-ref-daemon] listening on {}", pipe_str);

    // Create the first pipe instance (first_pipe_instance = true to claim the name)
    let mut server = ServerOptions::new()
        .first_pipe_instance(true)
        .pipe_mode(PipeMode::Byte)
        .create(&pipe_str)?;

    loop {
        if let Some(w) = watcher {
            daemon.apply_watcher_events(w)?;
        }

        server.connect().await?;

        // Create the next pipe instance immediately so new clients can connect
        // while we handle the current one.
        let next_server = ServerOptions::new()
            .pipe_mode(PipeMode::Byte)
            .create(&pipe_str)?;

        // Read length-prefixed JSON (4-byte LE u32 length, then JSON bytes)
        let mut len_buf = [0u8; 4];
        if server.read_exact(&mut len_buf).await.is_err() {
            server = next_server;
            continue;
        }
        let len = u32::from_le_bytes(len_buf) as usize;

        let mut buf = vec![0u8; len];
        if server.read_exact(&mut buf).await.is_err() {
            server = next_server;
            continue;
        }

        let response = match serde_json::from_slice::<HookEvent>(&buf) {
            Ok(event) => daemon.handle_event(&event),
            Err(e) => HookResponse {
                action: HookAction::Proceed,
                message: Some(format!("parse error: {e}")),
            },
        };

        let resp_bytes = serde_json::to_vec(&response)?;
        let resp_len = (resp_bytes.len() as u32).to_le_bytes();
        let _ = server.write_all(&resp_len).await;
        let _ = server.write_all(&resp_bytes).await;

        // Move to the next pipe instance for the next client
        server = next_server;
    }
}

#[cfg(unix)]
async fn handle_connection_async<S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin>(
    stream: &mut S,
    daemon: &mut Daemon,
) -> Result<()> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let mut len_buf = [0u8; 4];
    stream.read_exact(&mut len_buf).await?;
    let len = u32::from_le_bytes(len_buf) as usize;

    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;

    let response = match serde_json::from_slice::<HookEvent>(&buf) {
        Ok(event) => daemon.handle_event(&event),
        Err(e) => HookResponse {
            action: HookAction::Proceed,
            message: Some(format!("parse error: {e}")),
        },
    };

    let resp_bytes = serde_json::to_vec(&response)?;
    let resp_len = (resp_bytes.len() as u32).to_le_bytes();
    stream.write_all(&resp_len).await?;
    stream.write_all(&resp_bytes).await?;

    Ok(())
}
