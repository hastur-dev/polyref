use crate::detect::{Dependency, Language};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CacheEntry {
    pub library: String,
    pub version: String,
    pub language: Language,
    pub hash: String,
    pub generated_at: DateTime<Utc>,
    pub file_path: PathBuf,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Cache {
    entries: HashMap<String, CacheEntry>,
}

impl Cache {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    fn cache_key(dep: &Dependency) -> String {
        format!("{}:{}:{}", dep.language, dep.name, dep.version)
    }

    pub fn load(cache_dir: &Path) -> anyhow::Result<Self> {
        let cache_file = cache_dir.join("polyref_cache.json");
        if !cache_file.exists() {
            return Ok(Self::new());
        }
        let content = std::fs::read_to_string(&cache_file)?;
        let cache: Cache = serde_json::from_str(&content)?;
        Ok(cache)
    }

    pub fn save(&self, cache_dir: &Path) -> anyhow::Result<()> {
        std::fs::create_dir_all(cache_dir)?;
        let cache_file = cache_dir.join("polyref_cache.json");
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&cache_file, content)?;
        Ok(())
    }

    pub fn is_valid(&self, dep: &Dependency, max_age_hours: u64) -> bool {
        let key = Self::cache_key(dep);
        match self.entries.get(&key) {
            Some(entry) => {
                if entry.version != dep.version {
                    return false;
                }
                let age = Utc::now() - entry.generated_at;
                age.num_hours() < max_age_hours as i64
            }
            None => false,
        }
    }

    pub fn record(&mut self, dep: &Dependency, file_path: PathBuf) {
        let key = Self::cache_key(dep);
        self.entries.insert(
            key,
            CacheEntry {
                library: dep.name.clone(),
                version: dep.version.clone(),
                language: dep.language,
                hash: String::new(),
                generated_at: Utc::now(),
                file_path,
            },
        );
    }

    pub fn get(&self, dep: &Dependency) -> Option<&CacheEntry> {
        let key = Self::cache_key(dep);
        self.entries.get(&key)
    }
}

impl Default for Cache {
    fn default() -> Self {
        Self::new()
    }
}
