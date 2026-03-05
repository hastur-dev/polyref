use polyref_gen::rustdoc_gen::{
    parse_rustdoc_json, parse_rustdoc_json_str, generate_ref_file, EntryKind,
};
use std::path::Path;

fn fixture_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("sample_crate.json")
}

#[test]
fn test_parse_fixture_crate_name() {
    let output = parse_rustdoc_json(&fixture_path()).unwrap();
    assert_eq!(output.crate_name, "sample_crate");
}

#[test]
fn test_parse_fixture_crate_version() {
    let output = parse_rustdoc_json(&fixture_path()).unwrap();
    assert_eq!(output.crate_version, "1.2.3");
}

#[test]
fn test_parse_fixture_finds_free_function() {
    let output = parse_rustdoc_json(&fixture_path()).unwrap();
    let add = output.entries.iter().find(|e| e.name == "add").unwrap();
    assert_eq!(add.kind, EntryKind::Function);
    assert_eq!(add.arg_count, 2);
    assert!(add.parent.is_none());
}

#[test]
fn test_parse_fixture_finds_struct() {
    let output = parse_rustdoc_json(&fixture_path()).unwrap();
    let s = output.entries.iter().find(|e| e.name == "MyStruct").unwrap();
    assert_eq!(s.kind, EntryKind::Struct);
}

#[test]
fn test_parse_fixture_finds_enum() {
    let output = parse_rustdoc_json(&fixture_path()).unwrap();
    let e = output.entries.iter().find(|e| e.name == "MyEnum").unwrap();
    assert_eq!(e.kind, EntryKind::Enum);
}

#[test]
fn test_parse_fixture_finds_associated_function() {
    let output = parse_rustdoc_json(&fixture_path()).unwrap();
    let new_fn = output.entries.iter().find(|e| e.name == "new").unwrap();
    assert_eq!(new_fn.kind, EntryKind::AssociatedFunction);
    assert_eq!(new_fn.parent.as_deref(), Some("MyStruct"));
    assert_eq!(new_fn.arg_count, 1);
}

#[test]
fn test_parse_fixture_finds_method() {
    let output = parse_rustdoc_json(&fixture_path()).unwrap();
    let get = output.entries.iter().find(|e| e.name == "get_value").unwrap();
    assert_eq!(get.kind, EntryKind::Method);
    assert_eq!(get.parent.as_deref(), Some("MyStruct"));
    assert_eq!(get.arg_count, 0); // &self is not counted
}

#[test]
fn test_parse_fixture_finds_variants() {
    let output = parse_rustdoc_json(&fixture_path()).unwrap();
    let variants: Vec<_> = output
        .entries
        .iter()
        .filter(|e| e.kind == EntryKind::Variant)
        .collect();
    assert_eq!(variants.len(), 2);
}

#[test]
fn test_generate_ref_file_contains_version() {
    let output = parse_rustdoc_json(&fixture_path()).unwrap();
    let content = generate_ref_file(&output);
    assert!(content.contains("// Version: 1.2.3"));
}

#[test]
fn test_generate_ref_file_contains_impl_block() {
    let output = parse_rustdoc_json(&fixture_path()).unwrap();
    let content = generate_ref_file(&output);
    assert!(content.contains("impl MyStruct {"));
    assert!(content.contains("new("));
    assert!(content.contains("get_value("));
}

#[test]
fn test_parse_invalid_json() {
    let result = parse_rustdoc_json_str("not json");
    assert!(result.is_err());
}

#[test]
fn test_parse_missing_index() {
    let result = parse_rustdoc_json_str(r#"{"root": "0:0:0"}"#);
    assert!(result.is_err());
}
