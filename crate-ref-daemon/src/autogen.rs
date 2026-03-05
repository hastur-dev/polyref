use std::path::Path;

use anyhow::Result;

use polyref::detect::{detect_with_skip, Dependency, Language};
use polyref::generate::rust::RustGenerator;
use polyref::generate::Generator;

use crate::builder::update_entry;
use crate::index::FlatIndex;

/// Result of auto-generation: which refs were generated and which were skipped.
#[derive(Debug, Default)]
pub struct AutogenResult {
    /// Dependencies that already had reference files.
    pub skipped: Vec<String>,
    /// Dependencies for which new reference files were generated.
    pub generated: Vec<String>,
    /// Dependencies that failed to generate (name, error message).
    pub failed: Vec<(String, String)>,
}

/// Scan `project_dir` for Rust dependencies and generate missing reference files
/// into `ref_dir`. Returns a summary of what was generated vs skipped.
///
/// Only processes Rust dependencies. Non-Rust deps are silently ignored.
pub fn auto_generate_refs(
    project_dir: &Path,
    ref_dir: &Path,
    index: &mut FlatIndex,
) -> Result<AutogenResult> {
    let mut result = AutogenResult::default();

    // Detect project dependencies
    let project = match detect_with_skip(project_dir, &[]) {
        Ok(p) => p,
        Err(_) => {
            // No Cargo.toml or parse error — not an error, just nothing to do
            return Ok(result);
        }
    };

    let rust_deps: Vec<&Dependency> = project
        .dependencies
        .iter()
        .filter(|d| d.language == Language::Rust)
        .collect();

    if rust_deps.is_empty() {
        return Ok(result);
    }

    // Ensure ref_dir exists
    std::fs::create_dir_all(ref_dir)?;

    let generator = RustGenerator;

    for dep in rust_deps {
        let ref_file_name = format!("lib_{}.rs", dep.name.replace('-', "_"));
        let ref_path = ref_dir.join(&ref_file_name);

        if ref_path.exists() {
            result.skipped.push(dep.name.clone());
            continue;
        }

        // Generator writes to output_dir/rust/lib_<name>.rs (project layout),
        // but the daemon expects flat ref_dir/lib_<name>.rs. We generate into
        // ref_dir (which creates ref_dir/rust/<file>), then move to flat layout.
        match generator.generate(dep, ref_dir, Some(ref_dir)) {
            Ok(ref_file) => {
                // Move from project layout (ref_dir/rust/<file>) to flat layout (ref_dir/<file>)
                if ref_file.file_path != ref_path && ref_file.file_path.exists() {
                    if let Err(e) = std::fs::copy(&ref_file.file_path, &ref_path) {
                        eprintln!(
                            "[autogen] failed to copy {} to flat layout: {}",
                            ref_file_name, e
                        );
                    } else {
                        // Clean up the nested file
                        let _ = std::fs::remove_file(&ref_file.file_path);
                    }
                }
                // Update the daemon index with the newly generated file
                if ref_path.exists() {
                    if let Err(e) = update_entry(index, &ref_path) {
                        eprintln!(
                            "[autogen] generated {} but failed to index: {}",
                            ref_file_name, e
                        );
                    }
                }
                result.generated.push(dep.name.clone());
            }
            Err(e) => {
                result
                    .failed
                    .push((dep.name.clone(), format!("{e:#}")));
            }
        }
    }

    Ok(result)
}

/// Check which Rust dependencies from `project_dir` are missing reference files
/// in `ref_dir`. Returns the list of missing dependency names.
pub fn find_missing_refs(project_dir: &Path, ref_dir: &Path) -> Result<Vec<String>> {
    let project = match detect_with_skip(project_dir, &[]) {
        Ok(p) => p,
        Err(_) => return Ok(vec![]),
    };

    let missing: Vec<String> = project
        .dependencies
        .iter()
        .filter(|d| d.language == Language::Rust)
        .filter(|d| {
            let ref_file = ref_dir.join(format!("lib_{}.rs", d.name.replace('-', "_")));
            !ref_file.exists()
        })
        .map(|d| d.name.clone())
        .collect();

    Ok(missing)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::FlatIndex;

    #[test]
    fn test_autogen_empty_project_dir_no_error() {
        let tmp = tempfile::TempDir::new().unwrap();
        let project_dir = tmp.path().join("no_project");
        std::fs::create_dir_all(&project_dir).unwrap();
        let ref_dir = tmp.path().join("refs");

        let mut index = FlatIndex::new();
        let result = auto_generate_refs(&project_dir, &ref_dir, &mut index).unwrap();

        assert!(result.skipped.is_empty());
        assert!(result.generated.is_empty());
        assert!(result.failed.is_empty());
    }

    #[test]
    fn test_autogen_skips_existing_refs() {
        let tmp = tempfile::TempDir::new().unwrap();
        let project_dir = tmp.path().join("project");
        std::fs::create_dir_all(&project_dir).unwrap();

        // Create a minimal Cargo.toml with a dependency
        let cargo_toml = r#"
[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1"
"#;
        std::fs::write(project_dir.join("Cargo.toml"), cargo_toml).unwrap();

        // Pre-create the ref file so it should be skipped
        let ref_dir = tmp.path().join("refs");
        std::fs::create_dir_all(&ref_dir).unwrap();
        std::fs::write(ref_dir.join("lib_serde.rs"), "// existing ref").unwrap();

        let mut index = FlatIndex::new();
        let result = auto_generate_refs(&project_dir, &ref_dir, &mut index).unwrap();

        assert_eq!(result.skipped, vec!["serde"]);
        assert!(result.generated.is_empty());
    }

    #[test]
    fn test_autogen_detects_missing_deps() {
        let tmp = tempfile::TempDir::new().unwrap();
        let project_dir = tmp.path().join("project");
        std::fs::create_dir_all(&project_dir).unwrap();

        let cargo_toml = r#"
[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1"
anyhow = "1"
"#;
        std::fs::write(project_dir.join("Cargo.toml"), cargo_toml).unwrap();

        // Only serde has a ref file, anyhow is missing
        let ref_dir = tmp.path().join("refs");
        std::fs::create_dir_all(&ref_dir).unwrap();
        std::fs::write(ref_dir.join("lib_serde.rs"), "// serde ref").unwrap();

        let missing = find_missing_refs(&project_dir, &ref_dir).unwrap();
        assert_eq!(missing, vec!["anyhow"]);
    }

    #[test]
    fn test_find_missing_refs_no_cargo_toml() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ref_dir = tmp.path().join("refs");

        let missing = find_missing_refs(tmp.path(), &ref_dir).unwrap();
        assert!(missing.is_empty());
    }

    #[test]
    fn test_autogen_handles_hyphenated_crate_names() {
        let tmp = tempfile::TempDir::new().unwrap();
        let project_dir = tmp.path().join("project");
        std::fs::create_dir_all(&project_dir).unwrap();

        let cargo_toml = r#"
[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
serde-json = "1"
"#;
        std::fs::write(project_dir.join("Cargo.toml"), cargo_toml).unwrap();

        // Create ref file with underscored name (the convention)
        let ref_dir = tmp.path().join("refs");
        std::fs::create_dir_all(&ref_dir).unwrap();
        std::fs::write(ref_dir.join("lib_serde_json.rs"), "// ref").unwrap();

        let missing = find_missing_refs(&project_dir, &ref_dir).unwrap();
        assert!(missing.is_empty());
    }

    #[test]
    fn test_autogen_nonexistent_project_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let project_dir = tmp.path().join("does_not_exist");
        let ref_dir = tmp.path().join("refs");

        let mut index = FlatIndex::new();
        let result = auto_generate_refs(&project_dir, &ref_dir, &mut index).unwrap();

        assert!(result.skipped.is_empty());
        assert!(result.generated.is_empty());
        assert!(result.failed.is_empty());
    }
}
