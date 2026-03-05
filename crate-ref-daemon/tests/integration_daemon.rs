//! Integration test: spawn daemon, send hook event via IPC, verify response.
//! Runs against actual binaries.

use std::io::{Read, Write};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

#[cfg(unix)]
fn test_socket_path() -> std::path::PathBuf {
    std::path::PathBuf::from("/tmp/crate-ref-daemon-test.sock")
}

#[cfg(windows)]
fn test_pipe_name() -> String {
    r"\\.\pipe\crate-ref-daemon-test".to_string()
}

fn spawn_daemon(ref_dir: &str) -> Child {
    #[cfg(unix)]
    {
        let sock = test_socket_path();
        if sock.exists() {
            std::fs::remove_file(&sock).unwrap();
        }
    }

    Command::new(env!("CARGO_BIN_EXE_crate-ref-daemon"))
        .arg("--ref-dir")
        .arg(ref_dir)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn daemon")
}

fn wait_for_daemon() {
    // Give the daemon time to start listening
    for _ in 0..30 {
        thread::sleep(Duration::from_millis(100));

        #[cfg(unix)]
        if test_socket_path().exists() {
            return;
        }

        #[cfg(windows)]
        {
            // Try to open the named pipe
            if std::fs::OpenOptions::new()
                .read(true)
                .open(test_pipe_name())
                .is_ok()
            {
                return;
            }
        }
    }
    // Even if we can't detect, proceed after 3s — the test will fail on connect if daemon isn't up
}

fn send_event(event_json: &[u8]) -> Vec<u8> {
    #[cfg(unix)]
    {
        use std::os::unix::net::UnixStream;
        let sock = test_socket_path();
        let mut stream = UnixStream::connect(sock).expect("connect to daemon");
        let len = (event_json.len() as u32).to_le_bytes();
        stream.write_all(&len).unwrap();
        stream.write_all(event_json).unwrap();

        let mut resp_len_buf = [0u8; 4];
        stream.read_exact(&mut resp_len_buf).unwrap();
        let resp_len = u32::from_le_bytes(resp_len_buf) as usize;
        let mut resp = vec![0u8; resp_len];
        stream.read_exact(&mut resp).unwrap();
        resp
    }

    #[cfg(windows)]
    {
        let pipe_name = test_pipe_name();
        let mut pipe = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&pipe_name)
            .expect("connect to daemon pipe");

        let len = (event_json.len() as u32).to_le_bytes();
        pipe.write_all(&len).unwrap();
        pipe.write_all(event_json).unwrap();
        pipe.flush().unwrap();

        let mut resp_len_buf = [0u8; 4];
        pipe.read_exact(&mut resp_len_buf).unwrap();
        let resp_len = u32::from_le_bytes(resp_len_buf) as usize;
        let mut resp = vec![0u8; resp_len];
        pipe.read_exact(&mut resp).unwrap();
        resp
    }
}

#[test]
#[ignore] // Run with: cargo test --test integration_daemon -- --ignored
fn test_daemon_responds_proceed_for_non_rs_file() {
    let tmp = tempfile::TempDir::new().unwrap();
    let mut child = spawn_daemon(tmp.path().to_str().unwrap());
    wait_for_daemon();

    let event = serde_json::json!({
        "hook_event_name": "PostToolUse",
        "tool_name": "Write",
        "tool_input": { "file_path": "README.md", "content": "hello" },
        "tool_response": null
    });
    let resp_bytes = send_event(&serde_json::to_vec(&event).unwrap());
    let resp: serde_json::Value = serde_json::from_slice(&resp_bytes).unwrap();
    assert_eq!(resp["action"], "proceed");

    child.kill().ok();
    #[cfg(unix)]
    if test_socket_path().exists() {
        std::fs::remove_file(test_socket_path()).ok();
    }
}

#[test]
#[ignore]
fn test_daemon_responds_proceed_for_rs_file_with_matching_ref() {
    let tmp = tempfile::TempDir::new().unwrap();

    // Write a reference file that matches the content we'll send
    let ref_content = "fn hello() { let x = 42; println!(\"{}\", x); }";
    std::fs::write(tmp.path().join("lib_test.rs"), ref_content).unwrap();

    // Write the "source" file -- same content, should match
    let src_path = tmp.path().join("src_test.rs");
    std::fs::write(&src_path, ref_content).unwrap();

    let mut child = spawn_daemon(tmp.path().to_str().unwrap());
    wait_for_daemon();

    let event = serde_json::json!({
        "hook_event_name": "PostToolUse",
        "tool_name": "Write",
        "tool_input": { "file_path": src_path.to_str().unwrap() },
        "tool_response": null
    });
    let resp_bytes = send_event(&serde_json::to_vec(&event).unwrap());
    let resp: serde_json::Value = serde_json::from_slice(&resp_bytes).unwrap();
    assert_eq!(resp["action"], "proceed");

    child.kill().ok();
    #[cfg(unix)]
    if test_socket_path().exists() {
        std::fs::remove_file(test_socket_path()).ok();
    }
}
