use std::path::PathBuf;

/// Returns the polyref data directory for generated reference files.
///
/// Generated refs are stored in the OS temp directory so they get
/// cleaned out regularly. This is NOT for curated/bundled refs
/// (those live in-repo at `refs/`).
///
/// Resolution order:
/// 1. `POLYREF_DATA_DIR` environment variable (if set)
/// 2. OS temp directory + `polyref/`:
///    - Linux:   `$TMPDIR/polyref` or `/tmp/polyref`
///    - macOS:   `$TMPDIR/polyref` (usually `/var/folders/.../polyref`)
///    - Windows: `%TEMP%/polyref`
pub fn data_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("POLYREF_DATA_DIR") {
        if !dir.is_empty() {
            return Some(PathBuf::from(dir));
        }
    }

    Some(std::env::temp_dir().join("polyref"))
}

/// Returns the refs subdirectory within the data dir.
/// e.g. `/tmp/polyref/refs/`
pub fn global_refs_dir() -> Option<PathBuf> {
    data_dir().map(|d| d.join("refs"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_dir_env_override() {
        std::env::set_var("POLYREF_DATA_DIR", "/tmp/polyref-test-data");
        let dir = data_dir().unwrap();
        assert_eq!(dir, PathBuf::from("/tmp/polyref-test-data"));
        std::env::remove_var("POLYREF_DATA_DIR");
    }

    #[test]
    fn test_global_refs_dir_env_override() {
        std::env::set_var("POLYREF_DATA_DIR", "/tmp/polyref-test-data2");
        let dir = global_refs_dir().unwrap();
        assert_eq!(dir, PathBuf::from("/tmp/polyref-test-data2/refs"));
        std::env::remove_var("POLYREF_DATA_DIR");
    }

    #[test]
    fn test_data_dir_returns_temp_based() {
        std::env::remove_var("POLYREF_DATA_DIR");
        let dir = data_dir().unwrap();
        assert!(dir.ends_with("polyref"));
        // Should be under the OS temp directory
        let temp = std::env::temp_dir();
        assert!(dir.starts_with(&temp));
    }

    #[test]
    fn test_global_refs_dir_returns_some() {
        std::env::remove_var("POLYREF_DATA_DIR");
        let dir = global_refs_dir().unwrap();
        assert!(dir.ends_with("refs"));
        assert!(dir.to_string_lossy().contains("polyref"));
    }
}
