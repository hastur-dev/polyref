use polyref_drift::config::DriftConfig;
use std::io::Write;

#[test]
fn test_default_config() {
    let config = DriftConfig::default();
    assert_eq!(config.refs_dirs, vec!["refs".to_string()]);
    assert_eq!(config.max_age_days, 30);
    assert_eq!(config.output, "terminal");
    assert!(config.skip.is_empty());
    assert!(config.http_proxy.is_none());
    assert!(config.registries.crates_io);
    assert!(config.registries.pypi);
    assert!(config.registries.npm);
}

#[test]
fn test_validate_valid_config() {
    let config = DriftConfig::default();
    assert!(config.validate().is_ok());
}

#[test]
fn test_validate_empty_refs_dirs() {
    let mut config = DriftConfig::default();
    config.refs_dirs = vec![];
    let err = config.validate().unwrap_err();
    assert!(err.to_string().contains("refs_dirs must not be empty"));
}

#[test]
fn test_validate_zero_max_age() {
    let mut config = DriftConfig::default();
    config.max_age_days = 0;
    let err = config.validate().unwrap_err();
    assert!(err.to_string().contains("max_age_days must be > 0"));
}

#[test]
fn test_validate_invalid_output() {
    let mut config = DriftConfig::default();
    config.output = "xml".to_string();
    let err = config.validate().unwrap_err();
    assert!(err.to_string().contains("output must be"));
}

#[test]
fn test_validate_json_output() {
    let mut config = DriftConfig::default();
    config.output = "json".to_string();
    assert!(config.validate().is_ok());
}

#[test]
fn test_should_skip() {
    let mut config = DriftConfig::default();
    config.skip = vec!["serde".to_string(), "tokio".to_string()];
    assert!(config.should_skip("serde"));
    assert!(config.should_skip("tokio"));
    assert!(!config.should_skip("anyhow"));
}

#[test]
fn test_load_from_toml() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("drift-config.toml");
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(
        f,
        r#"
refs_dirs = ["refs", "extra-refs"]
max_age_days = 14
output = "json"
skip = ["legacy-crate"]

[registries]
crates_io = true
pypi = false
npm = true
"#
    )
    .unwrap();

    let config = DriftConfig::load(&path).unwrap();
    assert_eq!(config.refs_dirs, vec!["refs", "extra-refs"]);
    assert_eq!(config.max_age_days, 14);
    assert_eq!(config.output, "json");
    assert_eq!(config.skip, vec!["legacy-crate"]);
    assert!(config.registries.crates_io);
    assert!(!config.registries.pypi);
    assert!(config.registries.npm);
}

#[test]
fn test_load_or_default_missing_file() {
    let path = std::path::Path::new("/nonexistent/drift-config.toml");
    let config = DriftConfig::load_or_default(path);
    // Should fallback to defaults
    assert_eq!(config.refs_dirs, vec!["refs".to_string()]);
    assert_eq!(config.max_age_days, 30);
}
