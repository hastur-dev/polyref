// std::io Reference — Rust standard library I/O
// Version: std
// Lang: rust

use std::io;

// ============================================================================
// Read trait
// ============================================================================

impl Read {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {} // read bytes into buffer [min_args=1, max_args=1]
    fn read_to_string(&mut self, buf: &mut String) -> io::Result<usize> {} // read all to string [min_args=1, max_args=1]
    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {} // read all to vec [min_args=1, max_args=1]
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {} // read exact bytes [min_args=1, max_args=1]
}

// ============================================================================
// Write trait
// ============================================================================

impl Write {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {} // write bytes [min_args=1, max_args=1]
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {} // write all bytes [min_args=1, max_args=1]
    fn flush(&mut self) -> io::Result<()> {}                // flush output [min_args=0, max_args=0]
}

// ============================================================================
// BufRead trait
// ============================================================================

impl BufRead {
    fn read_line(&mut self, buf: &mut String) -> io::Result<usize> {} // read line [min_args=1, max_args=1]
    fn lines(&self) -> Lines<Self> {}                       // iterate over lines [min_args=0, max_args=0]
}

// ============================================================================
// BufReader / BufWriter
// ============================================================================

impl BufReader {
    fn new(inner: R) -> BufReader<R> {}                     // wrap reader in buffer [min_args=1, max_args=1]
    fn with_capacity(capacity: usize, inner: R) -> BufReader<R> {} // custom buffer size [min_args=2, max_args=2]
}

impl BufWriter {
    fn new(inner: W) -> BufWriter<W> {}                     // wrap writer in buffer [min_args=1, max_args=1]
    fn with_capacity(capacity: usize, inner: W) -> BufWriter<W> {} // custom buffer size [min_args=2, max_args=2]
}

// ============================================================================
// Free functions
// ============================================================================

fn stdin() -> Stdin {}                                      // standard input handle [min_args=0, max_args=0]
fn stdout() -> Stdout {}                                    // standard output handle [min_args=0, max_args=0]
fn stderr() -> Stderr {}                                    // standard error handle [min_args=0, max_args=0]
