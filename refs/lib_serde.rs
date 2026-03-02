// serde Reference — Serialization framework for Rust
// Cargo.toml: serde = { version = "1", features = ["derive"] }
// Usage: use serde::{Serialize, Deserialize};

use serde::{Serialize, Deserialize};

// ============================================================================
// BASIC USAGE
// ============================================================================

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    name: String,
    #[serde(default)]
    count: u32,                                             // defaults to 0 if missing
    #[serde(rename = "type")]
    kind: String,                                           // rename field in JSON
    #[serde(skip_serializing_if = "Option::is_none")]
    optional: Option<String>,                               // skip None values
}

// ============================================================================
// COMMON PATTERNS
// ============================================================================

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
enum Message {                                              // internally tagged enum
    Request { id: u32, method: String },
    Response { id: u32, result: String },
}

#[derive(Serialize, Deserialize)]
struct Wrapper {
    #[serde(flatten)]
    inner: std::collections::HashMap<String, String>,       // flatten nested struct
}
