use polyref::commands::enforce::{load_refs_from_dir, parse_ref_entries};
use polyref::detect::Language;
use std::path::Path;

fn refs_dir() -> &'static Path {
    Path::new("refs")
}

#[test]
fn test_std_dir_exists() {
    let std_dir = refs_dir().join("std");
    assert!(std_dir.exists(), "refs/std/ directory must exist");
}

#[test]
fn test_std_collections_ref_parses() {
    let content = std::fs::read_to_string("refs/std/std_collections.rs").unwrap();
    let entries = parse_ref_entries(&content, Language::Rust);
    assert!(!entries.is_empty(), "collections ref should produce entries");

    let method_names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(method_names.contains(&"insert"), "should have 'insert' method");
    assert!(method_names.contains(&"get"), "should have 'get' method");
    assert!(method_names.contains(&"contains_key"), "should have 'contains_key'");
}

#[test]
fn test_std_string_ref_parses() {
    let content = std::fs::read_to_string("refs/std/std_string.rs").unwrap();
    let entries = parse_ref_entries(&content, Language::Rust);
    assert!(!entries.is_empty(), "string ref should produce entries");

    let method_names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(method_names.contains(&"push_str"), "should have 'push_str'");
    assert!(method_names.contains(&"trim"), "should have 'trim'");
    assert!(method_names.contains(&"contains"), "should have 'contains'");
    assert!(method_names.contains(&"replace"), "should have 'replace'");
}

#[test]
fn test_std_option_ref_parses() {
    let content = std::fs::read_to_string("refs/std/std_option.rs").unwrap();
    let entries = parse_ref_entries(&content, Language::Rust);

    let method_names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(method_names.contains(&"unwrap"), "should have 'unwrap'");
    assert!(method_names.contains(&"map"), "should have 'map'");
    assert!(method_names.contains(&"and_then"), "should have 'and_then'");
    assert!(method_names.contains(&"is_some"), "should have 'is_some'");
}

#[test]
fn test_std_result_ref_parses() {
    let content = std::fs::read_to_string("refs/std/std_result.rs").unwrap();
    let entries = parse_ref_entries(&content, Language::Rust);

    let method_names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(method_names.contains(&"unwrap"), "should have 'unwrap'");
    assert!(method_names.contains(&"map_err"), "should have 'map_err'");
    assert!(method_names.contains(&"is_ok"), "should have 'is_ok'");
    assert!(method_names.contains(&"is_err"), "should have 'is_err'");
}

#[test]
fn test_std_io_ref_parses() {
    let content = std::fs::read_to_string("refs/std/std_io.rs").unwrap();
    let entries = parse_ref_entries(&content, Language::Rust);
    assert!(!entries.is_empty(), "io ref should produce entries");

    let method_names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(method_names.contains(&"read"), "should have 'read'");
    assert!(method_names.contains(&"write"), "should have 'write'");
    assert!(method_names.contains(&"flush"), "should have 'flush'");
}

#[test]
fn test_std_sync_ref_parses() {
    let content = std::fs::read_to_string("refs/std/std_sync.rs").unwrap();
    let entries = parse_ref_entries(&content, Language::Rust);
    assert!(!entries.is_empty(), "sync ref should produce entries");

    let method_names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(method_names.contains(&"lock"), "should have 'lock'");
    assert!(method_names.contains(&"send"), "should have 'send'");
    assert!(method_names.contains(&"recv"), "should have 'recv'");
}

#[test]
fn test_std_iter_ref_parses() {
    let content = std::fs::read_to_string("refs/std/std_iter.rs").unwrap();
    let entries = parse_ref_entries(&content, Language::Rust);
    assert!(!entries.is_empty(), "iter ref should produce entries");

    let method_names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();
    assert!(method_names.contains(&"map"), "should have 'map'");
    assert!(method_names.contains(&"filter"), "should have 'filter'");
    assert!(method_names.contains(&"collect"), "should have 'collect'");
    assert!(method_names.contains(&"fold"), "should have 'fold'");
    assert!(method_names.contains(&"enumerate"), "should have 'enumerate'");
}

#[test]
fn test_std_refs_loaded_via_loader() {
    let ref_files = load_refs_from_dir(refs_dir(), None, Language::Rust).unwrap();
    // Should include files from refs/rust/ AND refs/std/
    let lib_names: Vec<&str> = ref_files.iter().map(|rf| rf.library_name.as_str()).collect();

    // At least some stdlib refs should be loaded
    let has_std = lib_names.iter().any(|n| n.starts_with("std_"));
    assert!(has_std, "loader should find refs/std/ files, got: {:?}", lib_names);
}

#[test]
fn test_std_refs_entry_count() {
    // Each stdlib ref should produce a reasonable number of entries
    for (file, min_expected) in &[
        ("refs/std/std_collections.rs", 20),
        ("refs/std/std_string.rs", 15),
        ("refs/std/std_option.rs", 10),
        ("refs/std/std_result.rs", 10),
        ("refs/std/std_io.rs", 5),
        ("refs/std/std_sync.rs", 5),
        ("refs/std/std_iter.rs", 15),
    ] {
        let content = std::fs::read_to_string(file).unwrap();
        let entries = parse_ref_entries(&content, Language::Rust);
        assert!(entries.len() >= *min_expected,
            "{} should have at least {} entries, got {}",
            file, min_expected, entries.len());
    }
}
