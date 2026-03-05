use polyref_gen::typescript_gen::{
    parse_dts_file, parse_dts_str, generate_polyref_file, TsEntryKind,
};
use std::path::Path;

fn fixture_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("sample_lib.d.ts")
}

#[test]
fn test_parse_dts_module_name() {
    let output = parse_dts_file(&fixture_path()).unwrap();
    assert_eq!(output.module_name, "sample_lib.d");
}

#[test]
fn test_parse_dts_version() {
    let output = parse_dts_file(&fixture_path()).unwrap();
    assert_eq!(output.version, "3.1.0");
}

#[test]
fn test_parse_dts_finds_functions() {
    let output = parse_dts_file(&fixture_path()).unwrap();
    let funcs: Vec<_> = output
        .entries
        .iter()
        .filter(|e| e.kind == TsEntryKind::Function)
        .collect();
    assert_eq!(funcs.len(), 2);
    let create_app = funcs.iter().find(|e| e.name == "createApp").unwrap();
    assert_eq!(create_app.arg_count, 1);
    let ver = funcs.iter().find(|e| e.name == "version").unwrap();
    assert_eq!(ver.arg_count, 0);
}

#[test]
fn test_parse_dts_finds_interface() {
    let output = parse_dts_file(&fixture_path()).unwrap();
    let ifaces: Vec<_> = output
        .entries
        .iter()
        .filter(|e| e.kind == TsEntryKind::Interface)
        .collect();
    assert_eq!(ifaces.len(), 1);
    assert_eq!(ifaces[0].name, "AppConfig");
}

#[test]
fn test_parse_dts_finds_class() {
    let output = parse_dts_file(&fixture_path()).unwrap();
    let classes: Vec<_> = output
        .entries
        .iter()
        .filter(|e| e.kind == TsEntryKind::Class)
        .collect();
    assert_eq!(classes.len(), 1);
    assert_eq!(classes[0].name, "App");
}

#[test]
fn test_parse_dts_finds_methods() {
    let output = parse_dts_file(&fixture_path()).unwrap();
    let methods: Vec<_> = output
        .entries
        .iter()
        .filter(|e| e.kind == TsEntryKind::Method && e.parent.as_deref() == Some("App"))
        .collect();
    assert_eq!(methods.len(), 3); // start, stop, use
    let start = methods.iter().find(|e| e.name == "start").unwrap();
    assert_eq!(start.arg_count, 1);
    let stop = methods.iter().find(|e| e.name == "stop").unwrap();
    assert_eq!(stop.arg_count, 0);
}

#[test]
fn test_parse_dts_finds_fields() {
    let output = parse_dts_file(&fixture_path()).unwrap();
    let fields: Vec<_> = output
        .entries
        .iter()
        .filter(|e| e.kind == TsEntryKind::Field)
        .collect();
    // AppConfig: name, debug; App: name
    assert!(fields.len() >= 2);
}

#[test]
fn test_generate_polyref_contains_lang() {
    let output = parse_dts_file(&fixture_path()).unwrap();
    let content = generate_polyref_file(&output);
    assert!(content.contains("@lang typescript"));
}

#[test]
fn test_parse_dts_empty() {
    let path = Path::new("empty.d.ts");
    let output = parse_dts_str("", path).unwrap();
    assert!(output.entries.is_empty());
}
