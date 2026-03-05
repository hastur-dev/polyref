use polyref_gen::python_gen::{
    parse_pyi_stub, parse_pyi_stub_str, generate_polyref_file, PyEntryKind,
};
use std::path::Path;

fn fixture_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("sample_module.pyi")
}

#[test]
fn test_parse_pyi_module_name() {
    let output = parse_pyi_stub(&fixture_path()).unwrap();
    assert_eq!(output.module_name, "sample_module");
}

#[test]
fn test_parse_pyi_version() {
    let output = parse_pyi_stub(&fixture_path()).unwrap();
    assert_eq!(output.version, "2.0.0");
}

#[test]
fn test_parse_pyi_finds_functions() {
    let output = parse_pyi_stub(&fixture_path()).unwrap();
    let funcs: Vec<_> = output
        .entries
        .iter()
        .filter(|e| e.kind == PyEntryKind::Function)
        .collect();
    assert_eq!(funcs.len(), 2);
    let greet = funcs.iter().find(|e| e.name == "greet").unwrap();
    assert_eq!(greet.arg_count, 1);
    let add = funcs.iter().find(|e| e.name == "add").unwrap();
    assert_eq!(add.arg_count, 2);
}

#[test]
fn test_parse_pyi_finds_class() {
    let output = parse_pyi_stub(&fixture_path()).unwrap();
    let classes: Vec<_> = output
        .entries
        .iter()
        .filter(|e| e.kind == PyEntryKind::Class)
        .collect();
    assert_eq!(classes.len(), 1);
    assert_eq!(classes[0].name, "Calculator");
}

#[test]
fn test_parse_pyi_finds_methods() {
    let output = parse_pyi_stub(&fixture_path()).unwrap();
    let methods: Vec<_> = output
        .entries
        .iter()
        .filter(|e| e.kind == PyEntryKind::Method && e.parent.as_deref() == Some("Calculator"))
        .collect();
    // __init__, add, multiply, reset
    assert_eq!(methods.len(), 4);
    let add = methods.iter().find(|e| e.name == "add").unwrap();
    assert_eq!(add.arg_count, 1); // self excluded
}

#[test]
fn test_generate_polyref_contains_lang() {
    let output = parse_pyi_stub(&fixture_path()).unwrap();
    let content = generate_polyref_file(&output);
    assert!(content.contains("@lang python"));
    assert!(content.contains("@module sample_module"));
}

#[test]
fn test_generate_polyref_contains_class() {
    let output = parse_pyi_stub(&fixture_path()).unwrap();
    let content = generate_polyref_file(&output);
    assert!(content.contains("@class Calculator"));
}

#[test]
fn test_parse_pyi_empty_content() {
    let path = Path::new("empty.pyi");
    let output = parse_pyi_stub_str("", path).unwrap();
    assert!(output.entries.is_empty());
}
