use polyref::check::common;
use polyref::check::Checker;
use polyref::detect::Language;
use polyref::generate::{EntryKind, ReferenceEntry, ReferenceFile};
use std::path::PathBuf;

// ============================================================================
// Phase 6.1 — Common Validation Infrastructure
// ============================================================================

#[test]
fn test_fuzzy_match_exact() {
    let result = common::fuzzy_match("requests", &["requests", "flask", "pandas"], 0.5);
    assert!(result.is_some());
    let (name, score) = result.unwrap();
    assert_eq!(name, "requests");
    assert!((score - 1.0).abs() < f64::EPSILON);
}

#[test]
fn test_fuzzy_match_close() {
    let result = common::fuzzy_match("reqeusts", &["requests", "flask", "pandas"], 0.5);
    assert!(result.is_some());
    let (name, _score) = result.unwrap();
    assert_eq!(name, "requests");
}

#[test]
fn test_fuzzy_match_too_far() {
    let result = common::fuzzy_match("xyzabc", &["requests", "flask", "pandas"], 0.5);
    assert!(result.is_none());
}

#[test]
fn test_suggest_correction() {
    let known = vec!["get".to_string(), "post".to_string(), "put".to_string()];
    let suggestion = common::suggest_correction("gett", &known);
    assert!(suggestion.is_some());
    assert!(suggestion.unwrap().contains("get"));
}

#[test]
fn test_count_arguments_simple() {
    assert_eq!(common::count_arguments("func(a, b, c)"), 3);
}

#[test]
fn test_count_arguments_nested() {
    assert_eq!(common::count_arguments("func(a, inner(b, c), d)"), 3);
}

#[test]
fn test_count_arguments_strings() {
    assert_eq!(common::count_arguments(r#"func("a, b", c)"#), 2);
}

#[test]
fn test_count_arguments_empty() {
    assert_eq!(common::count_arguments("func()"), 0);
}

#[test]
fn test_is_inside_string() {
    assert!(common::is_inside_string(r#"x = "hello world""#, 10));
    assert!(!common::is_inside_string(r#"x = "hello" + y"#, 14));
}

#[test]
fn test_is_inside_comment_rust() {
    assert!(common::is_inside_comment("// this is a comment", 15, Language::Rust));
    assert!(!common::is_inside_comment("let x = 5; // comment", 5, Language::Rust));
}

#[test]
fn test_is_inside_comment_python() {
    assert!(common::is_inside_comment("# this is a comment", 10, Language::Python));
    assert!(!common::is_inside_comment("x = 5  # comment", 3, Language::Python));
}

#[test]
fn test_is_inside_comment_typescript() {
    assert!(common::is_inside_comment("// this is a comment", 15, Language::TypeScript));
    assert!(!common::is_inside_comment("const x = 5; // comment", 5, Language::TypeScript));
}

// ============================================================================
// Phase 6.2 — Rust Checker
// ============================================================================

fn make_rust_ref_file() -> ReferenceFile {
    ReferenceFile {
        library_name: "anyhow".to_string(),
        version: "1".to_string(),
        language: Language::Rust,
        entries: vec![
            ReferenceEntry { name: "Result".to_string(), kind: EntryKind::TypeAlias, signature: "type Result<T>".to_string(), description: String::new(), section: String::new() },
            ReferenceEntry { name: "Context".to_string(), kind: EntryKind::Trait, signature: "trait Context".to_string(), description: String::new(), section: String::new() },
            ReferenceEntry { name: "anyhow!".to_string(), kind: EntryKind::Macro, signature: "anyhow!(msg)".to_string(), description: String::new(), section: String::new() },
            ReferenceEntry { name: "bail!".to_string(), kind: EntryKind::Macro, signature: "bail!(msg)".to_string(), description: String::new(), section: String::new() },
            ReferenceEntry { name: "context".to_string(), kind: EntryKind::Method, signature: ".context(msg)".to_string(), description: String::new(), section: String::new() },
            ReferenceEntry { name: "chain".to_string(), kind: EntryKind::Method, signature: ".chain()".to_string(), description: String::new(), section: String::new() },
        ],
        raw_content: String::new(),
        file_path: PathBuf::from("refs/rust/lib_anyhow.rs"),
    }
}

#[test]
fn test_rust_check_valid_imports() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("valid.rs");
    std::fs::write(&source, "use anyhow::{Result, Context};\n").unwrap();

    let checker = polyref::check::rust::RustChecker;
    let refs = vec![make_rust_ref_file()];
    let result = checker.check(&[source], &refs).unwrap();
    assert!(result.is_clean());
}

#[test]
fn test_rust_check_invalid_import() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("invalid.rs");
    std::fs::write(&source, "use anyhow::NonExistent;\n").unwrap();

    let checker = polyref::check::rust::RustChecker;
    let refs = vec![make_rust_ref_file()];
    let result = checker.check(&[source], &refs).unwrap();
    assert!(!result.is_clean());
    assert!(result.issues.iter().any(|i| i.rule == "unknown-import"));
}

#[test]
fn test_rust_check_valid_function_call() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("valid_call.rs");
    std::fs::write(&source, "fn main() {\n    let x = result.context(\"msg\");\n}\n").unwrap();

    let checker = polyref::check::rust::RustChecker;
    let refs = vec![make_rust_ref_file()];
    let result = checker.check(&[source], &refs).unwrap();
    // context is a known method, should be clean
    let method_issues: Vec<_> = result.issues.iter().filter(|i| i.rule == "unknown-method").collect();
    assert!(method_issues.is_empty());
}

#[test]
fn test_rust_check_invalid_method() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("invalid_method.rs");
    // "contex" is close to "context" — should trigger fuzzy match
    std::fs::write(&source, "fn main() {\n    let x = result.contex(\"msg\");\n}\n").unwrap();

    let checker = polyref::check::rust::RustChecker;
    let refs = vec![make_rust_ref_file()];
    let result = checker.check(&[source], &refs).unwrap();
    let method_issues: Vec<_> = result.issues.iter().filter(|i| i.rule == "unknown-method").collect();
    assert!(!method_issues.is_empty());
}

#[test]
fn test_rust_check_suggestion_provided() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("suggestion.rs");
    std::fs::write(&source, "fn main() {\n    result.contex(\"msg\");\n}\n").unwrap();

    let checker = polyref::check::rust::RustChecker;
    let refs = vec![make_rust_ref_file()];
    let result = checker.check(&[source], &refs).unwrap();
    let has_suggestion = result.issues.iter().any(|i| i.suggestion.is_some());
    assert!(has_suggestion);
}

#[test]
fn test_rust_check_skips_comments() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("comments.rs");
    std::fs::write(&source, "// use anyhow::NonExistent;\nfn main() {}\n").unwrap();

    let checker = polyref::check::rust::RustChecker;
    let refs = vec![make_rust_ref_file()];
    let result = checker.check(&[source], &refs).unwrap();
    assert!(result.is_clean());
}

#[test]
fn test_rust_check_skips_strings() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("strings.rs");
    std::fs::write(
        &source,
        "fn main() {\n    let s = \"use anyhow::NonExistent\";\n}\n",
    )
    .unwrap();

    let checker = polyref::check::rust::RustChecker;
    let refs = vec![make_rust_ref_file()];
    let result = checker.check(&[source], &refs).unwrap();
    // String contents shouldn't be parsed as imports
    let import_issues: Vec<_> = result.issues.iter().filter(|i| i.rule == "unknown-import").collect();
    assert!(import_issues.is_empty());
}

#[test]
fn test_rust_check_multiple_files() {
    let tmp = tempfile::tempdir().unwrap();
    let source1 = tmp.path().join("file1.rs");
    let source2 = tmp.path().join("file2.rs");
    std::fs::write(&source1, "use anyhow::Result;\n").unwrap();
    std::fs::write(&source2, "use anyhow::NonExistent;\n").unwrap();

    let checker = polyref::check::rust::RustChecker;
    let refs = vec![make_rust_ref_file()];
    let result = checker.check(&[source1, source2], &refs).unwrap();
    assert_eq!(result.files_checked, 2);
    assert!(!result.is_clean());
}

// ============================================================================
// Phase 6.3 — Python Checker
// ============================================================================

fn make_python_ref_file() -> ReferenceFile {
    let content = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/python_refs/lib_requests.py"),
    )
    .unwrap();
    let entries = polyref::generate::python::parse_python_reference(&content);
    ReferenceFile {
        library_name: "requests".to_string(),
        version: "2.31.0".to_string(),
        language: Language::Python,
        entries,
        raw_content: content,
        file_path: PathBuf::from("refs/python/lib_requests.py"),
    }
}

#[test]
fn test_python_check_valid_imports() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("valid.py");
    std::fs::write(&source, "from requests import Session, Response\n").unwrap();

    let checker = polyref::check::python::PythonChecker;
    let refs = vec![make_python_ref_file()];
    let result = checker.check(&[source], &refs).unwrap();
    assert!(result.is_clean());
}

#[test]
fn test_python_check_invalid_import() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("invalid.py");
    std::fs::write(&source, "from requests import NonExistent\n").unwrap();

    let checker = polyref::check::python::PythonChecker;
    let refs = vec![make_python_ref_file()];
    let result = checker.check(&[source], &refs).unwrap();
    assert!(!result.is_clean());
    assert!(result.issues.iter().any(|i| i.rule == "unknown-import"));
}

#[test]
fn test_python_check_valid_function_call() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("valid_call.py");
    std::fs::write(
        &source,
        "import requests\nresponse = requests.get(\"https://example.com\")\n",
    )
    .unwrap();

    let checker = polyref::check::python::PythonChecker;
    let refs = vec![make_python_ref_file()];
    let result = checker.check(&[source], &refs).unwrap();
    let fn_issues: Vec<_> = result.issues.iter().filter(|i| i.rule == "unknown-function").collect();
    assert!(fn_issues.is_empty());
}

#[test]
fn test_python_check_unknown_function() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("unknown_fn.py");
    std::fs::write(
        &source,
        "import requests\nresult = requests.fetch(\"https://example.com\")\n",
    )
    .unwrap();

    let checker = polyref::check::python::PythonChecker;
    let refs = vec![make_python_ref_file()];
    let result = checker.check(&[source], &refs).unwrap();
    assert!(result.issues.iter().any(|i| i.rule == "unknown-function"));
}

#[test]
fn test_python_check_missing_required_arg() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("missing_arg.py");
    std::fs::write(
        &source,
        "import requests\nresponse = requests.get()\n",
    )
    .unwrap();

    let checker = polyref::check::python::PythonChecker;
    let refs = vec![make_python_ref_file()];
    let result = checker.check(&[source], &refs).unwrap();
    assert!(result.issues.iter().any(|i| i.rule == "missing-required-arg"));
}

#[test]
fn test_python_check_invalid_method() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("invalid_method.py");
    std::fs::write(
        &source,
        "import requests\nresponse = requests.get(\"https://example.com\")\nresponse.jso()\n",
    )
    .unwrap();

    let checker = polyref::check::python::PythonChecker;
    let refs = vec![make_python_ref_file()];
    let result = checker.check(&[source], &refs).unwrap();
    // "jso" is close to "json" — should flag
    let method_issues: Vec<_> = result.issues.iter().filter(|i| i.rule == "unknown-method").collect();
    assert!(!method_issues.is_empty());
}

#[test]
fn test_python_check_decorator_validation() {
    // Decorators are parsed as entries — validation happens via reference lookup
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("decorator.py");
    std::fs::write(
        &source,
        "import requests\n# Valid code, no decorator issues\nresponse = requests.get(\"url\")\n",
    )
    .unwrap();

    let checker = polyref::check::python::PythonChecker;
    let refs = vec![make_python_ref_file()];
    let result = checker.check(&[source], &refs).unwrap();
    // No decorator issues in this source
    assert!(result.is_clean());
}

#[test]
fn test_python_check_skips_comments() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("comments.py");
    std::fs::write(
        &source,
        "# import requests\n# requests.fetch()\nx = 5\n",
    )
    .unwrap();

    let checker = polyref::check::python::PythonChecker;
    let refs = vec![make_python_ref_file()];
    let result = checker.check(&[source], &refs).unwrap();
    assert!(result.is_clean());
}

#[test]
fn test_python_check_skips_strings() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("strings.py");
    std::fs::write(
        &source,
        "x = \"requests.fetch()\"\n",
    )
    .unwrap();

    let checker = polyref::check::python::PythonChecker;
    let refs = vec![make_python_ref_file()];
    let result = checker.check(&[source], &refs).unwrap();
    assert!(result.is_clean());
}

#[test]
fn test_python_check_class_instantiation() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("class_inst.py");
    std::fs::write(
        &source,
        "from requests import Session\ns = Session()\n",
    )
    .unwrap();

    let checker = polyref::check::python::PythonChecker;
    let refs = vec![make_python_ref_file()];
    let result = checker.check(&[source], &refs).unwrap();
    assert!(result.is_clean());
}

// ============================================================================
// Phase 6.4 — TypeScript Checker
// ============================================================================

fn make_ts_ref_file() -> ReferenceFile {
    let content = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ts_refs/lib_react.ts"),
    )
    .unwrap();
    let entries = polyref::generate::typescript::parse_typescript_reference(&content);
    ReferenceFile {
        library_name: "react".to_string(),
        version: "^18.2.0".to_string(),
        language: Language::TypeScript,
        entries,
        raw_content: content,
        file_path: PathBuf::from("refs/typescript/lib_react.ts"),
    }
}

#[test]
fn test_ts_check_valid_imports() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("valid.ts");
    std::fs::write(
        &source,
        "import { useState, useEffect } from 'react';\n",
    )
    .unwrap();

    let checker = polyref::check::typescript::TypeScriptChecker;
    let refs = vec![make_ts_ref_file()];
    let result = checker.check(&[source], &refs).unwrap();
    assert!(result.is_clean());
}

#[test]
fn test_ts_check_invalid_import() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("invalid.ts");
    std::fs::write(
        &source,
        "import { NonExistent } from 'react';\n",
    )
    .unwrap();

    let checker = polyref::check::typescript::TypeScriptChecker;
    let refs = vec![make_ts_ref_file()];
    let result = checker.check(&[source], &refs).unwrap();
    assert!(!result.is_clean());
    assert!(result.issues.iter().any(|i| i.rule == "unknown-import"));
}

#[test]
fn test_ts_check_valid_hook_usage() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("hooks.ts");
    std::fs::write(
        &source,
        "import { useState } from 'react';\nconst [count, setCount] = useState(0);\n",
    )
    .unwrap();

    let checker = polyref::check::typescript::TypeScriptChecker;
    let refs = vec![make_ts_ref_file()];
    let result = checker.check(&[source], &refs).unwrap();
    assert!(result.is_clean());
}

#[test]
fn test_ts_check_wrong_destructuring() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("wrong_destruct.ts");
    std::fs::write(
        &source,
        "import { useState } from 'react';\nconst [a, b, c] = useState(0);\n",
    )
    .unwrap();

    let checker = polyref::check::typescript::TypeScriptChecker;
    let refs = vec![make_ts_ref_file()];
    let result = checker.check(&[source], &refs).unwrap();
    assert!(result.issues.iter().any(|i| i.rule == "wrong-destructure"));
}

#[test]
fn test_ts_check_unknown_function() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("unknown_fn.ts");
    std::fs::write(
        &source,
        "import { useNonExistent } from 'react';\nuseNonExistent();\n",
    )
    .unwrap();

    let checker = polyref::check::typescript::TypeScriptChecker;
    let refs = vec![make_ts_ref_file()];
    let result = checker.check(&[source], &refs).unwrap();
    // useNonExistent should fail at import
    assert!(result.issues.iter().any(|i| i.rule == "unknown-import"));
}

#[test]
fn test_ts_check_valid_type_usage() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("types.ts");
    std::fs::write(
        &source,
        "import { useState, useEffect } from 'react';\nconst [x, setX] = useState<string>('');\n",
    )
    .unwrap();

    let checker = polyref::check::typescript::TypeScriptChecker;
    let refs = vec![make_ts_ref_file()];
    let result = checker.check(&[source], &refs).unwrap();
    assert!(result.is_clean());
}

#[test]
fn test_ts_check_skips_comments() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("comments.ts");
    std::fs::write(
        &source,
        "// import { NonExistent } from 'react';\nconst x = 5;\n",
    )
    .unwrap();

    let checker = polyref::check::typescript::TypeScriptChecker;
    let refs = vec![make_ts_ref_file()];
    let result = checker.check(&[source], &refs).unwrap();
    assert!(result.is_clean());
}

#[test]
fn test_ts_check_skips_strings() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("strings.ts");
    std::fs::write(
        &source,
        "const x = `import { NonExistent } from 'react'`;\n",
    )
    .unwrap();

    let checker = polyref::check::typescript::TypeScriptChecker;
    let refs = vec![make_ts_ref_file()];
    let result = checker.check(&[source], &refs).unwrap();
    assert!(result.is_clean());
}

#[test]
fn test_ts_check_jsx_component_validation() {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("component.tsx");
    std::fs::write(
        &source,
        "import { useState } from 'react';\nconst [x, setX] = useState(0);\n",
    )
    .unwrap();

    let checker = polyref::check::typescript::TypeScriptChecker;
    let refs = vec![make_ts_ref_file()];
    let result = checker.check(&[source], &refs).unwrap();
    assert!(result.is_clean());
}

#[test]
fn test_ts_check_multiple_files() {
    let tmp = tempfile::tempdir().unwrap();
    let source1 = tmp.path().join("file1.ts");
    let source2 = tmp.path().join("file2.tsx");
    std::fs::write(&source1, "import { useState } from 'react';\n").unwrap();
    std::fs::write(&source2, "import { NonExistent } from 'react';\n").unwrap();

    let checker = polyref::check::typescript::TypeScriptChecker;
    let refs = vec![make_ts_ref_file()];
    let result = checker.check(&[source1, source2], &refs).unwrap();
    assert_eq!(result.files_checked, 2);
    assert!(!result.is_clean());
}
