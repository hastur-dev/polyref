//! Enforcement pipeline tests — verify that the project compiles, passes clippy,
//! and all tool crates are healthy. These are slow (spawn cargo) so marked #[ignore].

use std::process::Command;

fn cargo_command(args: &[&str], cwd: &str) -> std::process::Output {
    Command::new("cargo")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("failed to run cargo")
}

#[test]
#[ignore]
fn test_main_crate_compiles() {
    let output = cargo_command(&["build"], ".");
    assert!(
        output.status.success(),
        "Main crate failed to compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore]
fn test_main_crate_clippy_clean() {
    let output = cargo_command(&["clippy", "--", "-D", "warnings"], ".");
    assert!(
        output.status.success(),
        "Main crate has clippy warnings:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore]
fn test_drift_crate_compiles() {
    let output = cargo_command(&["build"], "tools/polyref-drift");
    assert!(
        output.status.success(),
        "polyref-drift failed to compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore]
fn test_gen_crate_compiles() {
    let output = cargo_command(&["build"], "tools/polyref-gen");
    assert!(
        output.status.success(),
        "polyref-gen failed to compile:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
#[ignore]
fn test_all_tests_pass() {
    // Use --lib to test only the library, avoiding recompilation of this test binary
    // (which causes Windows file locking issues)
    let output = cargo_command(&["test", "--lib"], ".");
    assert!(
        output.status.success(),
        "Library tests failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
