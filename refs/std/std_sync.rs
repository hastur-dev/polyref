// std::sync Reference — Rust standard library synchronization primitives
// Version: std
// Lang: rust

use std::sync;

// ============================================================================
// Mutex
// ============================================================================

impl Mutex {
    fn new(t: T) -> Mutex<T> {}                             // create new Mutex [min_args=1, max_args=1]
    fn lock(&self) -> LockResult<MutexGuard<T>> {}          // acquire lock [min_args=0, max_args=0]
    fn try_lock(&self) -> TryLockResult<MutexGuard<T>> {}   // try to acquire lock [min_args=0, max_args=0]
    fn is_poisoned(&self) -> bool {}                        // check if poisoned [min_args=0, max_args=0]
}

// ============================================================================
// RwLock
// ============================================================================

impl RwLock {
    fn new(t: T) -> RwLock<T> {}                            // create new RwLock [min_args=1, max_args=1]
    fn read(&self) -> LockResult<RwLockReadGuard<T>> {}     // acquire read lock [min_args=0, max_args=0]
    fn write(&self) -> LockResult<RwLockWriteGuard<T>> {}   // acquire write lock [min_args=0, max_args=0]
    fn try_read(&self) -> TryLockResult<RwLockReadGuard<T>> {} // try read lock [min_args=0, max_args=0]
    fn try_write(&self) -> TryLockResult<RwLockWriteGuard<T>> {} // try write lock [min_args=0, max_args=0]
}

// ============================================================================
// Arc
// ============================================================================

impl Arc {
    fn new(data: T) -> Arc<T> {}                            // create new Arc [min_args=1, max_args=1]
    fn clone(&self) -> Arc<T> {}                            // clone Arc (increment refcount) [min_args=0, max_args=0]
    fn strong_count(this: &Arc<T>) -> usize {}              // get strong reference count [min_args=1, max_args=1]
    fn weak_count(this: &Arc<T>) -> usize {}                // get weak reference count [min_args=1, max_args=1]
    fn try_unwrap(this: Arc<T>) -> Result<T, Arc<T>> {}     // try to unwrap [min_args=1, max_args=1]
}

// ============================================================================
// mpsc channels
// ============================================================================

fn channel() -> (Sender<T>, Receiver<T>) {}                 // create unbounded channel [min_args=0, max_args=0]
fn sync_channel(bound: usize) -> (SyncSender<T>, Receiver<T>) {} // create bounded channel [min_args=1, max_args=1]

impl Sender {
    fn send(&self, t: T) -> Result<(), SendError<T>> {}     // send value [min_args=1, max_args=1]
    fn clone(&self) -> Sender<T> {}                         // clone sender [min_args=0, max_args=0]
}

impl Receiver {
    fn recv(&self) -> Result<T, RecvError> {}                // receive blocking [min_args=0, max_args=0]
    fn try_recv(&self) -> Result<T, TryRecvError> {}         // try receive non-blocking [min_args=0, max_args=0]
    fn recv_timeout(&self, timeout: Duration) -> Result<T, RecvTimeoutError> {} // receive with timeout [min_args=1, max_args=1]
}
