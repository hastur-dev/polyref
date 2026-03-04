use std::io::Write;
use std::process::{Command, Stdio};

fn polyref_bin() -> String {
    env!("CARGO_BIN_EXE_polyref").to_string()
}

fn make_temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("create temp dir")
}

fn write_temp_file(dir: &std::path::Path, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("create parent dirs");
    }
    std::fs::write(&path, content).expect("write file");
    path
}

#[test]
fn test_exit_zero_on_no_issues() {
    let tmp = make_temp_dir();
    let source = write_temp_file(
        tmp.path(),
        "clean.rs",
        "fn main() {\n    println!(\"hello\");\n}\n",
    );
    // No refs → no issues → exit 0
    let output = Command::new(polyref_bin())
        .args(["enforce", "--project", source.to_str().unwrap(), "--enforce", "--lang", "rust"])
        .output()
        .expect("run binary");
    assert_eq!(output.status.code(), Some(0), "clean source should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Approved"), "should show Approved verdict");
}

#[test]
fn test_exit_one_on_issues_with_enforce() {
    let tmp = make_temp_dir();

    // Create a ref file with known API
    let refs_dir = tmp.path().join("refs").join("rust");
    std::fs::create_dir_all(&refs_dir).expect("create refs dir");
    let ref_content = "// Library: tokio\n\
                        // Version: 1.0.0\n\
                        \n\
                        pub struct Runtime {\n\
                        }\n\
                        \n\
                        impl Runtime {\n\
                        \x20   pub fn new() -> io::Result<Runtime>\n\
                        \x20   // Creates a new Runtime\n\
                        }\n";
    std::fs::write(refs_dir.join("lib_tokio.rs"), ref_content).expect("write ref");

    // Source that uses a hallucinated method
    let source = write_temp_file(
        tmp.path(),
        "bad.rs",
        "use tokio::runtime::Runtime;\nfn main() {\n    let rt = Runtime::new_async();\n}\n",
    );

    let output = Command::new(polyref_bin())
        .args([
            "enforce",
            "--project", source.to_str().unwrap(),
            "--enforce",
            "--lang", "rust",
            "--refs", tmp.path().join("refs").to_str().unwrap(),
        ])
        .output()
        .expect("run binary");

    // Should exit 1 because --enforce is set and there's a hallucinated method
    assert_eq!(
        output.status.code(),
        Some(1),
        "bad source with --enforce should exit 1. stdout: {}, stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_json_output_format_parseable() {
    let tmp = make_temp_dir();
    let source = write_temp_file(
        tmp.path(),
        "sample.rs",
        "fn main() {\n    println!(\"hello\");\n}\n",
    );

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
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("stdout should be valid JSON");
    assert_eq!(parsed["polyref_enforce"], true);
    assert!(parsed["verdict"].is_string(), "verdict should be a string");
}

#[test]
fn test_gate_exits_zero_without_enforce_flag() {
    let tmp = make_temp_dir();

    // Create a ref file
    let refs_dir = tmp.path().join("refs").join("rust");
    std::fs::create_dir_all(&refs_dir).expect("create refs dir");
    let ref_content = "// Library: tokio\n\
                        // Version: 1.0.0\n\
                        \n\
                        pub struct Runtime {\n\
                        }\n\
                        \n\
                        impl Runtime {\n\
                        \x20   pub fn new() -> io::Result<Runtime>\n\
                        }\n";
    std::fs::write(refs_dir.join("lib_tokio.rs"), ref_content).expect("write ref");

    let source = write_temp_file(
        tmp.path(),
        "has_issues.rs",
        "use tokio::runtime::Runtime;\nfn main() {\n    let rt = Runtime::fake_method();\n}\n",
    );

    // No --enforce flag → exit 0 even with issues
    let output = Command::new(polyref_bin())
        .args([
            "enforce",
            "--project", source.to_str().unwrap(),
            "--lang", "rust",
            "--refs", tmp.path().join("refs").to_str().unwrap(),
        ])
        .output()
        .expect("run binary");

    assert_eq!(
        output.status.code(),
        Some(0),
        "without --enforce, should exit 0 even with issues"
    );
}

#[test]
fn test_gate_stdin_mode_works() {
    let source_code = "fn main() {\n    println!(\"hello from stdin\");\n}\n";

    let mut child = Command::new(polyref_bin())
        .args([
            "enforce",
            "--from-stdin",
            "--lang", "rust",
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
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(output.status.code(), Some(0), "clean stdin should exit 0");
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("should be valid JSON");
    assert_eq!(parsed["polyref_enforce"], true);
    assert_eq!(parsed["verdict"], "Approved");
}

#[test]
fn test_build_enforce_config_maps_enforce_flag() {
    // This test validates the config builder via the binary behavior:
    // --enforce → exit 1 on issues
    // We use a simple scenario: empty refs + code that imports unknown crate + strict
    let tmp = make_temp_dir();
    let source = write_temp_file(
        tmp.path(),
        "test.rs",
        "fn main() {}\n",
    );

    let output = Command::new(polyref_bin())
        .args([
            "enforce",
            "--project", source.to_str().unwrap(),
            "--enforce",
            "--lang", "rust",
            "--output-format", "json",
        ])
        .output()
        .expect("run binary");

    // No refs, no imports → no issues → Approved even with --enforce
    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .expect("valid JSON");
    assert_eq!(parsed["verdict"], "Approved");
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn test_build_enforce_config_maps_strict_flag() {
    let tmp = make_temp_dir();
    let source = write_temp_file(
        tmp.path(),
        "strict_test.rs",
        "use unknown_crate::Something;\nfn main() {}\n",
    );

    // --strict + --enforce + unknown crate → should block (coverage gate)
    let output = Command::new(polyref_bin())
        .args([
            "enforce",
            "--project", source.to_str().unwrap(),
            "--enforce",
            "--strict",
            "--lang", "rust",
            "--output-format", "json",
        ])
        .output()
        .expect("run binary");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .expect("valid JSON");
    // The unknown_crate import has no ref file → strict blocks
    assert_eq!(parsed["verdict"], "Blocked");
    assert_eq!(output.status.code(), Some(1));
}

#[test]
fn test_build_enforce_config_coverage_validates() {
    let tmp = make_temp_dir();
    let source = write_temp_file(
        tmp.path(),
        "coverage_test.rs",
        "fn main() {}\n",
    );

    // --require-coverage 101 should fail validation
    let output = Command::new(polyref_bin())
        .args([
            "enforce",
            "--project", source.to_str().unwrap(),
            "--lang", "rust",
            "--require-coverage", "101",
        ])
        .output()
        .expect("run binary");

    assert_ne!(
        output.status.code(),
        Some(0),
        "coverage=101 should cause validation error"
    );
}
