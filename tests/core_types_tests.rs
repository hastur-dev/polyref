use polyref::check::{Issue, Severity, ValidationResult};
use polyref::config::{Config, OutputFormat};
use polyref::detect::{Dependency, DetectedProject, Language};
use polyref::generate::EntryKind;
use std::path::PathBuf;

// ============================================================================
// Phase 1.2 — Core type tests
// ============================================================================

#[test]
fn test_language_enum_variants() {
    let rust = Language::Rust;
    let python = Language::Python;
    let ts = Language::TypeScript;
    assert_ne!(rust, python);
    assert_ne!(python, ts);
    assert_ne!(rust, ts);
}

#[test]
fn test_dependency_creation() {
    let dep = Dependency {
        name: "serde".to_string(),
        version: "1.0".to_string(),
        language: Language::Rust,
        source_file: "Cargo.toml".to_string(),
    };
    assert_eq!(dep.name, "serde");
    assert_eq!(dep.version, "1.0");
    assert_eq!(dep.language, Language::Rust);
    assert_eq!(dep.source_file, "Cargo.toml");
}

#[test]
fn test_detected_project_creation() {
    let project = DetectedProject {
        root: PathBuf::from("/test"),
        languages: vec![Language::Rust, Language::Python, Language::TypeScript],
        dependencies: vec![
            Dependency {
                name: "anyhow".to_string(),
                version: "1".to_string(),
                language: Language::Rust,
                source_file: "Cargo.toml".to_string(),
            },
            Dependency {
                name: "requests".to_string(),
                version: "2.31".to_string(),
                language: Language::Python,
                source_file: "requirements.txt".to_string(),
            },
        ],
        manifest_files: vec![PathBuf::from("Cargo.toml"), PathBuf::from("requirements.txt")],
    };
    assert_eq!(project.languages.len(), 3);
    assert_eq!(project.dependencies.len(), 2);
}

#[test]
fn test_severity_ordering() {
    assert!(Severity::Error > Severity::Warning);
    assert!(Severity::Warning > Severity::Info);
    assert!(Severity::Error > Severity::Info);
}

#[test]
fn test_issue_creation() {
    let issue = Issue {
        severity: Severity::Error,
        message: "unknown import".to_string(),
        file: PathBuf::from("main.rs"),
        line: 5,
        column: Some(10),
        code_snippet: "use anyhow::Foo;".to_string(),
        suggestion: Some("did you mean 'Result'?".to_string()),
        rule: "unknown-import".to_string(),
    };
    assert_eq!(issue.severity, Severity::Error);
    assert_eq!(issue.line, 5);
    assert!(issue.suggestion.is_some());
    assert_eq!(issue.rule, "unknown-import");
}

#[test]
fn test_validation_result_counts() {
    let result = ValidationResult {
        language: Language::Python,
        files_checked: 3,
        issues: vec![
            Issue {
                severity: Severity::Error,
                message: "err1".to_string(),
                file: PathBuf::from("a.py"),
                line: 1,
                column: None,
                code_snippet: "".to_string(),
                suggestion: None,
                rule: "unknown-import".to_string(),
            },
            Issue {
                severity: Severity::Warning,
                message: "warn1".to_string(),
                file: PathBuf::from("a.py"),
                line: 2,
                column: None,
                code_snippet: "".to_string(),
                suggestion: None,
                rule: "missing-arg".to_string(),
            },
            Issue {
                severity: Severity::Error,
                message: "err2".to_string(),
                file: PathBuf::from("b.py"),
                line: 1,
                column: None,
                code_snippet: "".to_string(),
                suggestion: None,
                rule: "unknown-function".to_string(),
            },
        ],
    };
    assert_eq!(result.error_count(), 2);
    assert_eq!(result.warning_count(), 1);
    assert!(!result.is_clean());
}

#[test]
fn test_reference_entry_kinds() {
    let kinds = vec![
        EntryKind::Function,
        EntryKind::Method,
        EntryKind::Class,
        EntryKind::Struct,
        EntryKind::Trait,
        EntryKind::Interface,
        EntryKind::TypeAlias,
        EntryKind::Enum,
        EntryKind::Constant,
        EntryKind::Decorator,
        EntryKind::Macro,
        EntryKind::Hook,
        EntryKind::Component,
        EntryKind::Property,
        EntryKind::Module,
    ];
    // All variants are distinct
    for (i, a) in kinds.iter().enumerate() {
        for (j, b) in kinds.iter().enumerate() {
            if i != j {
                assert_ne!(a, b);
            }
        }
    }
}

#[test]
fn test_validation_result_empty_is_clean() {
    let result = ValidationResult {
        language: Language::Rust,
        files_checked: 5,
        issues: vec![],
    };
    assert!(result.is_clean());
    assert_eq!(result.error_count(), 0);
    assert_eq!(result.warning_count(), 0);
}

// ============================================================================
// Phase 1.3 — Config tests
// ============================================================================

#[test]
fn test_config_default() {
    let config = Config::default();
    assert_eq!(config.project_root, PathBuf::from("."));
    assert_eq!(config.refs_dir, PathBuf::from("refs"));
    assert!(config.languages.is_none());
    assert!(config.skip_libraries.is_empty());
    assert_eq!(config.output_format, OutputFormat::Terminal);
    assert!(config.use_cache);
    assert_eq!(config.cache_max_age_hours, 168);
}

#[test]
fn test_config_load_missing_file() {
    let tmp = tempfile::tempdir().unwrap();
    let config = Config::load(tmp.path()).unwrap();
    assert_eq!(config.project_root, tmp.path());
    assert_eq!(config.refs_dir, PathBuf::from("refs"));
}

#[test]
fn test_config_load_from_toml() {
    let tmp = tempfile::tempdir().unwrap();
    let config_content = r#"
refs_dir = "custom_refs"
skip_libraries = ["serde", "tokio"]
output_format = "Json"
use_cache = false
cache_max_age_hours = 24
"#;
    std::fs::write(tmp.path().join("polyref.toml"), config_content).unwrap();
    let config = Config::load(tmp.path()).unwrap();
    assert_eq!(config.refs_dir, PathBuf::from("custom_refs"));
    assert_eq!(config.skip_libraries, vec!["serde", "tokio"]);
    assert_eq!(config.output_format, OutputFormat::Json);
    assert!(!config.use_cache);
    assert_eq!(config.cache_max_age_hours, 24);
}

#[test]
fn test_config_resolved_refs_dir_relative() {
    let mut config = Config::default();
    config.project_root = PathBuf::from("/home/user/project");
    config.refs_dir = PathBuf::from("refs");
    let resolved = config.resolved_refs_dir();
    assert_eq!(resolved, PathBuf::from("/home/user/project/refs"));
}

#[test]
fn test_config_resolved_refs_dir_absolute() {
    let mut config = Config::default();
    config.project_root = PathBuf::from("/home/user/project");
    config.refs_dir = PathBuf::from("/absolute/refs");
    let resolved = config.resolved_refs_dir();
    assert_eq!(resolved, PathBuf::from("/absolute/refs"));
}

#[test]
fn test_config_skip_libraries() {
    let tmp = tempfile::tempdir().unwrap();
    let config_content = r#"
skip_libraries = ["tokio", "serde", "anyhow"]
"#;
    std::fs::write(tmp.path().join("polyref.toml"), config_content).unwrap();
    let config = Config::load(tmp.path()).unwrap();
    assert_eq!(config.skip_libraries.len(), 3);
    assert!(config.skip_libraries.contains(&"tokio".to_string()));
    assert!(config.skip_libraries.contains(&"serde".to_string()));
    assert!(config.skip_libraries.contains(&"anyhow".to_string()));
}

// ============================================================================
// Config global_refs_dir tests
// ============================================================================

#[test]
fn test_config_default_global_refs_dir_none() {
    let config = Config::default();
    assert!(config.global_refs_dir.is_none());
    assert!(config.resolved_global_refs_dir().is_none());
}

#[test]
fn test_config_load_with_global_refs_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let config_content = r#"
global_refs_dir = "C:/Users/me/references"
"#;
    std::fs::write(tmp.path().join("polyref.toml"), config_content).unwrap();
    let config = Config::load(tmp.path()).unwrap();
    assert_eq!(
        config.global_refs_dir,
        Some(PathBuf::from("C:/Users/me/references"))
    );
}

#[test]
fn test_config_resolved_global_refs_dir_absolute() {
    let mut config = Config::default();
    config.project_root = PathBuf::from("/home/user/project");
    config.global_refs_dir = Some(PathBuf::from("/absolute/refs"));
    let resolved = config.resolved_global_refs_dir();
    assert_eq!(resolved, Some(PathBuf::from("/absolute/refs")));
}

#[test]
fn test_config_resolved_global_refs_dir_relative() {
    let mut config = Config::default();
    config.project_root = PathBuf::from("/home/user/project");
    config.global_refs_dir = Some(PathBuf::from("../shared_refs"));
    let resolved = config.resolved_global_refs_dir();
    assert_eq!(
        resolved,
        Some(PathBuf::from("/home/user/project/../shared_refs"))
    );
}

#[test]
fn test_config_load_without_global_refs_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let config_content = r#"
refs_dir = "refs"
"#;
    std::fs::write(tmp.path().join("polyref.toml"), config_content).unwrap();
    let config = Config::load(tmp.path()).unwrap();
    assert!(config.global_refs_dir.is_none());
    assert!(config.resolved_global_refs_dir().is_none());
}
