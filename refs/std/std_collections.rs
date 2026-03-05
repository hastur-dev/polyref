// std::collections Reference — Rust standard library collection types
// Version: std
// Lang: rust

use std::collections;

// ============================================================================
// HashMap
// ============================================================================

impl HashMap {
    fn new() -> HashMap {}                                  // create empty HashMap
    fn with_capacity(capacity: usize) -> HashMap {}         // create with capacity [min_args=1, max_args=1]
    fn insert(&mut self, k: K, v: V) -> Option<V> {}        // insert key-value pair [min_args=2, max_args=2]
    fn get(&self, k: &K) -> Option<&V> {}                   // get value by key [min_args=1, max_args=1]
    fn remove(&mut self, k: &K) -> Option<V> {}             // remove by key [min_args=1, max_args=1]
    fn contains_key(&self, k: &K) -> bool {}                // check if key exists [min_args=1, max_args=1]
    fn len(&self) -> usize {}                               // number of entries [min_args=0, max_args=0]
    fn is_empty(&self) -> bool {}                           // check if empty [min_args=0, max_args=0]
    fn keys(&self) -> Keys<K, V> {}                         // iterate over keys [min_args=0, max_args=0]
    fn values(&self) -> Values<K, V> {}                     // iterate over values [min_args=0, max_args=0]
    fn iter(&self) -> Iter<K, V> {}                         // iterate over key-value pairs [min_args=0, max_args=0]
    fn entry(&mut self, key: K) -> Entry<K, V> {}           // get entry for in-place manipulation [min_args=1, max_args=1]
    fn clear(&mut self) {}                                  // remove all entries [min_args=0, max_args=0]
    fn retain(&mut self, f: F) {}                           // keep only entries matching predicate [min_args=1, max_args=1]
}

// ============================================================================
// HashSet
// ============================================================================

impl HashSet {
    fn new() -> HashSet {}                                  // create empty HashSet
    fn with_capacity(capacity: usize) -> HashSet {}         // create with capacity [min_args=1, max_args=1]
    fn insert(&mut self, value: T) -> bool {}               // insert value [min_args=1, max_args=1]
    fn remove(&mut self, value: &T) -> bool {}              // remove value [min_args=1, max_args=1]
    fn contains(&self, value: &T) -> bool {}                // check membership [min_args=1, max_args=1]
    fn len(&self) -> usize {}                               // number of elements [min_args=0, max_args=0]
    fn is_empty(&self) -> bool {}                           // check if empty [min_args=0, max_args=0]
    fn iter(&self) -> Iter<T> {}                            // iterate over values [min_args=0, max_args=0]
    fn union(&self, other: &HashSet<T>) -> Union<T> {}      // set union [min_args=1, max_args=1]
    fn intersection(&self, other: &HashSet<T>) -> Intersection<T> {} // set intersection [min_args=1, max_args=1]
    fn difference(&self, other: &HashSet<T>) -> Difference<T> {} // set difference [min_args=1, max_args=1]
}

// ============================================================================
// BTreeMap
// ============================================================================

impl BTreeMap {
    fn new() -> BTreeMap {}                                 // create empty BTreeMap
    fn insert(&mut self, k: K, v: V) -> Option<V> {}        // insert key-value pair [min_args=2, max_args=2]
    fn get(&self, k: &K) -> Option<&V> {}                   // get value by key [min_args=1, max_args=1]
    fn remove(&mut self, k: &K) -> Option<V> {}             // remove by key [min_args=1, max_args=1]
    fn contains_key(&self, k: &K) -> bool {}                // check if key exists [min_args=1, max_args=1]
    fn len(&self) -> usize {}                               // number of entries [min_args=0, max_args=0]
    fn iter(&self) -> Iter<K, V> {}                         // iterate in key order [min_args=0, max_args=0]
}

// ============================================================================
// VecDeque
// ============================================================================

impl VecDeque {
    fn new() -> VecDeque {}                                 // create empty VecDeque
    fn with_capacity(capacity: usize) -> VecDeque {}        // create with capacity [min_args=1, max_args=1]
    fn push_back(&mut self, value: T) {}                    // push to back [min_args=1, max_args=1]
    fn push_front(&mut self, value: T) {}                   // push to front [min_args=1, max_args=1]
    fn pop_back(&mut self) -> Option<T> {}                  // pop from back [min_args=0, max_args=0]
    fn pop_front(&mut self) -> Option<T> {}                 // pop from front [min_args=0, max_args=0]
    fn len(&self) -> usize {}                               // number of elements [min_args=0, max_args=0]
    fn is_empty(&self) -> bool {}                           // check if empty [min_args=0, max_args=0]
}

// ============================================================================
// BinaryHeap
// ============================================================================

impl BinaryHeap {
    fn new() -> BinaryHeap {}                               // create empty BinaryHeap
    fn push(&mut self, item: T) {}                          // push item [min_args=1, max_args=1]
    fn pop(&mut self) -> Option<T> {}                       // pop greatest element [min_args=0, max_args=0]
    fn peek(&self) -> Option<&T> {}                         // peek at greatest element [min_args=0, max_args=0]
    fn len(&self) -> usize {}                               // number of elements [min_args=0, max_args=0]
}
