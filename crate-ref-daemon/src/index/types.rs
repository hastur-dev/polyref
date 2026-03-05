/// A single entry: 8-byte fingerprint + 8-byte path reference = 16 bytes.
/// 500K entries = ~8MB, fits comfortably in L3 cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexEntry {
    pub fingerprint: u64,
    /// Byte offset into the path pool where this entry's path string starts.
    pub path_offset: u32,
    /// Byte length of the path string (excludes null terminator).
    pub path_len: u32,
}

/// The in-memory flat index.
pub struct FlatIndex {
    /// Packed entries -- the hot scan target.
    pub entries: Vec<IndexEntry>,
    /// All path strings concatenated. Each path is followed by a null byte.
    pub path_pool: Vec<u8>,
}

impl Default for FlatIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl FlatIndex {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            path_pool: Vec::new(),
        }
    }

    /// Insert a new entry. Returns its index in `entries`.
    pub fn insert(&mut self, fingerprint: u64, path: &str) -> usize {
        let path_offset = self.path_pool.len() as u32;
        let path_len = path.len() as u32;
        self.path_pool.extend_from_slice(path.as_bytes());
        self.path_pool.push(0); // null terminator
        let idx = self.entries.len();
        self.entries.push(IndexEntry {
            fingerprint,
            path_offset,
            path_len,
        });
        idx
    }

    /// Update the fingerprint of an existing entry.
    pub fn update(&mut self, index: usize, fingerprint: u64) {
        self.entries[index].fingerprint = fingerprint;
    }

    /// Retrieve the path string for an entry.
    pub fn path_of(&self, entry: &IndexEntry) -> &str {
        let start = entry.path_offset as usize;
        let end = start + entry.path_len as usize;
        std::str::from_utf8(&self.path_pool[start..end]).expect("path pool contains valid UTF-8")
    }

    /// Find the entry index for a given path string.
    pub fn find_by_path(&self, path: &str) -> Option<usize> {
        self.entries.iter().position(|e| self.path_of(e) == path)
    }

    /// Remove entry by index using swap-remove (O(1)).
    pub fn remove(&mut self, index: usize) {
        self.entries.swap_remove(index);
        // Note: path_pool bytes are not reclaimed (acceptable -- pool is append-only)
    }

    /// Linear scan: return (entry_index, hamming_distance) for all entries
    /// within `threshold` Hamming distance of `query`.
    pub fn query(&self, query: u64, threshold: u32) -> Vec<(usize, u32)> {
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                let d = crate::simhash::hamming(e.fingerprint, query);
                if d <= threshold {
                    Some((i, d))
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_path_of() {
        let mut idx = FlatIndex::new();
        idx.insert(100, "path/a.rs");
        idx.insert(200, "path/b.rs");
        assert_eq!(idx.path_of(&idx.entries[0]), "path/a.rs");
        assert_eq!(idx.path_of(&idx.entries[1]), "path/b.rs");
    }

    #[test]
    fn test_find_by_path_hit() {
        let mut idx = FlatIndex::new();
        idx.insert(100, "path/a.rs");
        assert_eq!(idx.find_by_path("path/a.rs"), Some(0));
    }

    #[test]
    fn test_find_by_path_miss() {
        let idx = FlatIndex::new();
        assert_eq!(idx.find_by_path("nonexistent"), None);
    }

    #[test]
    fn test_update_fingerprint() {
        let mut idx = FlatIndex::new();
        idx.insert(1, "test.rs");
        idx.update(0, 99);
        assert_eq!(idx.entries[0].fingerprint, 99);
    }

    #[test]
    fn test_remove_by_swap() {
        let mut idx = FlatIndex::new();
        idx.insert(1, "a");
        idx.insert(2, "b");
        idx.insert(3, "c");
        idx.remove(1); // removes "b", swaps "c" into index 1
        assert_eq!(idx.len(), 2);
        // Both remaining paths should be retrievable
        let paths: Vec<&str> = idx.entries.iter().map(|e| idx.path_of(e)).collect();
        assert!(paths.contains(&"a"));
        assert!(paths.contains(&"c"));
    }

    #[test]
    fn test_query_exact_match() {
        let mut idx = FlatIndex::new();
        idx.insert(0xDEAD, "test.rs");
        let results = idx.query(0xDEAD, 0);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], (0, 0));
    }

    #[test]
    fn test_query_within_threshold() {
        let mut idx = FlatIndex::new();
        idx.insert(0, "test.rs");
        // 0b11111 has 5 bits set -> hamming distance = 5
        let results = idx.query(0b11111, 8);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1, 5);
    }

    #[test]
    fn test_query_outside_threshold() {
        let mut idx = FlatIndex::new();
        idx.insert(0, "test.rs");
        let results = idx.query(0b11111, 3);
        assert!(results.is_empty());
    }

    #[test]
    fn test_query_empty_index() {
        let idx = FlatIndex::new();
        let results = idx.query(42, 10);
        assert!(results.is_empty());
    }
}
