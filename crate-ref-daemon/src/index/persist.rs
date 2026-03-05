use std::io::Write;
use std::path::Path;

use anyhow::{bail, Result};
use memmap2::MmapOptions;

use super::types::{FlatIndex, IndexEntry};

const MAGIC: &[u8; 8] = b"CREFIDX1";

/// Persist `index` to `path` atomically (write to .tmp, then rename).
pub fn save(index: &FlatIndex, path: &Path) -> Result<()> {
    let tmp_path = path.with_extension("tmp");

    let mut file = std::fs::File::create(&tmp_path)?;

    // Write magic
    file.write_all(MAGIC)?;

    // Write entry count (u64 LE)
    let entry_count = index.entries.len() as u64;
    file.write_all(&entry_count.to_le_bytes())?;

    // Write path pool size (u64 LE)
    let pool_size = index.path_pool.len() as u64;
    file.write_all(&pool_size.to_le_bytes())?;

    // Write entries (each: fingerprint u64 LE, path_offset u32 LE, path_len u32 LE)
    for entry in &index.entries {
        file.write_all(&entry.fingerprint.to_le_bytes())?;
        file.write_all(&entry.path_offset.to_le_bytes())?;
        file.write_all(&entry.path_len.to_le_bytes())?;
    }

    // Write path pool
    file.write_all(&index.path_pool)?;

    file.flush()?;
    drop(file);

    // Atomic rename
    std::fs::rename(&tmp_path, path)?;

    Ok(())
}

/// Load an index from `path`.
/// If `path` does not exist, returns Ok(FlatIndex::new()) -- clean cold start.
/// Uses memmap2 for reading; advises Sequential on Unix.
pub fn load(path: &Path) -> Result<FlatIndex> {
    if !path.exists() {
        return Ok(FlatIndex::new());
    }

    let file = std::fs::File::open(path)?;
    let mmap = unsafe { MmapOptions::new().map(&file)? };

    #[cfg(unix)]
    mmap.advise(memmap2::Advice::Sequential)?;

    let data = &mmap[..];

    // Validate minimum size: magic(8) + entry_count(8) + pool_size(8) = 24
    if data.len() < 24 {
        bail!("invalid index file: too small");
    }

    // Check magic
    if &data[0..8] != MAGIC {
        bail!("invalid index file: bad magic");
    }

    let entry_count = u64::from_le_bytes(data[8..16].try_into().unwrap()) as usize;
    let pool_size = u64::from_le_bytes(data[16..24].try_into().unwrap()) as usize;

    let entries_start = 24;
    let entries_end = entries_start + entry_count * 16;
    let pool_end = entries_end + pool_size;

    if data.len() < pool_end {
        bail!("invalid index file: truncated");
    }

    let mut entries = Vec::with_capacity(entry_count);
    for i in 0..entry_count {
        let offset = entries_start + i * 16;
        let fingerprint = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
        let path_offset = u32::from_le_bytes(data[offset + 8..offset + 12].try_into().unwrap());
        let path_len = u32::from_le_bytes(data[offset + 12..offset + 16].try_into().unwrap());
        entries.push(IndexEntry {
            fingerprint,
            path_offset,
            path_len,
        });
    }

    let path_pool = data[entries_end..pool_end].to_vec();

    Ok(FlatIndex {
        entries,
        path_pool,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_save_and_load_roundtrip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("index.bin");

        let mut index = FlatIndex::new();
        index.insert(100, "path/a.rs");
        index.insert(200, "path/b.rs");
        index.insert(300, "path/c.rs");

        save(&index, &path).unwrap();
        let loaded = load(&path).unwrap();

        assert_eq!(loaded.len(), 3);
        for i in 0..3 {
            assert_eq!(loaded.entries[i].fingerprint, index.entries[i].fingerprint);
            assert_eq!(loaded.path_of(&loaded.entries[i]), index.path_of(&index.entries[i]));
        }
    }

    #[test]
    fn test_load_nonexistent_returns_empty() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("nonexistent.bin");
        let loaded = load(&path).unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn test_atomic_write_no_tmp_after_save() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("index.bin");
        let index = FlatIndex::new();
        save(&index, &path).unwrap();
        assert!(!path.with_extension("tmp").exists());
    }

    #[test]
    fn test_magic_mismatch_returns_err() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("bad.bin");
        // Write enough garbage bytes (at least 24)
        std::fs::write(&path, b"GARBAGE!0000000000000000").unwrap();
        let result = load(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_index_roundtrip() {
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("empty.bin");
        let index = FlatIndex::new();
        save(&index, &path).unwrap();
        let loaded = load(&path).unwrap();
        assert!(loaded.is_empty());
    }
}
