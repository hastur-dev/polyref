use super::shingles::bigrams;

const FNV_OFFSET: u64 = 14695981039346656037;
const FNV_PRIME: u64 = 1099511628211;

/// FNV-1a 64-bit hash of a byte slice.
fn fnv1a(data: &[u8]) -> u64 {
    let mut hash = FNV_OFFSET;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Compute a 64-bit SimHash fingerprint for `text`.
/// Returns 0 for empty input (no shingles).
pub fn simhash(text: &str) -> u64 {
    let mut v = [0i32; 64];
    let mut any = false;

    for shingle in bigrams(text) {
        any = true;
        let h = fnv1a(shingle.as_bytes());
        for (i, slot) in v.iter_mut().enumerate() {
            if (h >> i) & 1 == 1 {
                *slot += 1;
            } else {
                *slot -= 1;
            }
        }
    }

    if !any {
        return 0;
    }

    let mut result = 0u64;
    for (i, &val) in v.iter().enumerate() {
        if val > 0 {
            result |= 1u64 << i;
        }
    }
    result
}

/// Hamming distance between two SimHash fingerprints.
#[inline]
pub fn hamming(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

/// Returns true if two fingerprints are within `threshold` Hamming distance.
#[inline]
pub fn is_similar(a: u64, b: u64, threshold: u32) -> bool {
    hamming(a, b) <= threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simhash_identical() {
        let text = "hello world foo bar baz qux";
        let a = simhash(text);
        let b = simhash(text);
        assert_eq!(a, b);
        assert_eq!(hamming(a, b), 0);
    }

    #[test]
    fn test_simhash_empty() {
        assert_eq!(simhash(""), 0);
    }

    #[test]
    fn test_simhash_single_word() {
        assert_eq!(simhash("hello"), 0);
    }

    #[test]
    fn test_simhash_near_duplicate() {
        let a = simhash("the quick brown fox jumps over the lazy dog");
        let b = simhash("the quick brown fox leaps over the lazy dog");
        assert!(hamming(a, b) <= 15, "hamming was {}", hamming(a, b));
    }

    #[test]
    fn test_simhash_different() {
        let a = simhash("the quick brown fox jumps over the lazy dog");
        let b = simhash("quantum physics electromagnetic radiation wavelength");
        assert!(hamming(a, b) > 8, "hamming was {}", hamming(a, b));
    }

    #[test]
    fn test_hamming_identical() {
        let x: u64 = 0xDEADBEEF_CAFEBABE;
        assert_eq!(hamming(x, x), 0);
    }

    #[test]
    fn test_hamming_max() {
        assert_eq!(hamming(0u64, u64::MAX), 64);
    }

    #[test]
    fn test_is_similar_at_threshold() {
        let a = 0u64;
        let b = 0b11111111u64; // 8 bits set -> Hamming = 8
        assert!(is_similar(a, b, 8));
        assert!(!is_similar(a, b, 7));
    }
}
