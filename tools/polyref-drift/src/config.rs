use serde::{Deserialize, Serialize};
use std::path::Path;

/// Configuration for drift detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftConfig {
    /// Directories containing reference files to check
    #[serde(default = "default_refs_dirs")]
    pub refs_dirs: Vec<String>,

    /// Registries to check against
    #[serde(default = "default_registries")]
    pub registries: RegistryConfig,

    /// Maximum age in days before a ref file is considered stale
    #[serde(default = "default_max_age_days")]
    pub max_age_days: u32,

    /// HTTP proxy (e.g. for corporate environments)
    #[serde(default)]
    pub http_proxy: Option<String>,

    /// Libraries to skip drift checking
    #[serde(default)]
    pub skip: Vec<String>,

    /// Output format: terminal or json
    #[serde(default = "default_output")]
    pub output: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    #[serde(default = "default_true")]
    pub crates_io: bool,
    #[serde(default = "default_true")]
    pub pypi: bool,
    #[serde(default = "default_true")]
    pub npm: bool,
}

fn default_refs_dirs() -> Vec<String> {
    vec!["refs".to_string()]
}

fn default_registries() -> RegistryConfig {
    RegistryConfig {
        crates_io: true,
        pypi: true,
        npm: true,
    }
}

fn default_max_age_days() -> u32 {
    30
}

fn default_output() -> String {
    "terminal".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for DriftConfig {
    fn default() -> Self {
        Self {
            refs_dirs: default_refs_dirs(),
            registries: default_registries(),
            max_age_days: default_max_age_days(),
            http_proxy: None,
            skip: Vec::new(),
            output: default_output(),
        }
    }
}

impl DriftConfig {
    /// Load configuration from a TOML file.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: DriftConfig = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Load from path, falling back to defaults.
    pub fn load_or_default(path: &Path) -> Self {
        Self::load(path).unwrap_or_default()
    }

    /// Validate configuration values.
    pub fn validate(&self) -> anyhow::Result<()> {
        if self.refs_dirs.is_empty() {
            anyhow::bail!("refs_dirs must not be empty");
        }
        if self.max_age_days == 0 {
            anyhow::bail!("max_age_days must be > 0");
        }
        if !["terminal", "json"].contains(&self.output.as_str()) {
            anyhow::bail!("output must be 'terminal' or 'json'");
        }
        Ok(())
    }

    /// Check if a library should be skipped.
    pub fn should_skip(&self, lib_name: &str) -> bool {
        self.skip.iter().any(|s| s == lib_name)
    }
}
