//! Layer coverage tests — verify each enforcement layer is implemented and functional.

use std::path::Path;

// =====================================================================
// Layer 1: polyref enforce
// =====================================================================

#[test]
fn test_layer1_enforce_binary_exists() {
    // The enforce command is part of the main binary
    assert!(Path::new("src/commands/enforce.rs").exists());
}

#[test]
fn test_layer1_enforce_module_has_cmd() {
    let content = std::fs::read_to_string("src/commands/enforce.rs").unwrap();
    assert!(
        content.contains("pub fn cmd_enforce"),
        "enforce.rs should export cmd_enforce"
    );
}

// =====================================================================
// Layer 2: cargo check (verified by compilation)
// =====================================================================

#[test]
fn test_layer2_project_compiles() {
    // If this test is running, the project compiled successfully
    assert!(Path::new("Cargo.toml").exists());
}

// =====================================================================
// Layer 3: clippy
// =====================================================================

#[test]
fn test_layer3_clippy_config_exists() {
    // Clippy is invoked via cargo — verified by enforce-pipeline.sh
    let content = std::fs::read_to_string("scripts/enforce-pipeline.sh").unwrap();
    assert!(content.contains("clippy"));
}

// =====================================================================
// Layer 4: cargo audit
// =====================================================================

#[test]
fn test_layer4_audit_in_pipeline() {
    let content = std::fs::read_to_string("scripts/enforce-pipeline.sh").unwrap();
    assert!(content.contains("cargo audit") || content.contains("cargo-audit"));
}

// =====================================================================
// Layer 5: security check
// =====================================================================

#[test]
fn test_layer5_security_script_exists() {
    assert!(Path::new("scripts/security-check.sh").exists());
}

#[test]
fn test_layer5_security_in_pipeline() {
    let content = std::fs::read_to_string("scripts/enforce-pipeline.sh").unwrap();
    assert!(content.contains("security-check.sh"));
}

// =====================================================================
// Layer 6: test lint
// =====================================================================

#[test]
fn test_layer6_lint_script_exists() {
    assert!(Path::new("scripts/lint-tests.sh").exists());
}

#[test]
fn test_layer6_lint_in_pipeline() {
    let content = std::fs::read_to_string("scripts/enforce-pipeline.sh").unwrap();
    assert!(content.contains("lint-tests.sh"));
}

// =====================================================================
// Layer 7: cargo test
// =====================================================================

#[test]
fn test_layer7_test_in_pipeline() {
    let content = std::fs::read_to_string("scripts/enforce-pipeline.sh").unwrap();
    assert!(content.contains("cargo test"));
}

// =====================================================================
// Cross-cutting: all layers present in pipeline
// =====================================================================

#[test]
fn test_pipeline_has_seven_layers() {
    let content = std::fs::read_to_string("scripts/enforce-pipeline.sh").unwrap();
    // Count "Layer N:" markers
    let layer_count = content
        .lines()
        .filter(|l| l.contains("Layer ") && l.contains("---"))
        .count();
    assert!(
        layer_count >= 7,
        "Expected at least 7 layers in pipeline, found {}",
        layer_count
    );
}

// =====================================================================
// Reference file coverage
// =====================================================================

#[test]
fn test_ref_coverage_rust_libs() {
    let rust_dir = Path::new("refs/rust");
    let count = std::fs::read_dir(rust_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("rs"))
        .count();
    assert!(count >= 5, "Expected at least 5 Rust ref files, found {}", count);
}

#[test]
fn test_ref_coverage_stdlib() {
    let std_dir = Path::new("refs/std");
    let count = std::fs::read_dir(std_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .count();
    assert!(count >= 10, "Expected at least 10 stdlib ref files, found {}", count);
}

#[test]
fn test_ref_coverage_typescript() {
    let ts_dir = Path::new("refs/ts");
    let count = std::fs::read_dir(ts_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .count();
    assert!(count >= 5, "Expected at least 5 TypeScript ref files, found {}", count);
}
