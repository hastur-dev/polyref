// anyhow Reference — Flexible error handling for Rust
// Cargo.toml: anyhow = "1"
// Usage: use anyhow::{Result, Context, anyhow, bail, ensure};

use anyhow::{Result, Context, anyhow, bail, ensure};

// ============================================================================
// BASIC USAGE
// ============================================================================

fn read_config() -> Result<String> {                   // anyhow::Result<T> = Result<T, anyhow::Error>
    let content = std::fs::read_to_string("config.toml")
        .context("failed to read config file")?;       // .context() adds context to errors
    Ok(content)
}

// ============================================================================
// COMMON PATTERNS
// ============================================================================

// Creating errors
fn create_errors() -> Result<()> {
    bail!("something went wrong");                      // return Err(anyhow!(...))
    // ensure!(condition, "message");                   // bail if condition is false
    // return Err(anyhow!("formatted: {}", value));     // create ad-hoc error
}

// Chaining context
fn chain_context() -> Result<()> {
    let val: i32 = "not_a_number"
        .parse()
        .context("failed to parse value")?;             // adds human-readable context
    Ok(())
}

// Downcasting errors
fn downcast_example(err: anyhow::Error) {
    if let Some(io_err) = err.downcast_ref::<std::io::Error>() {
        // handle specific error type
        let _ = io_err;
    }
}
