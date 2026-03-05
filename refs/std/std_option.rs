// std::option Reference — Rust Option type
// Version: std
// Lang: rust

use std::option;

// ============================================================================
// Option<T>
// ============================================================================

impl Option {
    fn unwrap(self) -> T {}                                 // unwrap or panic [min_args=0, max_args=0]
    fn unwrap_or(self, default: T) -> T {}                  // unwrap or default [min_args=1, max_args=1]
    fn unwrap_or_else(self, f: F) -> T {}                   // unwrap or compute [min_args=1, max_args=1]
    fn unwrap_or_default(self) -> T {}                      // unwrap or Default::default [min_args=0, max_args=0]
    fn expect(self, msg: &str) -> T {}                      // unwrap or panic with message [min_args=1, max_args=1]
    fn is_some(&self) -> bool {}                            // check if Some [min_args=0, max_args=0]
    fn is_none(&self) -> bool {}                            // check if None [min_args=0, max_args=0]
    fn map(self, f: F) -> Option<U> {}                      // map inner value [min_args=1, max_args=1]
    fn map_or(self, default: U, f: F) -> U {}               // map or default [min_args=2, max_args=2]
    fn map_or_else(self, default: D, f: F) -> U {}          // map or compute default [min_args=2, max_args=2]
    fn and_then(self, f: F) -> Option<U> {}                 // flatmap [min_args=1, max_args=1]
    fn or(self, optb: Option<T>) -> Option<T> {}            // return self or alternative [min_args=1, max_args=1]
    fn or_else(self, f: F) -> Option<T> {}                  // return self or compute [min_args=1, max_args=1]
    fn filter(self, predicate: P) -> Option<T> {}           // filter by predicate [min_args=1, max_args=1]
    fn as_ref(&self) -> Option<&T> {}                       // convert to Option<&T> [min_args=0, max_args=0]
    fn as_mut(&mut self) -> Option<&mut T> {}               // convert to Option<&mut T> [min_args=0, max_args=0]
    fn take(&mut self) -> Option<T> {}                      // take value, leaving None [min_args=0, max_args=0]
    fn replace(&mut self, value: T) -> Option<T> {}         // replace value [min_args=1, max_args=1]
    fn zip(self, other: Option<U>) -> Option<(T, U)> {}     // zip two Options [min_args=1, max_args=1]
    fn flatten(self) -> Option<T> {}                        // flatten Option<Option<T>> [min_args=0, max_args=0]
}
