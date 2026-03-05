use polyref::typescript_bridge;
use std::path::Path;

const TS_CHECKER_DIR: &str = "typescript/polyref_ts";

#[test]
fn test_is_ts_checker_available_when_not_built() {
    // If dist/cli.js doesn't exist, should return false
    let nonexistent = Path::new("/tmp/nonexistent_checker");
    assert!(!typescript_bridge::is_ts_checker_available(nonexistent));
}

#[test]
fn test_run_ts_checker_fails_when_not_built() {
    let nonexistent = Path::new("/tmp/nonexistent_checker");
    let result = typescript_bridge::run_ts_checker(
        "test.ts", "refs", nonexistent, false, "human",
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not built"));
}

#[test]
fn test_run_ts_checker_stdin_fails_when_not_built() {
    let nonexistent = Path::new("/tmp/nonexistent_checker");
    let result = typescript_bridge::run_ts_checker_stdin(
        "const x = 1;", "refs", nonexistent, false, "human",
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not built"));
}

#[test]
fn test_ts_checker_dir_detection() {
    let ts_dir = Path::new(TS_CHECKER_DIR);
    // This just verifies the function works without panicking
    let available = typescript_bridge::is_ts_checker_available(ts_dir);
    // May or may not be built, that's fine — we're testing the detection logic
    if available {
        println!("TS checker is built at {}", ts_dir.display());
    } else {
        println!("TS checker not built yet (expected in dev)");
    }
}
