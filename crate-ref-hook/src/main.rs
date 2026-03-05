use anyhow::Result;
use std::io::{Read, Write};
use std::process;

fn main() {
    if let Err(e) = run() {
        eprintln!("[crate-ref-hook] error: {e:#}");
        process::exit(0); // Never block Claude Code on hook errors
    }
}

fn run() -> Result<()> {
    // Read JSON from stdin (Claude Code sends the full event here)
    let mut stdin_buf = Vec::new();
    std::io::stdin().read_to_end(&mut stdin_buf)?;

    // Ensure daemon is running; spawn if not
    ensure_daemon_running()?;

    // Connect to daemon and exchange messages
    #[cfg(unix)]
    {
        use std::os::unix::net::UnixStream;
        let sock = socket_path();
        let mut stream = UnixStream::connect(&sock)?;
        let resp_buf = exchange(&mut stream, &stdin_buf)?;
        std::io::stdout().write_all(&resp_buf)?;
        exit_from_response(&resp_buf);
    }

    #[cfg(windows)]
    {
        let pipe_path = socket_path();
        let pipe_str = pipe_path.to_string_lossy().to_string();
        let mut pipe = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&pipe_str)?;
        let resp_buf = exchange(&mut pipe, &stdin_buf)?;
        std::io::stdout().write_all(&resp_buf)?;
        exit_from_response(&resp_buf);
    }

    #[allow(unreachable_code)]
    {
        process::exit(0);
    }
}

fn exchange<S: Read + Write>(stream: &mut S, data: &[u8]) -> Result<Vec<u8>> {
    // Send length-prefixed JSON
    let len = (data.len() as u32).to_le_bytes();
    stream.write_all(&len)?;
    stream.write_all(data)?;
    stream.flush()?;

    // Read response
    let mut resp_len_buf = [0u8; 4];
    stream.read_exact(&mut resp_len_buf)?;
    let resp_len = u32::from_le_bytes(resp_len_buf) as usize;
    let mut resp_buf = vec![0u8; resp_len];
    stream.read_exact(&mut resp_buf)?;

    Ok(resp_buf)
}

fn exit_from_response(resp_buf: &[u8]) {
    let response: serde_json::Value = serde_json::from_slice(resp_buf).unwrap_or_default();
    if response.get("action").and_then(|a| a.as_str()) == Some("block") {
        process::exit(2);
    }
    process::exit(0);
}

fn socket_path() -> std::path::PathBuf {
    #[cfg(unix)]
    {
        std::path::PathBuf::from("/tmp/crate-ref-daemon.sock")
    }
    #[cfg(windows)]
    {
        std::path::PathBuf::from(r"\\.\pipe\crate-ref-daemon")
    }
}

fn ensure_daemon_running() -> Result<()> {
    #[cfg(unix)]
    {
        let sock = socket_path();
        if sock.exists() {
            return Ok(());
        }
    }

    #[cfg(windows)]
    {
        // On Windows, use WaitNamedPipe to check if the daemon pipe exists
        // without consuming a pipe instance (unlike opening it).
        use std::os::windows::ffi::OsStrExt;
        let pipe_path = socket_path();
        let wide: Vec<u16> = pipe_path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        // SAFETY: WaitNamedPipeW is a read-only probe; zero timeout = just check existence
        let exists =
            unsafe { windows_sys::Win32::System::Pipes::WaitNamedPipeW(wide.as_ptr(), 0) };
        if exists != 0 {
            return Ok(());
        }
    }

    // Spawn daemon in background
    let daemon_bin = std::env::current_exe()?
        .parent()
        .unwrap_or(std::path::Path::new("."))
        .join(if cfg!(windows) {
            "crate-ref-daemon.exe"
        } else {
            "crate-ref-daemon"
        });

    std::process::Command::new(&daemon_bin)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()?;

    // Wait for pipe to appear (up to 2 seconds)
    for _ in 0..20 {
        std::thread::sleep(std::time::Duration::from_millis(100));

        #[cfg(unix)]
        if socket_path().exists() {
            return Ok(());
        }

        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            let pipe_path = socket_path();
            let wide: Vec<u16> = pipe_path
                .as_os_str()
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let exists = unsafe {
                windows_sys::Win32::System::Pipes::WaitNamedPipeW(wide.as_ptr(), 0)
            };
            if exists != 0 {
                return Ok(());
            }
        }
    }

    anyhow::bail!("daemon did not start within 2 seconds")
}
