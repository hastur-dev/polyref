use polyref_drift::config::DriftConfig;
use polyref_drift::drift_detector::{detect_drift, extract_lib_name, extract_version, detect_registry};
use std::io::Write;
use std::path::Path;

// =====================================================================
// extract_lib_name tests
// =====================================================================

#[test]
fn test_extract_lib_name_with_prefix() {
    let path = Path::new("/refs/rust/lib_serde.rs");
    assert_eq!(extract_lib_name(path), "serde");
}

#[test]
fn test_extract_lib_name_without_prefix() {
    let path = Path::new("/refs/rust/tokio.rs");
    assert_eq!(extract_lib_name(path), "tokio");
}

#[test]
fn test_extract_lib_name_polyref_ext() {
    let path = Path::new("/refs/ts/express.polyref");
    assert_eq!(extract_lib_name(path), "express");
}

// =====================================================================
// extract_version tests
// =====================================================================

#[test]
fn test_extract_version_rust_comment() {
    let content = "// Version: 1.0.219\n// some other content\n";
    assert_eq!(extract_version(content), "1.0.219");
}

#[test]
fn test_extract_version_hash_comment() {
    let content = "# Version: 2.31.0\n# Module: requests\n";
    assert_eq!(extract_version(content), "2.31.0");
}

#[test]
fn test_extract_version_missing() {
    let content = "// No version info here\nfn foo() {}\n";
    assert_eq!(extract_version(content), "unknown");
}

#[test]
fn test_extract_version_empty() {
    assert_eq!(extract_version(""), "unknown");
}

#[test]
fn test_extract_version_deep_in_file() {
    // Version beyond first 10 lines should not be found
    let mut content = String::new();
    for i in 0..15 {
        content.push_str(&format!("// Line {}\n", i));
    }
    content.push_str("// Version: 1.0.0\n");
    assert_eq!(extract_version(&content), "unknown");
}

// =====================================================================
// detect_registry tests
// =====================================================================

#[test]
fn test_detect_registry_rust_hint() {
    let path = Path::new("/refs/rust/serde.rs");
    assert_eq!(detect_registry("rust", path), "crates.io");
}

#[test]
fn test_detect_registry_rs_extension() {
    let path = Path::new("/refs/auto/tokio.rs");
    assert_eq!(detect_registry("auto", path), "crates.io");
}

#[test]
fn test_detect_registry_ts_hint() {
    let path = Path::new("/refs/ts/express.polyref");
    assert_eq!(detect_registry("ts", path), "npm");
}

#[test]
fn test_detect_registry_polyref_with_lang_python() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("requests.polyref");
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(f, "@lang python\n@module requests").unwrap();

    assert_eq!(detect_registry("auto", &path), "pypi");
}

#[test]
fn test_detect_registry_polyref_with_lang_typescript() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("axios.polyref");
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(f, "@lang typescript\n@module axios").unwrap();

    assert_eq!(detect_registry("auto", &path), "npm");
}

#[test]
fn test_detect_registry_unknown() {
    let path = Path::new("/refs/auto/something.txt");
    assert_eq!(detect_registry("auto", path), "unknown");
}

// =====================================================================
// detect_drift integration tests
// =====================================================================

#[test]
fn test_detect_drift_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let config = DriftConfig {
        refs_dirs: vec![dir.path().to_string_lossy().to_string()],
        ..DriftConfig::default()
    };
    let results = detect_drift(&config).unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_detect_drift_nonexistent_dir() {
    let config = DriftConfig {
        refs_dirs: vec!["/nonexistent/path/to/refs".to_string()],
        ..DriftConfig::default()
    };
    let results = detect_drift(&config).unwrap();
    assert!(results.is_empty());
}

#[test]
fn test_detect_drift_skips_stdlib() {
    let dir = tempfile::tempdir().unwrap();
    let rust_dir = dir.path().join("rust");
    std::fs::create_dir_all(&rust_dir).unwrap();
    // Create a stdlib ref file (should be skipped)
    let mut f = std::fs::File::create(rust_dir.join("std_collections.rs")).unwrap();
    writeln!(f, "// Version: 1.0.0\nimpl Vec<T> {{}}").unwrap();

    let config = DriftConfig {
        refs_dirs: vec![dir.path().to_string_lossy().to_string()],
        ..DriftConfig::default()
    };
    let results = detect_drift(&config).unwrap();
    // std_ prefixed files should be skipped
    assert!(results.iter().all(|r| !r.library_name.starts_with("std_")));
}

#[test]
fn test_detect_drift_skips_configured_libs() {
    let dir = tempfile::tempdir().unwrap();
    let rust_dir = dir.path().join("rust");
    std::fs::create_dir_all(&rust_dir).unwrap();
    let mut f = std::fs::File::create(rust_dir.join("serde.rs")).unwrap();
    writeln!(f, "// Version: 1.0.0").unwrap();
    let mut f2 = std::fs::File::create(rust_dir.join("tokio.rs")).unwrap();
    writeln!(f2, "// Version: 1.0.0").unwrap();

    let config = DriftConfig {
        refs_dirs: vec![dir.path().to_string_lossy().to_string()],
        skip: vec!["serde".to_string()],
        ..DriftConfig::default()
    };
    let results = detect_drift(&config).unwrap();
    // serde should be skipped, tokio should remain
    assert!(results.iter().all(|r| r.library_name != "serde"));
    assert!(results.iter().any(|r| r.library_name == "tokio"));
}

#[test]
fn test_detect_drift_scans_rust_subdir() {
    let dir = tempfile::tempdir().unwrap();
    let rust_dir = dir.path().join("rust");
    std::fs::create_dir_all(&rust_dir).unwrap();
    let mut f = std::fs::File::create(rust_dir.join("anyhow.rs")).unwrap();
    writeln!(f, "// Version: 1.0.0").unwrap();

    let config = DriftConfig {
        refs_dirs: vec![dir.path().to_string_lossy().to_string()],
        ..DriftConfig::default()
    };
    let results = detect_drift(&config).unwrap();
    // Should find anyhow (will have error since no real registry, but it should be in results)
    assert!(results.iter().any(|r| r.library_name == "anyhow"));
}
