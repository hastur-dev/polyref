// tokio Reference — Async runtime for Rust
// Cargo.toml: tokio = { version = "1", features = ["full"] }
// Usage: use tokio;

use tokio;

// ============================================================================
// BASIC USAGE
// ============================================================================

#[tokio::main]
async fn main() {                                           // async main entry point
    println!("hello from tokio");
}

#[tokio::test]
async fn test_async() {                                     // async test
    assert!(true);
}

// ============================================================================
// COMMON PATTERNS
// ============================================================================

async fn spawn_example() {
    let handle = tokio::spawn(async {                       // spawn a task
        42
    });
    let result = handle.await.unwrap();                     // wait for result
}

async fn channel_example() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(100);     // multi-producer channel
    tx.send("hello").await.unwrap();
    let msg = rx.recv().await.unwrap();
}

async fn sleep_example() {
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;  // async sleep
}
