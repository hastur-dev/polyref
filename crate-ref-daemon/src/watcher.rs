use anyhow::Result;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

pub struct RefWatcher {
    _debouncer: notify_debouncer_mini::Debouncer<notify::RecommendedWatcher>,
    pub rx: Receiver<DebounceEventResult>,
}

impl RefWatcher {
    /// Watch `dir` non-recursively for changes to `*.rs` files.
    pub fn watch(dir: &Path) -> Result<Self> {
        let (tx, rx) = channel();
        let mut debouncer = new_debouncer(Duration::from_millis(200), tx)?;
        debouncer
            .watcher()
            .watch(dir, notify::RecursiveMode::NonRecursive)?;
        Ok(Self {
            _debouncer: debouncer,
            rx,
        })
    }
}

/// Drain any pending debounced events, returning unique changed `.rs` file paths.
/// Returns immediately if no events are pending.
pub fn drain_events(watcher: &RefWatcher) -> Vec<PathBuf> {
    let mut paths = std::collections::HashSet::new();
    while let Ok(result) = watcher.rx.try_recv() {
        if let Ok(events) = result {
            for event in events {
                let p = &event.path;
                if p.extension().map(|e| e == "rs").unwrap_or(false) {
                    paths.insert(p.clone());
                }
            }
        }
    }
    paths.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watcher_creates_without_error() {
        let tmp = tempfile::TempDir::new().unwrap();
        let watcher = RefWatcher::watch(tmp.path());
        assert!(watcher.is_ok());
    }

    #[test]
    fn test_drain_empty_initially() {
        let tmp = tempfile::TempDir::new().unwrap();
        let watcher = RefWatcher::watch(tmp.path()).unwrap();
        let events = drain_events(&watcher);
        assert!(events.is_empty());
    }

    #[test]
    fn test_drain_detects_rs_file_write() {
        let tmp = tempfile::TempDir::new().unwrap();
        let watcher = RefWatcher::watch(tmp.path()).unwrap();
        let file_path = tmp.path().join("test.rs");
        std::fs::write(&file_path, "fn main() {}").unwrap();
        std::thread::sleep(Duration::from_millis(400));
        let events = drain_events(&watcher);
        assert!(
            events.iter().any(|p| p.ends_with("test.rs")),
            "expected test.rs in events: {:?}",
            events
        );
    }

    #[test]
    fn test_drain_ignores_non_rs_files() {
        let tmp = tempfile::TempDir::new().unwrap();
        let watcher = RefWatcher::watch(tmp.path()).unwrap();
        std::fs::write(tmp.path().join("foo.txt"), "hello").unwrap();
        std::thread::sleep(Duration::from_millis(400));
        let events = drain_events(&watcher);
        assert!(events.is_empty());
    }
}
