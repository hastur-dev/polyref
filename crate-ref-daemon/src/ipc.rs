use std::path::PathBuf;

/// Returns the canonical socket path for the daemon.
/// Unix: /tmp/crate-ref-daemon.sock
/// Windows: \\.\pipe\crate-ref-daemon
pub fn socket_path() -> PathBuf {
    #[cfg(unix)]
    {
        PathBuf::from("/tmp/crate-ref-daemon.sock")
    }
    #[cfg(windows)]
    {
        PathBuf::from(r"\\.\pipe\crate-ref-daemon")
    }
}

/// Returns true if the daemon socket exists (daemon is running).
pub fn daemon_is_running() -> bool {
    #[cfg(unix)]
    {
        socket_path().exists()
    }
    #[cfg(windows)]
    {
        std::fs::OpenOptions::new()
            .read(true)
            .open(socket_path())
            .is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_socket_path_is_absolute() {
        assert!(socket_path().is_absolute());
    }

    #[test]
    fn test_daemon_is_running_returns_bool() {
        // This is a non-panicking smoke test; the daemon may or may not be running.
        let _running: bool = daemon_is_running();
    }
}
