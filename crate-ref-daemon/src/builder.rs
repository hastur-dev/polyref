use std::path::Path;

use anyhow::Result;
use memmap2::MmapOptions;

use crate::index::FlatIndex;
use crate::simhash::simhash;

/// Scan `ref_dir` for `lib_*.rs` files and build a fresh FlatIndex.
pub fn build_index(ref_dir: &Path) -> Result<FlatIndex> {
    let mut index = FlatIndex::new();
    for entry in std::fs::read_dir(ref_dir)? {
        let entry = entry?;
        let path = entry.path();
        if is_ref_file(&path) {
            let fp = fingerprint_file(&path)?;
            index.insert(fp, &path.to_string_lossy());
        }
    }
    Ok(index)
}

/// Update a single file's entry (add if new, update fingerprint if existing).
pub fn update_entry(index: &mut FlatIndex, path: &Path) -> Result<()> {
    let fp = fingerprint_file(path)?;
    let path_str = path.to_string_lossy().to_string();
    match index.find_by_path(&path_str) {
        Some(i) => index.update(i, fp),
        None => {
            index.insert(fp, &path_str);
        }
    }
    Ok(())
}

/// Remove a file's entry from the index (no-op if not present).
pub fn remove_entry(index: &mut FlatIndex, path: &Path) {
    let path_str = path.to_string_lossy().to_string();
    if let Some(i) = index.find_by_path(&path_str) {
        index.remove(i);
    }
}

/// Returns true if `path` is a `lib_*.rs` reference file.
fn is_ref_file(path: &Path) -> bool {
    path.extension().map(|e| e == "rs").unwrap_or(false)
        && path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.starts_with("lib_"))
            .unwrap_or(false)
}

/// Compute a SimHash fingerprint for the contents of `path`.
/// Uses memmap2; advises Sequential on Unix.
fn fingerprint_file(path: &Path) -> Result<u64> {
    let file = std::fs::File::open(path)?;
    let metadata = file.metadata()?;
    if metadata.len() == 0 {
        return Ok(simhash(""));
    }
    let mmap = unsafe { MmapOptions::new().map(&file)? };
    #[cfg(unix)]
    mmap.advise(memmap2::Advice::Sequential)?;
    let text = std::str::from_utf8(&mmap).unwrap_or("");
    Ok(simhash(text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_index_empty_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let index = build_index(tmp.path()).unwrap();
        assert!(index.is_empty());
    }

    #[test]
    fn test_build_index_finds_lib_files() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("lib_foo.rs"), "fn foo() {}").unwrap();
        std::fs::write(tmp.path().join("lib_bar.rs"), "fn bar() {}").unwrap();
        std::fs::write(tmp.path().join("lib_baz.rs"), "fn baz() {}").unwrap();
        let index = build_index(tmp.path()).unwrap();
        assert_eq!(index.len(), 3);
    }

    #[test]
    fn test_build_index_ignores_non_lib_files() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("lib_test.rs"), "fn test() {}").unwrap();
        std::fs::write(tmp.path().join("main.rs"), "fn main() {}").unwrap();
        std::fs::write(tmp.path().join("mod.rs"), "mod foo;").unwrap();
        let index = build_index(tmp.path()).unwrap();
        assert_eq!(index.len(), 1);
    }

    #[test]
    fn test_update_entry_new_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let file_path = tmp.path().join("lib_new.rs");
        std::fs::write(&file_path, "fn new() { let x = 1; }").unwrap();
        let mut index = FlatIndex::new();
        update_entry(&mut index, &file_path).unwrap();
        assert_eq!(index.len(), 1);
    }

    #[test]
    fn test_update_entry_existing_changes_fingerprint() {
        let tmp = tempfile::TempDir::new().unwrap();
        let file_path = tmp.path().join("lib_a.rs");
        std::fs::write(&file_path, "hello world foo bar baz qux").unwrap();
        let mut index = FlatIndex::new();
        update_entry(&mut index, &file_path).unwrap();
        let fp1 = index.entries[0].fingerprint;

        std::fs::write(&file_path, "completely different content baz qux alpha beta gamma").unwrap();
        update_entry(&mut index, &file_path).unwrap();
        let fp2 = index.entries[0].fingerprint;

        assert_ne!(fp1, fp2);
    }

    #[test]
    fn test_remove_entry_existing() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(tmp.path().join("lib_a.rs"), "fn a() {}").unwrap();
        std::fs::write(tmp.path().join("lib_b.rs"), "fn b() {}").unwrap();
        let mut index = build_index(tmp.path()).unwrap();
        assert_eq!(index.len(), 2);
        let path = tmp.path().join("lib_a.rs");
        remove_entry(&mut index, &path);
        assert_eq!(index.len(), 1);
    }

    #[test]
    fn test_remove_entry_nonexistent() {
        let mut index = FlatIndex::new();
        let path = std::path::PathBuf::from("nonexistent.rs");
        remove_entry(&mut index, &path); // should not panic
        assert_eq!(index.len(), 0);
    }
}
