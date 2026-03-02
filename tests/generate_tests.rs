use polyref::detect::{Dependency, Language};
use polyref::generate::cache::Cache;
use polyref::generate::templates;
use polyref::generate::{EntryKind, Generator, ReferenceEntry};
use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

// ============================================================================
// Phase 3.1 — Cache tests
// ============================================================================

#[test]
fn test_cache_create_and_save() {
    let tmp = tempfile::tempdir().unwrap();
    let mut cache = Cache::new();
    let dep = Dependency {
        name: "serde".to_string(),
        version: "1.0".to_string(),
        language: Language::Rust,
        source_file: "Cargo.toml".to_string(),
    };
    cache.record(&dep, PathBuf::from("refs/rust/lib_serde.rs"));
    cache.save(tmp.path()).unwrap();

    // Reload and verify
    let loaded = Cache::load(tmp.path()).unwrap();
    assert!(loaded.is_valid(&dep, 168));
}

#[test]
fn test_cache_is_valid_current() {
    let mut cache = Cache::new();
    let dep = Dependency {
        name: "serde".to_string(),
        version: "1.0".to_string(),
        language: Language::Rust,
        source_file: "Cargo.toml".to_string(),
    };
    cache.record(&dep, PathBuf::from("test.rs"));
    assert!(cache.is_valid(&dep, 168));
}

#[test]
fn test_cache_is_valid_expired() {
    let mut cache = Cache::new();
    let dep = Dependency {
        name: "serde".to_string(),
        version: "1.0".to_string(),
        language: Language::Rust,
        source_file: "Cargo.toml".to_string(),
    };
    cache.record(&dep, PathBuf::from("test.rs"));
    // 0 hours max age → immediately expired
    assert!(!cache.is_valid(&dep, 0));
}

#[test]
fn test_cache_is_valid_version_mismatch() {
    let mut cache = Cache::new();
    let dep = Dependency {
        name: "serde".to_string(),
        version: "1.0".to_string(),
        language: Language::Rust,
        source_file: "Cargo.toml".to_string(),
    };
    cache.record(&dep, PathBuf::from("test.rs"));

    let dep2 = Dependency {
        name: "serde".to_string(),
        version: "2.0".to_string(),
        language: Language::Rust,
        source_file: "Cargo.toml".to_string(),
    };
    assert!(!cache.is_valid(&dep2, 168));
}

#[test]
fn test_cache_is_valid_missing() {
    let cache = Cache::new();
    let dep = Dependency {
        name: "nonexistent".to_string(),
        version: "1.0".to_string(),
        language: Language::Rust,
        source_file: "Cargo.toml".to_string(),
    };
    assert!(!cache.is_valid(&dep, 168));
}

// ============================================================================
// Phase 3.1 — Template tests
// ============================================================================

#[test]
fn test_section_header_format() {
    let header = templates::section_header("TEST SECTION");
    assert!(header.contains("============"));
    assert!(header.contains("TEST SECTION"));
}

#[test]
fn test_file_header_rust_format() {
    let header = templates::file_header_rust("anyhow", "1.0", "Result, Context");
    assert!(header.contains("anyhow Reference"));
    assert!(header.contains("Cargo.toml"));
    assert!(header.contains("1.0"));
}

#[test]
fn test_file_header_python_format() {
    let header = templates::file_header_python("requests", "2.31.0", "Session, Response");
    assert!(header.contains("requests Reference"));
    assert!(header.contains("pip install"));
    assert!(header.contains("2.31.0"));
}

#[test]
fn test_file_header_typescript_format() {
    let header = templates::file_header_typescript("react", "^18.2.0", "useState, useEffect");
    assert!(header.contains("react Reference"));
    assert!(header.contains("package.json"));
    assert!(header.contains("^18.2.0"));
}

// ============================================================================
// Phase 3.2 — Rust Reference Generator
// ============================================================================

#[test]
fn test_rust_gen_parse_sections() {
    let content = r#"// test Reference
// Cargo.toml: test = "1"

// ============================================================================
// CORE FUNCTIONS
// ============================================================================

fn example() {}

// ============================================================================
// UTILITIES
// ============================================================================

fn helper() {}
"#;
    let entries = polyref::generate::rust::parse_rust_reference(content);
    let sections: Vec<&str> = entries.iter().map(|e| e.section.as_str()).collect();
    assert!(sections.contains(&"CORE FUNCTIONS"));
    assert!(sections.contains(&"UTILITIES"));
}

#[test]
fn test_rust_gen_parse_functions() {
    let content = "fn do_something(a: i32, b: &str) -> Result<()> {\n    Ok(())\n}\n\npub fn public_fn() {}\n";
    let entries = polyref::generate::rust::parse_rust_reference(content);
    let fn_entries: Vec<&ReferenceEntry> = entries
        .iter()
        .filter(|e| e.kind == EntryKind::Function)
        .collect();
    assert!(fn_entries.iter().any(|e| e.name == "do_something"));
    assert!(fn_entries.iter().any(|e| e.name == "public_fn"));
}

#[test]
fn test_rust_gen_parse_methods() {
    let content = "result.context(\"msg\");\nerr.chain();\n";
    let entries = polyref::generate::rust::parse_rust_reference(content);
    let method_entries: Vec<&ReferenceEntry> = entries
        .iter()
        .filter(|e| e.kind == EntryKind::Method)
        .collect();
    assert!(method_entries.iter().any(|e| e.name == "context"));
    assert!(method_entries.iter().any(|e| e.name == "chain"));
}

#[test]
fn test_rust_gen_parse_macros() {
    let content = "anyhow!(\"error msg\");\nbail!(\"failure\");\nensure!(condition, \"msg\");\n";
    let entries = polyref::generate::rust::parse_rust_reference(content);
    let macro_entries: Vec<&ReferenceEntry> = entries
        .iter()
        .filter(|e| e.kind == EntryKind::Macro)
        .collect();
    assert!(macro_entries.iter().any(|e| e.name == "anyhow!"));
    assert!(macro_entries.iter().any(|e| e.name == "bail!"));
    assert!(macro_entries.iter().any(|e| e.name == "ensure!"));
}

#[test]
fn test_rust_gen_parse_use_statements() {
    let content = "use anyhow::{Result, Context, bail};\n";
    let entries = polyref::generate::rust::parse_rust_reference(content);
    let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(names.contains(&"Result"));
    assert!(names.contains(&"Context"));
    assert!(names.contains(&"bail"));
}

#[test]
fn test_rust_gen_user_provided_file() {
    let tmp = tempfile::tempdir().unwrap();
    let rust_dir = tmp.path().join("rust");
    std::fs::create_dir_all(&rust_dir).unwrap();

    let ref_content = "// test Reference\nfn my_function() {}\n";
    std::fs::write(rust_dir.join("lib_test_crate.rs"), ref_content).unwrap();

    let gen = polyref::generate::rust::RustGenerator;
    let dep = Dependency {
        name: "test_crate".to_string(),
        version: "1.0".to_string(),
        language: Language::Rust,
        source_file: "Cargo.toml".to_string(),
    };
    let result = gen.generate(&dep, tmp.path(), None).unwrap();
    assert!(result.raw_content.contains("my_function"));
}

#[test]
fn test_rust_gen_stub_generation() {
    // Disable fetching so we always get a stub
    std::env::set_var("POLYREF_NO_FETCH", "1");
    let tmp = tempfile::tempdir().unwrap();
    let gen = polyref::generate::rust::RustGenerator;
    let dep = Dependency {
        name: "unknown_crate".to_string(),
        version: "0.1.0".to_string(),
        language: Language::Rust,
        source_file: "Cargo.toml".to_string(),
    };
    let result = gen.generate(&dep, tmp.path(), None).unwrap();
    assert!(result.raw_content.contains("stub"));
    assert!(result.file_path.exists());
    std::env::remove_var("POLYREF_NO_FETCH");
}

#[test]
fn test_rust_gen_stub_fallback_when_fetch_fails() {
    // Use a crate name that doesn't exist on docs.rs
    // This should hit a 404 and fall back to stub generation
    let tmp = tempfile::tempdir().unwrap();
    let gen = polyref::generate::rust::RustGenerator;
    let dep = Dependency {
        name: "zzz_nonexistent_crate_12345_xyzzy".to_string(),
        version: "0.0.0".to_string(),
        language: Language::Rust,
        source_file: "Cargo.toml".to_string(),
    };
    // Should NOT panic, should fall back gracefully to stub
    let result = gen.generate(&dep, tmp.path(), None).unwrap();
    assert!(result.file_path.exists());
    // Should contain stub marker since fetch will fail
    assert!(result.raw_content.contains("stub") || result.raw_content.contains("Reference"));
}

// ============================================================================
// Phase 4.1 — Python Reference Generator
// ============================================================================

#[test]
fn test_python_gen_parse_functions() {
    let content = std::fs::read_to_string(fixtures_dir().join("python_refs/lib_requests.py")).unwrap();
    let entries = polyref::generate::python::parse_python_reference(&content);
    let functions: Vec<&str> = entries
        .iter()
        .filter(|e| e.kind == EntryKind::Function)
        .map(|e| e.name.as_str())
        .collect();
    assert!(functions.contains(&"get"));
    assert!(functions.contains(&"post"));
    assert!(functions.contains(&"put"));
    assert!(functions.contains(&"delete"));
}

#[test]
fn test_python_gen_parse_classes() {
    let content = std::fs::read_to_string(fixtures_dir().join("python_refs/lib_requests.py")).unwrap();
    let entries = polyref::generate::python::parse_python_reference(&content);
    let classes: Vec<&str> = entries
        .iter()
        .filter(|e| e.kind == EntryKind::Class)
        .map(|e| e.name.as_str())
        .collect();
    assert!(classes.contains(&"Response"));
    assert!(classes.contains(&"Session"));
}

#[test]
fn test_python_gen_parse_methods() {
    let content = std::fs::read_to_string(fixtures_dir().join("python_refs/lib_requests.py")).unwrap();
    let entries = polyref::generate::python::parse_python_reference(&content);
    let methods: Vec<&str> = entries
        .iter()
        .filter(|e| e.kind == EntryKind::Method)
        .map(|e| e.name.as_str())
        .collect();
    assert!(methods.contains(&"json"));
    assert!(methods.contains(&"raise_for_status"));
    assert!(methods.contains(&"close"));
}

#[test]
fn test_python_gen_parse_properties() {
    let content = std::fs::read_to_string(fixtures_dir().join("python_refs/lib_requests.py")).unwrap();
    let entries = polyref::generate::python::parse_python_reference(&content);
    let props: Vec<&str> = entries
        .iter()
        .filter(|e| e.kind == EntryKind::Property)
        .map(|e| e.name.as_str())
        .collect();
    assert!(props.contains(&"ok"));
    assert!(props.contains(&"is_redirect"));
}

#[test]
fn test_python_gen_parse_constants() {
    let content = std::fs::read_to_string(fixtures_dir().join("python_refs/lib_requests.py")).unwrap();
    let entries = polyref::generate::python::parse_python_reference(&content);
    // Class attributes like status_code, text, etc.
    let consts: Vec<&str> = entries
        .iter()
        .filter(|e| e.kind == EntryKind::Constant)
        .map(|e| e.name.as_str())
        .collect();
    assert!(consts.contains(&"status_code"));
    assert!(consts.contains(&"text"));
}

#[test]
fn test_python_gen_parse_exceptions() {
    let content = std::fs::read_to_string(fixtures_dir().join("python_refs/lib_requests.py")).unwrap();
    let entries = polyref::generate::python::parse_python_reference(&content);
    let classes: Vec<&str> = entries
        .iter()
        .filter(|e| e.kind == EntryKind::Class)
        .map(|e| e.name.as_str())
        .collect();
    assert!(classes.contains(&"HTTPError"));
    assert!(classes.contains(&"ConnectionError"));
    assert!(classes.contains(&"Timeout"));
}

#[test]
fn test_python_gen_parse_imports() {
    let content = std::fs::read_to_string(fixtures_dir().join("python_refs/lib_requests.py")).unwrap();
    let entries = polyref::generate::python::parse_python_reference(&content);
    let modules: Vec<&str> = entries
        .iter()
        .filter(|e| e.kind == EntryKind::Module)
        .map(|e| e.name.as_str())
        .collect();
    assert!(modules.contains(&"Session"));
    assert!(modules.contains(&"Response"));
    assert!(modules.contains(&"HTTPError"));
}

#[test]
fn test_python_gen_parse_signatures() {
    let sig = polyref::generate::python::extract_python_function_sig(
        "def get(url: str, params: dict = None, **kwargs) -> Response: ...",
    )
    .unwrap();
    assert_eq!(sig.name, "get");
    assert_eq!(sig.params.len(), 3); // url, params, kwargs
    assert_eq!(sig.return_type, Some("Response".to_string()));
}

#[test]
fn test_python_gen_kwargs_handling() {
    let sig = polyref::generate::python::extract_python_function_sig(
        "def request(method: str, url: str, **kwargs) -> Response: ...",
    )
    .unwrap();
    assert!(sig.params.iter().any(|p| p.is_kwargs));
    assert_eq!(sig.params.len(), 3);
}

#[test]
fn test_python_gen_user_provided_file() {
    let tmp = tempfile::tempdir().unwrap();
    let py_dir = tmp.path().join("python");
    std::fs::create_dir_all(&py_dir).unwrap();

    let ref_content = "# test Reference\ndef my_function(): ...\n";
    std::fs::write(py_dir.join("lib_test_pkg.py"), ref_content).unwrap();

    let gen = polyref::generate::python::PythonGenerator;
    let dep = Dependency {
        name: "test_pkg".to_string(),
        version: "1.0".to_string(),
        language: Language::Python,
        source_file: "requirements.txt".to_string(),
    };
    let result = gen.generate(&dep, tmp.path(), None).unwrap();
    assert!(result.raw_content.contains("my_function"));
}

// ============================================================================
// Phase 5.1 — TypeScript Reference Generator
// ============================================================================

#[test]
fn test_ts_gen_parse_functions() {
    let content = std::fs::read_to_string(fixtures_dir().join("ts_refs/lib_react.ts")).unwrap();
    let entries = polyref::generate::typescript::parse_typescript_reference(&content);
    let fns: Vec<&str> = entries
        .iter()
        .filter(|e| e.kind == EntryKind::Function || e.kind == EntryKind::Hook)
        .map(|e| e.name.as_str())
        .collect();
    assert!(fns.contains(&"createContext"));
    assert!(fns.contains(&"createRef"));
    assert!(fns.contains(&"forwardRef"));
    assert!(fns.contains(&"memo"));
}

#[test]
fn test_ts_gen_parse_hooks() {
    let content = std::fs::read_to_string(fixtures_dir().join("ts_refs/lib_react.ts")).unwrap();
    let entries = polyref::generate::typescript::parse_typescript_reference(&content);
    let hooks: Vec<&str> = entries
        .iter()
        .filter(|e| e.kind == EntryKind::Hook)
        .map(|e| e.name.as_str())
        .collect();
    assert!(hooks.contains(&"useState"));
    assert!(hooks.contains(&"useEffect"));
    assert!(hooks.contains(&"useCallback"));
    assert!(hooks.contains(&"useMemo"));
    assert!(hooks.contains(&"useRef"));
    assert!(hooks.contains(&"useContext"));
    assert!(hooks.contains(&"useReducer"));
}

#[test]
fn test_ts_gen_parse_interfaces() {
    let content = std::fs::read_to_string(fixtures_dir().join("ts_refs/lib_react.ts")).unwrap();
    let entries = polyref::generate::typescript::parse_typescript_reference(&content);
    let interfaces: Vec<&str> = entries
        .iter()
        .filter(|e| e.kind == EntryKind::Interface)
        .map(|e| e.name.as_str())
        .collect();
    assert!(interfaces.contains(&"FC"));
    assert!(interfaces.contains(&"Context"));
}

#[test]
fn test_ts_gen_parse_type_aliases() {
    let content = std::fs::read_to_string(fixtures_dir().join("ts_refs/lib_react.ts")).unwrap();
    let entries = polyref::generate::typescript::parse_typescript_reference(&content);
    let types: Vec<&str> = entries
        .iter()
        .filter(|e| e.kind == EntryKind::TypeAlias)
        .map(|e| e.name.as_str())
        .collect();
    assert!(types.contains(&"ReactNode"));
    assert!(types.contains(&"ReactElement"));
    assert!(types.contains(&"JSXElement"));
}

#[test]
fn test_ts_gen_parse_generics() {
    let content = std::fs::read_to_string(fixtures_dir().join("ts_refs/lib_react.ts")).unwrap();
    let entries = polyref::generate::typescript::parse_typescript_reference(&content);
    // useState Hook entry should have generics in signature
    let use_state = entries.iter().find(|e| e.name == "useState" && e.kind == EntryKind::Hook).unwrap();
    assert!(use_state.signature.contains("<S>"), "signature was: {}", use_state.signature);
}

#[test]
fn test_ts_gen_parse_imports() {
    let content = std::fs::read_to_string(fixtures_dir().join("ts_refs/lib_react.ts")).unwrap();
    let entries = polyref::generate::typescript::parse_typescript_reference(&content);
    let imports: Vec<&str> = entries
        .iter()
        .filter(|e| e.kind == EntryKind::Module)
        .map(|e| e.name.as_str())
        .collect();
    assert!(imports.contains(&"useState"));
    assert!(imports.contains(&"useEffect"));
}

#[test]
fn test_ts_gen_parse_components() {
    let content = std::fs::read_to_string(fixtures_dir().join("ts_refs/lib_react.ts")).unwrap();
    let entries = polyref::generate::typescript::parse_typescript_reference(&content);
    let components: Vec<&str> = entries
        .iter()
        .filter(|e| e.kind == EntryKind::Component)
        .map(|e| e.name.as_str())
        .collect();
    assert!(
        components.contains(&"ExampleComponent") || components.contains(&"ThemedComponent"),
        "Should detect JSX-returning functions as components"
    );
}

#[test]
fn test_ts_gen_user_provided_file() {
    let tmp = tempfile::tempdir().unwrap();
    let ts_dir = tmp.path().join("typescript");
    std::fs::create_dir_all(&ts_dir).unwrap();

    let ref_content = "// test Reference\nfunction myFunc(): void;\n";
    std::fs::write(ts_dir.join("lib_test_lib.ts"), ref_content).unwrap();

    let gen = polyref::generate::typescript::TypeScriptGenerator;
    let dep = Dependency {
        name: "test_lib".to_string(),
        version: "1.0".to_string(),
        language: Language::TypeScript,
        source_file: "package.json".to_string(),
    };
    let result = gen.generate(&dep, tmp.path(), None).unwrap();
    assert!(result.raw_content.contains("myFunc"));
}

// ============================================================================
// Global refs directory fallback tests
// ============================================================================

#[test]
fn test_rust_gen_global_refs_fallback() {
    let tmp = tempfile::tempdir().unwrap();
    let global_dir = tempfile::tempdir().unwrap();

    // Put a rich reference file in the global dir (flat layout)
    let ref_content = "// serde Reference\nuse serde::{Serialize, Deserialize};\nfn serialize() {}\npub struct Value {}\n";
    std::fs::write(global_dir.path().join("lib_serde.rs"), ref_content).unwrap();

    let gen = polyref::generate::rust::RustGenerator;
    let dep = Dependency {
        name: "serde".to_string(),
        version: "1.0".to_string(),
        language: Language::Rust,
        source_file: "Cargo.toml".to_string(),
    };

    // No local ref, but global ref exists → should use global
    let result = gen.generate(&dep, tmp.path(), Some(global_dir.path())).unwrap();
    assert!(result.raw_content.contains("Serialize"));
    assert!(result.raw_content.contains("serialize"));
    assert!(result.entries.iter().any(|e| e.name == "Serialize"));
    assert!(result.entries.iter().any(|e| e.kind == EntryKind::Struct && e.name == "Value"));
    // File path should point to global dir, not local
    assert!(result.file_path.starts_with(global_dir.path()));
    // No stub should be written in the local refs dir
    assert!(!tmp.path().join("rust").join("lib_serde.rs").exists());
}

#[test]
fn test_rust_gen_local_ref_takes_priority_over_global() {
    let tmp = tempfile::tempdir().unwrap();
    let global_dir = tempfile::tempdir().unwrap();

    // Put a ref in both local and global
    let local_dir = tmp.path().join("rust");
    std::fs::create_dir_all(&local_dir).unwrap();
    std::fs::write(local_dir.join("lib_serde.rs"), "// LOCAL ref\nfn local_fn() {}\n").unwrap();
    std::fs::write(global_dir.path().join("lib_serde.rs"), "// GLOBAL ref\nfn global_fn() {}\n").unwrap();

    let gen = polyref::generate::rust::RustGenerator;
    let dep = Dependency {
        name: "serde".to_string(),
        version: "1.0".to_string(),
        language: Language::Rust,
        source_file: "Cargo.toml".to_string(),
    };

    let result = gen.generate(&dep, tmp.path(), Some(global_dir.path())).unwrap();
    // Should use local, not global
    assert!(result.raw_content.contains("LOCAL"));
    assert!(!result.raw_content.contains("GLOBAL"));
    assert!(result.entries.iter().any(|e| e.name == "local_fn"));
}

#[test]
fn test_rust_gen_no_global_ref_generates_stub() {
    std::env::set_var("POLYREF_NO_FETCH", "1");
    let tmp = tempfile::tempdir().unwrap();
    let global_dir = tempfile::tempdir().unwrap();
    // Global dir exists but has no matching file

    let gen = polyref::generate::rust::RustGenerator;
    let dep = Dependency {
        name: "nonexistent_crate".to_string(),
        version: "0.1.0".to_string(),
        language: Language::Rust,
        source_file: "Cargo.toml".to_string(),
    };

    let result = gen.generate(&dep, tmp.path(), Some(global_dir.path())).unwrap();
    assert!(result.raw_content.contains("stub"));
    assert!(result.file_path.exists());
    std::env::remove_var("POLYREF_NO_FETCH");
}

#[test]
fn test_python_gen_global_refs_fallback() {
    let tmp = tempfile::tempdir().unwrap();
    let global_dir = tempfile::tempdir().unwrap();

    let ref_content = "# requests Reference\nfrom requests import Session, Response\ndef get(url: str) -> Response: ...\n";
    std::fs::write(global_dir.path().join("lib_requests.py"), ref_content).unwrap();

    let gen = polyref::generate::python::PythonGenerator;
    let dep = Dependency {
        name: "requests".to_string(),
        version: "2.31.0".to_string(),
        language: Language::Python,
        source_file: "requirements.txt".to_string(),
    };

    let result = gen.generate(&dep, tmp.path(), Some(global_dir.path())).unwrap();
    assert!(result.raw_content.contains("Session"));
    assert!(result.entries.iter().any(|e| e.name == "get" && e.kind == EntryKind::Function));
    assert!(result.file_path.starts_with(global_dir.path()));
    assert!(!tmp.path().join("python").join("lib_requests.py").exists());
}

#[test]
fn test_ts_gen_global_refs_fallback() {
    let tmp = tempfile::tempdir().unwrap();
    let global_dir = tempfile::tempdir().unwrap();

    let ref_content = "// react Reference\nimport { useState, useEffect } from 'react';\nfunction useState<S>(initial: S): [S, (s: S) => void];\n";
    std::fs::write(global_dir.path().join("lib_react.ts"), ref_content).unwrap();

    let gen = polyref::generate::typescript::TypeScriptGenerator;
    let dep = Dependency {
        name: "react".to_string(),
        version: "^18.0.0".to_string(),
        language: Language::TypeScript,
        source_file: "package.json".to_string(),
    };

    let result = gen.generate(&dep, tmp.path(), Some(global_dir.path())).unwrap();
    assert!(result.raw_content.contains("useState"));
    assert!(result.file_path.starts_with(global_dir.path()));
    assert!(!tmp.path().join("typescript").join("lib_react.ts").exists());
}

#[test]
fn test_rust_gen_global_refs_none_skips_lookup() {
    std::env::set_var("POLYREF_NO_FETCH", "1");
    let tmp = tempfile::tempdir().unwrap();

    let gen = polyref::generate::rust::RustGenerator;
    let dep = Dependency {
        name: "anyhow".to_string(),
        version: "1.0".to_string(),
        language: Language::Rust,
        source_file: "Cargo.toml".to_string(),
    };

    // global_refs_dir = None → should generate stub
    let result = gen.generate(&dep, tmp.path(), None).unwrap();
    assert!(result.raw_content.contains("stub"));
    std::env::remove_var("POLYREF_NO_FETCH");
}

#[test]
fn test_rust_gen_global_refs_dash_to_underscore() {
    let tmp = tempfile::tempdir().unwrap();
    let global_dir = tempfile::tempdir().unwrap();

    // Crate name has dashes, file uses underscores
    let ref_content = "// serde-json Reference\nfn from_str() {}\n";
    std::fs::write(global_dir.path().join("lib_serde_json.rs"), ref_content).unwrap();

    let gen = polyref::generate::rust::RustGenerator;
    let dep = Dependency {
        name: "serde-json".to_string(),
        version: "1.0".to_string(),
        language: Language::Rust,
        source_file: "Cargo.toml".to_string(),
    };

    let result = gen.generate(&dep, tmp.path(), Some(global_dir.path())).unwrap();
    assert!(result.raw_content.contains("from_str"));
    assert!(result.entries.iter().any(|e| e.name == "from_str"));
}

#[test]
fn test_global_ref_file_is_parsed_not_stub() {
    let tmp = tempfile::tempdir().unwrap();
    let global_dir = tempfile::tempdir().unwrap();

    // Create a realistic reference file with multiple entry types
    let ref_content = r#"// anyhow Reference
// Cargo.toml: anyhow = "1"

use anyhow::{Result, Context, bail};

// ============================================================================
// CORE TYPES
// ============================================================================

pub struct Error {}
pub trait Context {}

// ============================================================================
// MACROS
// ============================================================================

anyhow!("error message");
bail!("fatal error");
ensure!(condition, "msg");

// ============================================================================
// METHODS
// ============================================================================

result.context("msg");
err.chain();
"#;
    std::fs::write(global_dir.path().join("lib_anyhow.rs"), ref_content).unwrap();

    let gen = polyref::generate::rust::RustGenerator;
    let dep = Dependency {
        name: "anyhow".to_string(),
        version: "1.0".to_string(),
        language: Language::Rust,
        source_file: "Cargo.toml".to_string(),
    };

    let result = gen.generate(&dep, tmp.path(), Some(global_dir.path())).unwrap();

    // Verify it's NOT a stub
    assert!(!result.raw_content.contains("stub"));

    // Verify multiple entry types are parsed
    assert!(result.entries.iter().any(|e| e.kind == EntryKind::Module && e.name == "Result"));
    assert!(result.entries.iter().any(|e| e.kind == EntryKind::Module && e.name == "Context"));
    assert!(result.entries.iter().any(|e| e.kind == EntryKind::Struct && e.name == "Error"));
    assert!(result.entries.iter().any(|e| e.kind == EntryKind::Macro && e.name == "anyhow!"));
    assert!(result.entries.iter().any(|e| e.kind == EntryKind::Macro && e.name == "bail!"));
    assert!(result.entries.iter().any(|e| e.kind == EntryKind::Method && e.name == "context"));

    // Verify we got a reasonable number of entries (not just a stub with 0)
    assert!(result.entries.len() >= 8, "Expected at least 8 entries, got {}", result.entries.len());
}
