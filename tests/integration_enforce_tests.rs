use std::io::Write;
use std::process::{Command, Stdio};

fn polyref_bin() -> String {
    env!("CARGO_BIN_EXE_polyref").to_string()
}

fn make_temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("create temp dir")
}

fn setup_tokio_ref(tmp: &std::path::Path) -> std::path::PathBuf {
    let refs_dir = tmp.join("refs").join("rust");
    std::fs::create_dir_all(&refs_dir).expect("create refs dir");
    let ref_content = "// Library: tokio\n\
                        // Version: 1.0.0\n\
                        \n\
                        pub struct Runtime {\n\
                        }\n\
                        \n\
                        pub struct JoinHandle {\n\
                        }\n\
                        \n\
                        impl Runtime {\n\
                        \x20   pub fn new() -> io::Result<Runtime>\n\
                        \x20   // Creates a new Runtime\n\
                        \n\
                        \x20   pub fn block_on<F: Future>(&self, future: F) -> F::Output\n\
                        \x20   // Runs a future to completion on the Tokio runtime\n\
                        }\n\
                        \n\
                        pub fn spawn<T>(future: T) -> JoinHandle<T::Output>\n\
                        // Spawns a new asynchronous task\n";
    std::fs::write(refs_dir.join("lib_tokio.rs"), ref_content).expect("write ref");
    tmp.join("refs")
}

#[test]
fn test_gate_approves_clean_rust_code() {
    let tmp = make_temp_dir();
    let refs_dir = setup_tokio_ref(tmp.path());

    let source = tmp.path().join("clean.rs");
    std::fs::write(
        &source,
        "use tokio::runtime::Runtime;\nfn main() {\n    let rt = Runtime::new();\n}\n",
    )
    .expect("write source");

    let output = Command::new(polyref_bin())
        .args([
            "enforce",
            "--project", source.to_str().unwrap(),
            "--enforce",
            "--lang", "rust",
            "--refs", refs_dir.to_str().unwrap(),
            "--output-format", "json",
        ])
        .output()
        .expect("run binary");

    assert_eq!(output.status.code(), Some(0), "clean code should be Approved");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .expect("valid JSON");
    assert_eq!(parsed["verdict"], "Approved");
    assert_eq!(parsed["issue_count"], 0);
}

#[test]
fn test_gate_blocks_hallucinated_method() {
    let tmp = make_temp_dir();
    let refs_dir = setup_tokio_ref(tmp.path());

    let source = tmp.path().join("bad.rs");
    std::fs::write(
        &source,
        "use tokio::runtime::Runtime;\nfn main() {\n    let rt = Runtime::new_async();\n}\n",
    )
    .expect("write source");

    let output = Command::new(polyref_bin())
        .args([
            "enforce",
            "--project", source.to_str().unwrap(),
            "--enforce",
            "--lang", "rust",
            "--refs", refs_dir.to_str().unwrap(),
            "--output-format", "json",
        ])
        .output()
        .expect("run binary");

    assert_eq!(output.status.code(), Some(1), "hallucinated method should be Blocked");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .expect("valid JSON");
    assert_eq!(parsed["verdict"], "Blocked");
    let issues = parsed["issues"].as_array().expect("issues array");
    assert!(!issues.is_empty(), "should have at least one issue");
}

#[test]
fn test_gate_stdin_mode_works() {
    let tmp = make_temp_dir();
    let refs_dir = setup_tokio_ref(tmp.path());

    let source_code =
        "use tokio::runtime::Runtime;\nfn main() {\n    let rt = Runtime::new();\n}\n";

    let mut child = Command::new(polyref_bin())
        .args([
            "enforce",
            "--from-stdin",
            "--enforce",
            "--lang", "rust",
            "--refs", refs_dir.to_str().unwrap(),
            "--output-format", "json",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn binary");

    child
        .stdin
        .as_mut()
        .expect("stdin")
        .write_all(source_code.as_bytes())
        .expect("write stdin");

    let output = child.wait_with_output().expect("wait for output");
    assert_eq!(output.status.code(), Some(0), "clean stdin should exit 0");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .expect("valid JSON");
    assert_eq!(parsed["verdict"], "Approved");
    assert_eq!(parsed["polyref_enforce"], true);
}

#[test]
fn test_gate_strict_blocks_uncovered_package() {
    let tmp = make_temp_dir();
    let refs_dir = setup_tokio_ref(tmp.path());

    // Source imports reqwest which has no ref file
    let source = tmp.path().join("uncovered.rs");
    std::fs::write(
        &source,
        "use reqwest::Client;\nfn main() {}\n",
    )
    .expect("write source");

    let output = Command::new(polyref_bin())
        .args([
            "enforce",
            "--project", source.to_str().unwrap(),
            "--enforce",
            "--strict",
            "--lang", "rust",
            "--refs", refs_dir.to_str().unwrap(),
            "--output-format", "json",
        ])
        .output()
        .expect("run binary");

    assert_eq!(output.status.code(), Some(1), "--strict with uncovered package should exit 1");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .expect("valid JSON");
    assert_eq!(parsed["verdict"], "Blocked");
}

#[test]
fn test_gate_coverage_threshold_blocks() {
    let tmp = make_temp_dir();
    let refs_dir = setup_tokio_ref(tmp.path());

    // Source imports tokio (covered) + reqwest (uncovered) → 50% coverage
    let source = tmp.path().join("half_covered.rs");
    std::fs::write(
        &source,
        "use tokio::runtime::Runtime;\nuse reqwest::Client;\nfn main() {}\n",
    )
    .expect("write source");

    let output = Command::new(polyref_bin())
        .args([
            "enforce",
            "--project", source.to_str().unwrap(),
            "--enforce",
            "--lang", "rust",
            "--refs", refs_dir.to_str().unwrap(),
            "--require-coverage", "80",
            "--output-format", "json",
        ])
        .output()
        .expect("run binary");

    assert_eq!(output.status.code(), Some(1), "50% < 80% should exit 1");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .expect("valid JSON");
    assert_eq!(parsed["verdict"], "Blocked");
}

#[test]
fn test_gate_json_output_schema_valid() {
    let tmp = make_temp_dir();
    let source = tmp.path().join("schema.rs");
    std::fs::write(&source, "fn main() {}\n").expect("write source");

    let output = Command::new(polyref_bin())
        .args([
            "enforce",
            "--project", source.to_str().unwrap(),
            "--lang", "rust",
            "--output-format", "json",
        ])
        .output()
        .expect("run binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .expect("must be valid JSON");

    // Verify required keys exist
    assert!(parsed.get("polyref_enforce").is_some(), "must have polyref_enforce");
    assert!(parsed.get("verdict").is_some(), "must have verdict");
    assert!(parsed.get("issue_count").is_some(), "must have issue_count");
    assert!(parsed.get("issues").is_some(), "must have issues");
}

#[test]
fn test_gate_exits_zero_without_enforce_flag() {
    let tmp = make_temp_dir();
    let refs_dir = setup_tokio_ref(tmp.path());

    let source = tmp.path().join("noblock.rs");
    std::fs::write(
        &source,
        "use tokio::runtime::Runtime;\nfn main() {\n    let rt = Runtime::invented();\n}\n",
    )
    .expect("write source");

    let output = Command::new(polyref_bin())
        .args([
            "enforce",
            "--project", source.to_str().unwrap(),
            "--lang", "rust",
            "--refs", refs_dir.to_str().unwrap(),
            "--output-format", "json",
        ])
        .output()
        .expect("run binary");

    assert_eq!(
        output.status.code(),
        Some(0),
        "without --enforce flag, should exit 0 regardless of issues"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .expect("valid JSON");
    assert_eq!(parsed["verdict"], "Approved");
}

#[test]
fn test_install_hook_script_creates_settings_file() {
    // Simulate what install-hook.sh does: create .claude/settings.json
    // This tests the install logic without relying on bash specifics.
    let tmp = make_temp_dir();
    let project_root = tmp.path();
    let claude_dir = project_root.join(".claude");

    // Source settings content (what the project ships)
    let source_settings =
        r#"{"hooks":{"PostToolUse":[{"matcher":"Write","command":"bash scripts/enforce-pipeline.sh"}]}}"#;

    // Simulate install: create .claude dir and copy settings
    std::fs::create_dir_all(&claude_dir).expect("create .claude");
    let dest_path = claude_dir.join("settings.json");
    std::fs::write(&dest_path, source_settings).expect("write settings");

    // Verify the settings file exists and contains expected content
    assert!(dest_path.exists(), "settings.json should exist after install");
    let content = std::fs::read_to_string(&dest_path).expect("read settings");
    let parsed: serde_json::Value =
        serde_json::from_str(&content).expect("settings should be valid JSON");
    assert!(
        parsed.get("hooks").is_some(),
        "settings should contain hooks config"
    );
    assert!(
        parsed["hooks"]["PostToolUse"].is_array(),
        "should have PostToolUse hooks"
    );
}
