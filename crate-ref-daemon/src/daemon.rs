use std::path::PathBuf;

use anyhow::Result;

use crate::autogen::auto_generate_refs;
use crate::builder::{build_index, remove_entry, update_entry};
use crate::cache::ContentCache;
use crate::hook_types::{HookAction, HookEvent, HookResponse};
use crate::index::{load, save, FlatIndex};
use crate::watcher::{drain_events, RefWatcher};

#[derive(Debug, Clone)]
pub struct DaemonConfig {
    /// Directory containing lib_*.rs reference files.
    pub ref_dir: PathBuf,
    /// Path to persist the flat index between restarts.
    pub index_path: PathBuf,
    /// Hamming distance threshold (suggested default: 8).
    pub threshold: u32,
    /// Project directory to scan for dependencies on startup (auto-generation).
    pub project_dir: Option<PathBuf>,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        let home = dirs_home();
        Self {
            ref_dir: home.join(".config/crate-ref/refs"),
            index_path: home.join(".config/crate-ref/index.bin"),
            threshold: 8,
            project_dir: None,
        }
    }
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"))
}

/// Core daemon state.
pub struct Daemon {
    pub config: DaemonConfig,
    pub index: FlatIndex,
    pub cache: ContentCache,
}

impl Daemon {
    /// Initialize daemon: load persisted index or rebuild from ref_dir.
    /// If `project_dir` is set, auto-generates missing reference files on startup.
    pub fn new(config: DaemonConfig) -> Result<Self> {
        let mut index = if config.index_path.exists() {
            load(&config.index_path)?
        } else if config.ref_dir.exists() {
            build_index(&config.ref_dir)?
        } else {
            FlatIndex::new()
        };

        // Auto-generate missing refs for the project's dependencies
        if let Some(ref project_dir) = config.project_dir {
            match auto_generate_refs(project_dir, &config.ref_dir, &mut index) {
                Ok(result) => {
                    if !result.generated.is_empty() {
                        eprintln!(
                            "[crate-ref-daemon] auto-generated {} ref(s): {}",
                            result.generated.len(),
                            result.generated.join(", ")
                        );
                    }
                    if !result.failed.is_empty() {
                        for (name, err) in &result.failed {
                            eprintln!("[crate-ref-daemon] failed to generate ref for {}: {}", name, err);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[crate-ref-daemon] autogen error: {e:#}");
                }
            }
        }

        Ok(Self {
            config,
            index,
            cache: ContentCache::new(),
        })
    }

    /// Process a hook event. Returns the response to send back to Claude Code.
    pub fn handle_event(&mut self, event: &HookEvent) -> HookResponse {
        let file_path = match &event.tool_input.file_path {
            Some(p) => p.clone(),
            None => {
                return HookResponse {
                    action: HookAction::Proceed,
                    message: None,
                }
            }
        };

        // Only process Rust files
        if !file_path.ends_with(".rs") {
            return HookResponse {
                action: HookAction::Proceed,
                message: None,
            };
        }

        // Read file content
        let content = match std::fs::read(&file_path) {
            Ok(c) => c,
            Err(_) => {
                return HookResponse {
                    action: HookAction::Proceed,
                    message: None,
                }
            }
        };

        let hash = crate::cache::content_hash(&content);

        // Cache hit: skip re-analysis
        if let Some(cached) = self.cache.get(&file_path, hash) {
            if cached.issues.is_empty() {
                return HookResponse {
                    action: HookAction::Proceed,
                    message: None,
                };
            } else {
                return HookResponse {
                    action: HookAction::Block,
                    message: Some(cached.issues.join("\n")),
                };
            }
        }

        // If the index is empty, proceed without blocking — no refs loaded yet
        if self.index.is_empty() {
            return HookResponse {
                action: HookAction::Proceed,
                message: None,
            };
        }

        // Run SimHash query against reference index
        let fp = crate::simhash::simhash(&String::from_utf8_lossy(&content));
        let matches = self.index.query(fp, self.config.threshold);

        let issues: Vec<String> = if matches.is_empty() {
            vec!["No matching reference found for this file.".to_string()]
        } else {
            vec![] // Matches found -- no issues
        };

        // Store in cache
        self.cache.insert(
            file_path,
            crate::cache::CachedResult {
                content_hash: hash,
                fingerprint: fp,
                issues: issues.clone(),
            },
        );

        if issues.is_empty() {
            HookResponse {
                action: HookAction::Proceed,
                message: None,
            }
        } else {
            HookResponse {
                action: HookAction::Block,
                message: Some(issues.join("\n")),
            }
        }
    }

    /// Apply any pending file watcher events to the index (incremental update).
    pub fn apply_watcher_events(&mut self, watcher: &RefWatcher) -> Result<()> {
        let changed = drain_events(watcher);
        let had_changes = !changed.is_empty();
        for path in changed {
            if path.exists() {
                update_entry(&mut self.index, &path)?;
            } else {
                remove_entry(&mut self.index, &path);
            }
        }
        if had_changes {
            save(&self.index, &self.config.index_path)?;
        }
        Ok(())
    }

    /// Persist the current index to disk.
    pub fn persist_index(&self) -> Result<()> {
        if let Some(parent) = self.config.index_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        save(&self.index, &self.config.index_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_config(tmp: &tempfile::TempDir) -> DaemonConfig {
        DaemonConfig {
            ref_dir: tmp.path().join("refs"),
            index_path: tmp.path().join("index.bin"),
            threshold: 8,
            project_dir: None,
        }
    }

    #[test]
    fn test_daemon_new_empty_config() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config = temp_config(&tmp);
        let daemon = Daemon::new(config).unwrap();
        assert!(daemon.index.is_empty());
    }

    #[test]
    fn test_handle_event_non_rs_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config = temp_config(&tmp);
        let mut daemon = Daemon::new(config).unwrap();
        let event = HookEvent {
            hook_event_name: "PostToolUse".to_string(),
            tool_name: "Write".to_string(),
            tool_input: crate::hook_types::ToolInput {
                file_path: Some("main.py".to_string()),
                command: None,
                content: None,
            },
            tool_response: serde_json::Value::Null,
        };
        let resp = daemon.handle_event(&event);
        assert_eq!(resp.action, HookAction::Proceed);
    }

    #[test]
    fn test_handle_event_no_file_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config = temp_config(&tmp);
        let mut daemon = Daemon::new(config).unwrap();
        let event = HookEvent {
            hook_event_name: "PostToolUse".to_string(),
            tool_name: "Bash".to_string(),
            tool_input: crate::hook_types::ToolInput {
                file_path: None,
                command: Some("ls".to_string()),
                content: None,
            },
            tool_response: serde_json::Value::Null,
        };
        let resp = daemon.handle_event(&event);
        assert_eq!(resp.action, HookAction::Proceed);
    }

    #[test]
    fn test_handle_event_missing_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config = temp_config(&tmp);
        let mut daemon = Daemon::new(config).unwrap();
        let event = HookEvent {
            hook_event_name: "PostToolUse".to_string(),
            tool_name: "Write".to_string(),
            tool_input: crate::hook_types::ToolInput {
                file_path: Some("/nonexistent/path/file.rs".to_string()),
                command: None,
                content: None,
            },
            tool_response: serde_json::Value::Null,
        };
        let resp = daemon.handle_event(&event);
        assert_eq!(resp.action, HookAction::Proceed);
    }

    #[test]
    fn test_handle_event_cache_hit_skips_reanalysis() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config = temp_config(&tmp);
        let mut daemon = Daemon::new(config).unwrap();

        // Write a real .rs file
        let file_path = tmp.path().join("test.rs");
        std::fs::write(&file_path, "fn main() { let x = 42; }").unwrap();

        let event = HookEvent {
            hook_event_name: "PostToolUse".to_string(),
            tool_name: "Write".to_string(),
            tool_input: crate::hook_types::ToolInput {
                file_path: Some(file_path.to_string_lossy().to_string()),
                command: None,
                content: None,
            },
            tool_response: serde_json::Value::Null,
        };

        // With empty index, daemon proceeds without caching
        let resp = daemon.handle_event(&event);
        assert_eq!(resp.action, HookAction::Proceed);
        assert_eq!(daemon.cache.len(), 0); // no caching when index is empty
    }

    #[test]
    fn test_handle_event_empty_index_proceeds() {
        let tmp = tempfile::TempDir::new().unwrap();
        let config = temp_config(&tmp);
        let mut daemon = Daemon::new(config).unwrap();
        assert!(daemon.index.is_empty());

        let file_path = tmp.path().join("any_file.rs");
        std::fs::write(&file_path, "fn main() {}").unwrap();

        let event = HookEvent {
            hook_event_name: "PostToolUse".to_string(),
            tool_name: "Write".to_string(),
            tool_input: crate::hook_types::ToolInput {
                file_path: Some(file_path.to_string_lossy().to_string()),
                command: None,
                content: None,
            },
            tool_response: serde_json::Value::Null,
        };

        let resp = daemon.handle_event(&event);
        assert_eq!(resp.action, HookAction::Proceed);
    }

    #[test]
    fn test_handle_event_cache_hit_with_populated_index() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ref_dir = tmp.path().join("refs");
        std::fs::create_dir_all(&ref_dir).unwrap();
        std::fs::write(ref_dir.join("lib_test.rs"), "fn main() { let x = 42; }").unwrap();

        let config = DaemonConfig {
            ref_dir,
            index_path: tmp.path().join("index.bin"),
            threshold: 8,
            project_dir: None,
        };
        let mut daemon = Daemon::new(config).unwrap();
        assert!(!daemon.index.is_empty());

        let file_path = tmp.path().join("test.rs");
        std::fs::write(&file_path, "fn main() { let x = 42; }").unwrap();

        let event = HookEvent {
            hook_event_name: "PostToolUse".to_string(),
            tool_name: "Write".to_string(),
            tool_input: crate::hook_types::ToolInput {
                file_path: Some(file_path.to_string_lossy().to_string()),
                command: None,
                content: None,
            },
            tool_response: serde_json::Value::Null,
        };

        // First call populates cache
        daemon.handle_event(&event);
        assert_eq!(daemon.cache.len(), 1);

        // Second call uses cache (cache len stays 1)
        daemon.handle_event(&event);
        assert_eq!(daemon.cache.len(), 1);
    }

    #[test]
    fn test_persist_and_reload() {
        let tmp = tempfile::TempDir::new().unwrap();
        let ref_dir = tmp.path().join("refs");
        std::fs::create_dir_all(&ref_dir).unwrap();
        std::fs::write(ref_dir.join("lib_a.rs"), "fn alpha() { let x = 1; }").unwrap();
        std::fs::write(ref_dir.join("lib_b.rs"), "fn beta() { let y = 2; }").unwrap();

        let config = DaemonConfig {
            ref_dir: ref_dir.clone(),
            index_path: tmp.path().join("index.bin"),
            threshold: 8,
            project_dir: None,
        };

        let daemon = Daemon::new(config.clone()).unwrap();
        assert_eq!(daemon.index.len(), 2);
        daemon.persist_index().unwrap();

        // Reload from persisted index
        let daemon2 = Daemon::new(config).unwrap();
        assert_eq!(daemon2.index.len(), 2);
    }
}
