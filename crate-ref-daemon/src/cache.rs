use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};

/// Hash file bytes to a u64 for change detection (not cryptographic).
pub fn content_hash(data: &[u8]) -> u64 {
    let mut h = DefaultHasher::new();
    data.hash(&mut h);
    h.finish()
}

/// Cached result for a single file analysis.
#[derive(Debug, Clone)]
pub struct CachedResult {
    pub content_hash: u64,
    pub fingerprint: u64,
    pub issues: Vec<String>,
}

/// In-memory cache: file path -> last analysis result.
/// On a hook event, if content_hash matches the stored entry,
/// skip re-analysis entirely.
pub struct ContentCache {
    inner: HashMap<String, CachedResult>,
}

impl Default for ContentCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentCache {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    /// Return the cached result for `path` only if `current_hash` matches.
    pub fn get(&self, path: &str, current_hash: u64) -> Option<&CachedResult> {
        self.inner
            .get(path)
            .filter(|r| r.content_hash == current_hash)
    }

    /// Store or update a result for `path`.
    pub fn insert(&mut self, path: String, result: CachedResult) {
        self.inner.insert(path, result);
    }

    /// Remove the cache entry for `path`.
    pub fn invalidate(&mut self, path: &str) {
        self.inner.remove(path);
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_miss_empty() {
        let cache = ContentCache::new();
        assert!(cache.get("foo.rs", 42).is_none());
    }

    #[test]
    fn test_cache_hit_correct_hash() {
        let mut cache = ContentCache::new();
        cache.insert(
            "foo.rs".to_string(),
            CachedResult {
                content_hash: 42,
                fingerprint: 100,
                issues: vec![],
            },
        );
        assert!(cache.get("foo.rs", 42).is_some());
    }

    #[test]
    fn test_cache_miss_wrong_hash() {
        let mut cache = ContentCache::new();
        cache.insert(
            "foo.rs".to_string(),
            CachedResult {
                content_hash: 42,
                fingerprint: 100,
                issues: vec![],
            },
        );
        assert!(cache.get("foo.rs", 99).is_none());
    }

    #[test]
    fn test_cache_invalidate() {
        let mut cache = ContentCache::new();
        cache.insert(
            "foo.rs".to_string(),
            CachedResult {
                content_hash: 42,
                fingerprint: 100,
                issues: vec![],
            },
        );
        cache.invalidate("foo.rs");
        assert!(cache.get("foo.rs", 42).is_none());
    }

    #[test]
    fn test_cache_len() {
        let mut cache = ContentCache::new();
        for i in 0..3 {
            cache.insert(
                format!("file{i}.rs"),
                CachedResult {
                    content_hash: i,
                    fingerprint: i,
                    issues: vec![],
                },
            );
        }
        assert_eq!(cache.len(), 3);
        cache.invalidate("file1.rs");
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_content_hash_deterministic() {
        let data = b"hello world";
        assert_eq!(content_hash(data), content_hash(data));
    }

    #[test]
    fn test_content_hash_differs() {
        assert_ne!(content_hash(b"hello"), content_hash(b"world"));
    }
}
