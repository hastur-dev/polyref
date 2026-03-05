//! Final audit tests — verify that the polyref system is complete and consistent.

use std::path::Path;

/// Verify all expected module files exist.
#[test]
fn test_all_source_modules_exist() {
    let modules = [
        "src/lib.rs",
        "src/main.rs",
        "src/config.rs",
        "src/ast/mod.rs",
        "src/ast/rust_ast.rs",
        "src/check/mod.rs",
        "src/check/rust.rs",
        "src/commands/mod.rs",
        "src/commands/enforce.rs",
        "src/typescript_bridge.rs",
    ];
    for m in &modules {
        assert!(
            Path::new(m).exists(),
            "Expected source module '{}' to exist",
            m
        );
    }
}

/// Verify all reference directories exist.
#[test]
fn test_reference_directories_exist() {
    let dirs = ["refs/rust", "refs/std", "refs/ts"];
    for d in &dirs {
        assert!(
            Path::new(d).exists() && Path::new(d).is_dir(),
            "Expected reference directory '{}' to exist",
            d
        );
    }
}

/// Verify that stdlib ref files exist.
#[test]
fn test_stdlib_ref_files_present() {
    let expected = [
        "refs/std/std_collections.rs",
        "refs/std/std_io.rs",
        "refs/std/std_string.rs",
        "refs/std/std_option.rs",
        "refs/std/std_result.rs",
        "refs/std/std_sync.rs",
        "refs/std/std_iter.rs",
    ];
    for f in &expected {
        assert!(Path::new(f).exists(), "Expected stdlib ref '{}' to exist", f);
    }
}

/// Verify that Python stdlib ref files exist.
#[test]
fn test_python_stdlib_refs_present() {
    let expected = [
        "refs/std/pathlib.polyref",
        "refs/std/os.polyref",
        "refs/std/json.polyref",
        "refs/std/subprocess.polyref",
        "refs/std/datetime.polyref",
        "refs/std/typing.polyref",
    ];
    for f in &expected {
        assert!(
            Path::new(f).exists(),
            "Expected Python stdlib ref '{}' to exist",
            f
        );
    }
}

/// Verify TypeScript ref files exist.
#[test]
fn test_typescript_refs_present() {
    let expected = [
        "refs/ts/express.polyref",
        "refs/ts/axios.polyref",
        "refs/ts/lodash.polyref",
        "refs/ts/react.polyref",
        "refs/ts/zod.polyref",
    ];
    for f in &expected {
        assert!(Path::new(f).exists(), "Expected TS ref '{}' to exist", f);
    }
}

/// Verify that tools crates exist.
#[test]
fn test_tools_crates_exist() {
    let crates = [
        "tools/polyref-drift/Cargo.toml",
        "tools/polyref-gen/Cargo.toml",
    ];
    for c in &crates {
        assert!(Path::new(c).exists(), "Expected tool crate '{}' to exist", c);
    }
}

/// Verify that stdlib ref files have a Version header.
#[test]
fn test_stdlib_ref_files_have_version() {
    for entry in std::fs::read_dir("refs/std").unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            let content = std::fs::read_to_string(&path).unwrap();
            assert!(
                content.contains("Version:"),
                "Stdlib ref file {:?} missing Version header",
                path
            );
        }
    }
}

/// Verify polyref v2 files have @lang directive.
#[test]
fn test_polyref_v2_files_have_lang() {
    for dir in &["refs/std", "refs/ts"] {
        if !Path::new(dir).exists() {
            continue;
        }
        for entry in std::fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("polyref") {
                let content = std::fs::read_to_string(&path).unwrap();
                assert!(
                    content.contains("@lang "),
                    "Polyref file {:?} missing @lang directive",
                    path
                );
            }
        }
    }
}
