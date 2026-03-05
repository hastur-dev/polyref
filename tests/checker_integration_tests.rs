use polyref::check::Checker;
use polyref::check::rust::RustChecker;
use polyref::detect::Language;
use polyref::generate::{EntryKind, ReferenceEntry, ReferenceFile};
use std::path::PathBuf;

fn make_ref_file(entries: Vec<ReferenceEntry>) -> ReferenceFile {
    ReferenceFile {
        library_name: "testlib".to_string(),
        version: "1.0".to_string(),
        language: Language::Rust,
        entries,
        raw_content: String::new(),
        file_path: PathBuf::from("refs/rust/lib_testlib.rs"),
    }
}

fn make_method_entry(name: &str, min: Option<usize>, max: Option<usize>) -> ReferenceEntry {
    ReferenceEntry {
        name: name.to_string(),
        kind: EntryKind::Method,
        signature: format!("fn {}()", name),
        description: String::new(),
        section: String::new(),
        type_context: None,
        parent: None,
        min_args: min,
        max_args: max,
        original_path: None,
    }
}

#[test]
fn test_ast_checker_detects_unknown_method() {
    let source = r#"
use testlib;
fn main() {
    let x = testlib::new();
    x.nonexistent_method();
}
"#;
    let tmp = tempfile::NamedTempFile::with_suffix(".rs").unwrap();
    std::fs::write(tmp.path(), source).unwrap();

    let ref_file = make_ref_file(vec![
        make_method_entry("push", Some(1), Some(1)),
        make_method_entry("pop", Some(0), Some(0)),
    ]);

    let checker = RustChecker;
    let result = checker.check(&[tmp.path().to_path_buf()], &[ref_file]).unwrap();
    let unknown_issues: Vec<_> = result.issues.iter()
        .filter(|i| i.rule == "unknown-method")
        .collect();
    assert!(!unknown_issues.is_empty(), "should detect unknown method 'nonexistent_method'");
    assert!(unknown_issues[0].message.contains("nonexistent_method"));
}

#[test]
fn test_ast_checker_accepts_known_method() {
    let source = r#"
use testlib;
fn main() {
    let x = testlib::new();
    x.push(1);
}
"#;
    let tmp = tempfile::NamedTempFile::with_suffix(".rs").unwrap();
    std::fs::write(tmp.path(), source).unwrap();

    let ref_file = make_ref_file(vec![
        make_method_entry("push", Some(1), Some(1)),
        make_method_entry("new", None, None),
    ]);

    let checker = RustChecker;
    let result = checker.check(&[tmp.path().to_path_buf()], &[ref_file]).unwrap();
    let push_issues: Vec<_> = result.issues.iter()
        .filter(|i| i.message.contains("push"))
        .collect();
    assert!(push_issues.is_empty(), "push is a known method, should not flag: {:?}", push_issues);
}

#[test]
fn test_ast_checker_arg_count_too_few() {
    let source = r#"
use testlib;
fn main() {
    let x = testlib::new();
    x.insert();
}
"#;
    let tmp = tempfile::NamedTempFile::with_suffix(".rs").unwrap();
    std::fs::write(tmp.path(), source).unwrap();

    let ref_file = make_ref_file(vec![
        make_method_entry("insert", Some(2), Some(2)),
        make_method_entry("new", None, None),
    ]);

    let checker = RustChecker;
    let result = checker.check(&[tmp.path().to_path_buf()], &[ref_file]).unwrap();
    let arg_issues: Vec<_> = result.issues.iter()
        .filter(|i| i.rule == "too-few-args")
        .collect();
    assert!(!arg_issues.is_empty(), "should detect too few args for insert()");
}

#[test]
fn test_ast_checker_arg_count_too_many() {
    let source = r#"
use testlib;
fn main() {
    let x = testlib::new();
    x.clear(1, 2, 3);
}
"#;
    let tmp = tempfile::NamedTempFile::with_suffix(".rs").unwrap();
    std::fs::write(tmp.path(), source).unwrap();

    let ref_file = make_ref_file(vec![
        make_method_entry("clear", Some(0), Some(0)),
        make_method_entry("new", None, None),
    ]);

    let checker = RustChecker;
    let result = checker.check(&[tmp.path().to_path_buf()], &[ref_file]).unwrap();
    let arg_issues: Vec<_> = result.issues.iter()
        .filter(|i| i.rule == "too-many-args")
        .collect();
    assert!(!arg_issues.is_empty(), "should detect too many args for clear(1, 2, 3)");
}

#[test]
fn test_regex_fallback_on_parse_error() {
    // Source with syntax error should fall back to regex-based checking
    let source = "use testlib;\nfn main() { let x = ; x.push(1); }";
    let tmp = tempfile::NamedTempFile::with_suffix(".rs").unwrap();
    std::fs::write(tmp.path(), source).unwrap();

    let ref_file = make_ref_file(vec![
        make_method_entry("push", Some(1), Some(1)),
    ]);

    let checker = RustChecker;
    // Should not panic — falls back to regex
    let result = checker.check(&[tmp.path().to_path_buf()], &[ref_file]);
    assert!(result.is_ok(), "regex fallback should handle parse errors gracefully");
}

#[test]
fn test_commands_enforce_module_functions() {
    // Test the extracted enforce module helper functions
    use polyref::commands::enforce::*;

    assert_eq!(detect_language_from_content("fn main() {}", "auto"), Language::Rust);
    assert_eq!(detect_language_from_content("def foo():", "auto"), Language::Python);
    assert_eq!(detect_language_from_content("function bar() {}", "auto"), Language::TypeScript);
    assert_eq!(detect_language_from_content("fn main() {}", "python"), Language::Python);

    let config = build_enforce_config_from_args(true, false, Some(80), false, "json");
    assert!(config.hard_block);
    assert_eq!(config.require_coverage, Some(80));

    assert_eq!(extract_lib_name_from_path(&PathBuf::from("refs/rust/lib_tokio.rs")), "tokio");
    assert_eq!(extract_lib_name_from_path(&PathBuf::from("refs/requests.polyref")), "requests");

    assert_eq!(extract_version_from_content("// Version: 2.0\ncode"), "2.0");
    assert_eq!(extract_version_from_content("no version here"), "unknown");
}
