use std::path::PathBuf;

/// Returns the default output directory for generated reference files.
///
/// Generated refs go to the OS temp directory so they get cleaned out
/// regularly by the OS.
///
/// Resolution order:
/// 1. `POLYREF_DATA_DIR` environment variable → `$POLYREF_DATA_DIR/refs`
/// 2. OS temp directory:
///    - Linux:   `/tmp/polyref/refs`
///    - macOS:   `$TMPDIR/polyref/refs`
///    - Windows: `%TEMP%/polyref/refs`
pub fn default_refs_output_dir() -> Option<PathBuf> {
    data_dir().map(|d| d.join("refs"))
}

fn data_dir() -> Option<PathBuf> {
    if let Ok(dir) = std::env::var("POLYREF_DATA_DIR") {
        if !dir.is_empty() {
            return Some(PathBuf::from(dir));
        }
    }

    Some(std::env::temp_dir().join("polyref"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_refs_output_dir_env() {
        std::env::set_var("POLYREF_DATA_DIR", "/tmp/polyref-gen-test");
        let dir = default_refs_output_dir().unwrap();
        assert_eq!(dir, PathBuf::from("/tmp/polyref-gen-test/refs"));
        std::env::remove_var("POLYREF_DATA_DIR");
    }

    #[test]
    fn test_default_refs_output_dir_temp() {
        std::env::remove_var("POLYREF_DATA_DIR");
        let dir = default_refs_output_dir().unwrap();
        assert!(dir.to_string_lossy().contains("polyref"));
        assert!(dir.ends_with("refs"));
        let temp = std::env::temp_dir();
        assert!(dir.starts_with(&temp));
    }
}
