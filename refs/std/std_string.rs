// std::string Reference — Rust String type
// Version: std
// Lang: rust

use std::string;

// ============================================================================
// String
// ============================================================================

impl String {
    fn new() -> String {}                                   // create empty String
    fn with_capacity(capacity: usize) -> String {}          // create with capacity [min_args=1, max_args=1]
    fn from(s: &str) -> String {}                           // create from str [min_args=1, max_args=1]
    fn push_str(&mut self, string: &str) {}                 // append string slice [min_args=1, max_args=1]
    fn push(&mut self, ch: char) {}                         // append char [min_args=1, max_args=1]
    fn len(&self) -> usize {}                               // byte length [min_args=0, max_args=0]
    fn is_empty(&self) -> bool {}                           // check if empty [min_args=0, max_args=0]
    fn capacity(&self) -> usize {}                          // allocated capacity [min_args=0, max_args=0]
    fn clear(&mut self) {}                                  // remove all content [min_args=0, max_args=0]
    fn truncate(&mut self, new_len: usize) {}               // truncate to length [min_args=1, max_args=1]
    fn insert(&mut self, idx: usize, ch: char) {}           // insert char at index [min_args=2, max_args=2]
    fn insert_str(&mut self, idx: usize, string: &str) {}   // insert string at index [min_args=2, max_args=2]
    fn remove(&mut self, idx: usize) -> char {}             // remove char at index [min_args=1, max_args=1]
    fn contains(&self, pat: &str) -> bool {}                // check if contains pattern [min_args=1, max_args=1]
    fn starts_with(&self, pat: &str) -> bool {}             // check prefix [min_args=1, max_args=1]
    fn ends_with(&self, pat: &str) -> bool {}               // check suffix [min_args=1, max_args=1]
    fn find(&self, pat: &str) -> Option<usize> {}           // find pattern position [min_args=1, max_args=1]
    fn replace(&self, from: &str, to: &str) -> String {}    // replace all occurrences [min_args=2, max_args=2]
    fn replacen(&self, from: &str, to: &str, count: usize) -> String {} // replace N occurrences [min_args=3, max_args=3]
    fn trim(&self) -> &str {}                               // trim whitespace [min_args=0, max_args=0]
    fn trim_start(&self) -> &str {}                         // trim leading whitespace [min_args=0, max_args=0]
    fn trim_end(&self) -> &str {}                           // trim trailing whitespace [min_args=0, max_args=0]
    fn to_uppercase(&self) -> String {}                     // convert to uppercase [min_args=0, max_args=0]
    fn to_lowercase(&self) -> String {}                     // convert to lowercase [min_args=0, max_args=0]
    fn split(&self, pat: &str) -> Split<&str> {}            // split by pattern [min_args=1, max_args=1]
    fn splitn(&self, n: usize, pat: &str) -> SplitN<&str> {} // split N times [min_args=2, max_args=2]
    fn chars(&self) -> Chars {}                             // iterate over chars [min_args=0, max_args=0]
    fn bytes(&self) -> Bytes {}                             // iterate over bytes [min_args=0, max_args=0]
    fn as_str(&self) -> &str {}                             // borrow as str [min_args=0, max_args=0]
    fn as_bytes(&self) -> &[u8] {}                          // view as byte slice [min_args=0, max_args=0]
}
