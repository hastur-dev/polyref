use polyref::detect::{self, Dependency, Language};
use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures")
}

// ============================================================================
// Phase 2.1 — Rust Detection
// ============================================================================

#[test]
fn test_detect_rust_simple_deps() {
    let path = fixtures_dir().join("rust_project");
    let deps = detect::rust::detect_rust(&path).unwrap();
    // Should find: anyhow, serde, tokio from [dependencies]
    let dep_names: Vec<&str> = deps.iter().filter(|d| d.source_file == "Cargo.toml").map(|d| d.name.as_str()).collect();
    assert!(dep_names.contains(&"anyhow"));
    assert!(dep_names.contains(&"serde"));
    assert!(dep_names.contains(&"tokio"));
}

#[test]
fn test_detect_rust_dev_deps() {
    let path = fixtures_dir().join("rust_project");
    let deps = detect::rust::detect_rust(&path).unwrap();
    let dev_dep = deps.iter().find(|d| d.name == "tempfile");
    assert!(dev_dep.is_some());
}

#[test]
fn test_detect_rust_table_format() {
    let path = fixtures_dir().join("rust_project");
    let deps = detect::rust::detect_rust(&path).unwrap();
    let serde_dep = deps.iter().find(|d| d.name == "serde").unwrap();
    assert_eq!(serde_dep.version, "1");
    assert_eq!(serde_dep.language, Language::Rust);
}

#[test]
fn test_detect_rust_missing_cargo_toml() {
    let tmp = tempfile::tempdir().unwrap();
    let deps = detect::rust::detect_rust(tmp.path()).unwrap();
    assert!(deps.is_empty());
}

#[test]
fn test_detect_rust_invalid_toml() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(tmp.path().join("Cargo.toml"), "not valid {{ toml").unwrap();
    let result = detect::rust::detect_rust(tmp.path());
    assert!(result.is_err());
}

#[test]
fn test_detect_rust_no_deps_section() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    let deps = detect::rust::detect_rust(tmp.path()).unwrap();
    assert!(deps.is_empty());
}

// ============================================================================
// Phase 2.2 — Python Detection
// ============================================================================

#[test]
fn test_detect_python_requirements_txt() {
    let path = fixtures_dir().join("python_project");
    let deps = detect::python::detect_python(&path).unwrap();
    // From requirements.txt (after pyproject.toml which takes priority for shared names)
    let names: Vec<&str> = deps.iter().map(|d| d.name.as_str()).collect();
    assert!(names.contains(&"requests"));
    assert!(names.contains(&"flask"));
    assert!(names.contains(&"pandas"));
    assert!(names.contains(&"numpy"));
    assert!(names.contains(&"pytest"));
}

#[test]
fn test_detect_python_version_formats() {
    let path = fixtures_dir().join("python_project");
    let deps = detect::python::detect_python(&path).unwrap();

    // requirements.txt versions
    let pandas = deps.iter().find(|d| d.name == "pandas").unwrap();
    assert_eq!(pandas.version, "*");

    let numpy = deps.iter().find(|d| d.name == "numpy").unwrap();
    assert!(numpy.version.contains("~="));
}

#[test]
fn test_detect_python_pyproject_toml() {
    let path = fixtures_dir().join("python_project");
    let deps = detect::python::detect_python(&path).unwrap();
    // pyproject.toml has requests, flask, sqlalchemy
    let names: Vec<&str> = deps.iter().map(|d| d.name.as_str()).collect();
    assert!(names.contains(&"sqlalchemy"));
}

#[test]
fn test_detect_python_extras_stripped() {
    let path = fixtures_dir().join("python_project");
    let deps = detect::python::detect_python(&path).unwrap();
    // sqlalchemy[asyncio] should become just sqlalchemy
    let sa = deps.iter().find(|d| d.name == "sqlalchemy").unwrap();
    assert!(!sa.name.contains('['));
}

#[test]
fn test_detect_python_comments_ignored() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("requirements.txt"),
        "# This is a comment\nrequests==2.0\n\n# Another comment\n",
    )
    .unwrap();
    let deps = detect::python::detect_python(tmp.path()).unwrap();
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0].name, "requests");
}

#[test]
fn test_detect_python_missing_files() {
    let tmp = tempfile::tempdir().unwrap();
    let deps = detect::python::detect_python(tmp.path()).unwrap();
    assert!(deps.is_empty());
}

#[test]
fn test_detect_python_pipfile() {
    let tmp = tempfile::tempdir().unwrap();
    let pipfile = r#"
[packages]
requests = "==2.31.0"
flask = "*"

[dev-packages]
pytest = ">=7.0"
"#;
    std::fs::write(tmp.path().join("Pipfile"), pipfile).unwrap();
    let deps = detect::python::detect_python(tmp.path()).unwrap();
    let names: Vec<&str> = deps.iter().map(|d| d.name.as_str()).collect();
    assert!(names.contains(&"requests"));
    assert!(names.contains(&"flask"));
    assert!(names.contains(&"pytest"));
}

// ============================================================================
// Phase 2.3 — TypeScript Detection
// ============================================================================

#[test]
fn test_detect_ts_package_json() {
    let path = fixtures_dir().join("ts_project");
    let deps = detect::typescript::detect_typescript(&path).unwrap();
    let names: Vec<&str> = deps.iter().map(|d| d.name.as_str()).collect();
    assert!(names.contains(&"react"));
    assert!(names.contains(&"axios"));
    assert!(names.contains(&"lodash"));
    // devDeps (excluding @types)
    assert!(names.contains(&"typescript"));
    assert!(names.contains(&"vitest"));
}

#[test]
fn test_detect_ts_version_formats() {
    let path = fixtures_dir().join("ts_project");
    let deps = detect::typescript::detect_typescript(&path).unwrap();
    let react = deps.iter().find(|d| d.name == "react").unwrap();
    assert_eq!(react.version, "^18.2.0");
}

#[test]
fn test_detect_ts_types_filtered() {
    let path = fixtures_dir().join("ts_project");
    let deps = detect::typescript::detect_typescript(&path).unwrap();
    let type_deps: Vec<&Dependency> = deps.iter().filter(|d| d.name.starts_with("@types/")).collect();
    assert!(type_deps.is_empty(), "@types packages should be filtered out");
}

#[test]
fn test_detect_ts_tsconfig_exists() {
    let path = fixtures_dir().join("ts_project");
    assert!(path.join("tsconfig.json").exists());
}

#[test]
fn test_detect_ts_missing_package_json() {
    let tmp = tempfile::tempdir().unwrap();
    let deps = detect::typescript::detect_typescript(tmp.path()).unwrap();
    assert!(deps.is_empty());
}

#[test]
fn test_detect_ts_workspace_version() {
    let tmp = tempfile::tempdir().unwrap();
    let pkg = r#"{
  "name": "test",
  "version": "1.0.0",
  "dependencies": {
    "shared-lib": "workspace:*"
  }
}"#;
    std::fs::write(tmp.path().join("package.json"), pkg).unwrap();
    let deps = detect::typescript::detect_typescript(tmp.path()).unwrap();
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0].version, "workspace");
}

// ============================================================================
// Phase 2.4 — Unified Detection
// ============================================================================

#[test]
fn test_detect_unified_rust_only() {
    let path = fixtures_dir().join("rust_project");
    let detected = detect::detect(&path).unwrap();
    assert!(detected.languages.contains(&Language::Rust));
    assert!(!detected.languages.contains(&Language::Python));
}

#[test]
fn test_detect_unified_python_only() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("requirements.txt"),
        "requests==2.0\n",
    )
    .unwrap();
    let detected = detect::detect(tmp.path()).unwrap();
    assert!(detected.languages.contains(&Language::Python));
    assert!(!detected.languages.contains(&Language::Rust));
}

#[test]
fn test_detect_unified_ts_only() {
    let path = fixtures_dir().join("ts_project");
    let detected = detect::detect(&path).unwrap();
    assert!(detected.languages.contains(&Language::TypeScript));
}

#[test]
fn test_detect_unified_multi_language() {
    let tmp = tempfile::tempdir().unwrap();
    // Create Cargo.toml
    std::fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nanyhow = \"1\"\n",
    )
    .unwrap();
    // Create requirements.txt
    std::fs::write(tmp.path().join("requirements.txt"), "flask==3.0\n").unwrap();
    // Create package.json
    std::fs::write(
        tmp.path().join("package.json"),
        r#"{"name": "t", "version": "1.0.0", "dependencies": {"react": "^18.0.0"}}"#,
    )
    .unwrap();

    let detected = detect::detect(tmp.path()).unwrap();
    assert!(detected.languages.contains(&Language::Rust));
    assert!(detected.languages.contains(&Language::Python));
    assert!(detected.languages.contains(&Language::TypeScript));
}

#[test]
fn test_detect_unified_empty_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let detected = detect::detect(tmp.path()).unwrap();
    assert!(detected.languages.is_empty());
    assert!(detected.dependencies.is_empty());
}

#[test]
fn test_detect_unified_skip_libraries() {
    let tmp = tempfile::tempdir().unwrap();
    std::fs::write(
        tmp.path().join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\nanyhow = \"1\"\nserde = \"1\"\n",
    )
    .unwrap();

    let detected = detect::detect_with_skip(tmp.path(), &["serde".to_string()]).unwrap();
    let names: Vec<&str> = detected.dependencies.iter().map(|d| d.name.as_str()).collect();
    assert!(names.contains(&"anyhow"));
    assert!(!names.contains(&"serde"));
}

#[test]
fn test_detect_unified_deduplication() {
    let tmp = tempfile::tempdir().unwrap();
    // Both pyproject.toml and requirements.txt have requests
    std::fs::write(tmp.path().join("requirements.txt"), "requests==2.31\n").unwrap();
    std::fs::write(
        tmp.path().join("pyproject.toml"),
        "[project]\nname = \"t\"\nversion = \"0.1.0\"\ndependencies = [\"requests>=2.31\"]\n",
    )
    .unwrap();

    let detected = detect::detect(tmp.path()).unwrap();
    let request_deps: Vec<&Dependency> = detected.dependencies.iter().filter(|d| d.name == "requests").collect();
    assert_eq!(request_deps.len(), 1, "requests should appear only once after dedup");
}
