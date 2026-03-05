// std::result Reference — Rust Result type
// Version: std
// Lang: rust

use std::result;

// ============================================================================
// Result<T, E>
// ============================================================================

impl Result {
    fn unwrap(self) -> T {}                                 // unwrap or panic [min_args=0, max_args=0]
    fn unwrap_err(self) -> E {}                             // unwrap error or panic [min_args=0, max_args=0]
    fn unwrap_or(self, default: T) -> T {}                  // unwrap or default [min_args=1, max_args=1]
    fn unwrap_or_else(self, op: F) -> T {}                  // unwrap or compute [min_args=1, max_args=1]
    fn unwrap_or_default(self) -> T {}                      // unwrap or Default::default [min_args=0, max_args=0]
    fn expect(self, msg: &str) -> T {}                      // unwrap or panic with message [min_args=1, max_args=1]
    fn expect_err(self, msg: &str) -> E {}                  // unwrap error or panic with message [min_args=1, max_args=1]
    fn is_ok(&self) -> bool {}                              // check if Ok [min_args=0, max_args=0]
    fn is_err(&self) -> bool {}                             // check if Err [min_args=0, max_args=0]
    fn ok(self) -> Option<T> {}                             // convert to Option [min_args=0, max_args=0]
    fn err(self) -> Option<E> {}                            // convert error to Option [min_args=0, max_args=0]
    fn map(self, op: F) -> Result<U, E> {}                  // map Ok value [min_args=1, max_args=1]
    fn map_err(self, op: F) -> Result<T, F> {}              // map Err value [min_args=1, max_args=1]
    fn map_or(self, default: U, f: F) -> U {}               // map or default [min_args=2, max_args=2]
    fn map_or_else(self, default: D, f: F) -> U {}          // map or compute default [min_args=2, max_args=2]
    fn and_then(self, op: F) -> Result<U, E> {}             // flatmap [min_args=1, max_args=1]
    fn or_else(self, op: F) -> Result<T, F> {}              // alternative on error [min_args=1, max_args=1]
    fn as_ref(&self) -> Result<&T, &E> {}                   // convert to Result<&T, &E> [min_args=0, max_args=0]
    fn as_mut(&mut self) -> Result<&mut T, &mut E> {}       // convert to Result<&mut T, &mut E> [min_args=0, max_args=0]
}
