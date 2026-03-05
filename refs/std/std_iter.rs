// std::iter Reference — Rust Iterator trait and common adapters
// Version: std
// Lang: rust

use std::iter;

// ============================================================================
// Iterator trait methods
// ============================================================================

impl Iterator {
    fn next(&mut self) -> Option<Self::Item> {}             // advance iterator [min_args=0, max_args=0]
    fn collect(self) -> B {}                                // collect into container [min_args=0, max_args=0]
    fn map(self, f: F) -> Map<Self, F> {}                   // transform each element [min_args=1, max_args=1]
    fn filter(self, predicate: P) -> Filter<Self, P> {}     // keep matching elements [min_args=1, max_args=1]
    fn filter_map(self, f: F) -> FilterMap<Self, F> {}      // filter and transform [min_args=1, max_args=1]
    fn flat_map(self, f: F) -> FlatMap<Self, U, F> {}       // map and flatten [min_args=1, max_args=1]
    fn flatten(self) -> Flatten<Self> {}                    // flatten nested iterators [min_args=0, max_args=0]
    fn for_each(self, f: F) {}                              // consume applying function [min_args=1, max_args=1]
    fn fold(self, init: B, f: F) -> B {}                    // fold/reduce with initial [min_args=2, max_args=2]
    fn reduce(self, f: F) -> Option<Self::Item> {}          // reduce without initial [min_args=1, max_args=1]
    fn any(&mut self, f: F) -> bool {}                      // check if any match [min_args=1, max_args=1]
    fn all(&mut self, f: F) -> bool {}                      // check if all match [min_args=1, max_args=1]
    fn find(&mut self, predicate: P) -> Option<Self::Item> {} // find first match [min_args=1, max_args=1]
    fn find_map(&mut self, f: F) -> Option<B> {}            // find and transform [min_args=1, max_args=1]
    fn position(&mut self, predicate: P) -> Option<usize> {} // find index [min_args=1, max_args=1]
    fn count(self) -> usize {}                              // count elements [min_args=0, max_args=0]
    fn sum(self) -> S {}                                    // sum all elements [min_args=0, max_args=0]
    fn product(self) -> P {}                                // product of all elements [min_args=0, max_args=0]
    fn min(self) -> Option<Self::Item> {}                   // find minimum [min_args=0, max_args=0]
    fn max(self) -> Option<Self::Item> {}                   // find maximum [min_args=0, max_args=0]
    fn min_by_key(self, f: F) -> Option<Self::Item> {}      // minimum by key [min_args=1, max_args=1]
    fn max_by_key(self, f: F) -> Option<Self::Item> {}      // maximum by key [min_args=1, max_args=1]
    fn take(self, n: usize) -> Take<Self> {}                // take first n elements [min_args=1, max_args=1]
    fn skip(self, n: usize) -> Skip<Self> {}                // skip first n elements [min_args=1, max_args=1]
    fn take_while(self, predicate: P) -> TakeWhile<Self, P> {} // take while predicate [min_args=1, max_args=1]
    fn skip_while(self, predicate: P) -> SkipWhile<Self, P> {} // skip while predicate [min_args=1, max_args=1]
    fn chain(self, other: U) -> Chain<Self, U> {}           // chain two iterators [min_args=1, max_args=1]
    fn zip(self, other: U) -> Zip<Self, U> {}               // zip two iterators [min_args=1, max_args=1]
    fn enumerate(self) -> Enumerate<Self> {}                // add index to elements [min_args=0, max_args=0]
    fn peekable(self) -> Peekable<Self> {}                  // make peekable [min_args=0, max_args=0]
    fn cloned(self) -> Cloned<Self> {}                      // clone each element [min_args=0, max_args=0]
    fn copied(self) -> Copied<Self> {}                      // copy each element [min_args=0, max_args=0]
    fn inspect(self, f: F) -> Inspect<Self, F> {}           // inspect each element [min_args=1, max_args=1]
    fn partition(self, f: F) -> (B, B) {}                   // partition into two collections [min_args=1, max_args=1]
    fn unzip(self) -> (A, B) {}                             // unzip pairs [min_args=0, max_args=0]
    fn rev(self) -> Rev<Self> {}                            // reverse iterator [min_args=0, max_args=0]
    fn last(self) -> Option<Self::Item> {}                  // get last element [min_args=0, max_args=0]
    fn nth(&mut self, n: usize) -> Option<Self::Item> {}    // get nth element [min_args=1, max_args=1]
}
