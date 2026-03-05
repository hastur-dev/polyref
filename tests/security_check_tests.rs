//! Security check tests — verify no obvious security issues in production code.

use std::path::Path;

/// Verify no unsafe blocks in main crate src/.
#[test]
fn test_no_unsafe_in_src() {
    let mut unsafe_locations = Vec::new();
    visit_rs_files(Path::new("src"), &mut |path, content| {
        for (i, line) in content.lines().enumerate() {
            if line.contains("unsafe {") || line.contains("unsafe fn ") {
                unsafe_locations.push(format!("{}:{}", path.display(), i + 1));
            }
        }
    });
    assert!(
        unsafe_locations.is_empty(),
        "Found unsafe code in src/: {:?}",
        unsafe_locations
    );
}

/// Verify no hardcoded secrets in production code.
#[test]
fn test_no_hardcoded_secrets() {
    let suspicious_patterns = [
        "password = \"",
        "secret = \"",
        "api_key = \"",
        "token = \"",
        "AWS_SECRET",
        "PRIVATE_KEY",
    ];
    let mut findings = Vec::new();
    visit_rs_files(Path::new("src"), &mut |path, content| {
        for (i, line) in content.lines().enumerate() {
            let lower = line.to_lowercase();
            for pattern in &suspicious_patterns {
                if lower.contains(&pattern.to_lowercase())
                    && !line.trim().starts_with("//")
                    && !line.contains("test")
                {
                    findings.push(format!("{}:{}: {}", path.display(), i + 1, line.trim()));
                }
            }
        }
    });
    assert!(
        findings.is_empty(),
        "Possible hardcoded secrets: {:?}",
        findings
    );
}

/// Verify the security check script exists.
#[test]
fn test_security_script_exists() {
    assert!(
        Path::new("scripts/security-check.sh").exists(),
        "scripts/security-check.sh should exist"
    );
}

fn visit_rs_files(dir: &Path, cb: &mut dyn FnMut(&Path, &str)) {
    if !dir.exists() {
        return;
    }
    for entry in std::fs::read_dir(dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            visit_rs_files(&path, cb);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            cb(&path, &content);
        }
    }
}
