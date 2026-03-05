use crate::detect::Language;
use std::path::PathBuf;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ModelConfig {
    /// Strip markdown code fences from input (for raw model output)
    #[serde(default)]
    pub strip_fences: Option<bool>,
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self { strip_fences: None }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Config {
    /// Project root directory
    #[serde(default = "default_project_root")]
    pub project_root: PathBuf,
    /// Where to store generated reference files
    #[serde(default = "default_refs_dir")]
    pub refs_dir: PathBuf,
    /// Which languages to process (None = auto-detect)
    #[serde(default)]
    pub languages: Option<Vec<Language>>,
    /// Libraries to skip
    #[serde(default)]
    pub skip_libraries: Vec<String>,
    /// Output format
    #[serde(default)]
    pub output_format: OutputFormat,
    /// Whether to use cached reference files
    #[serde(default = "default_use_cache")]
    pub use_cache: bool,
    /// Maximum age of cached files in hours
    #[serde(default = "default_cache_max_age_hours")]
    pub cache_max_age_hours: u64,
    /// Optional global directory of flat reference files (e.g. coding/references/)
    #[serde(default)]
    pub global_refs_dir: Option<PathBuf>,
    /// Model input/output configuration
    #[serde(default)]
    pub model: ModelConfig,
}

fn default_project_root() -> PathBuf {
    PathBuf::from(".")
}

fn default_refs_dir() -> PathBuf {
    PathBuf::from("refs")
}

fn default_use_cache() -> bool {
    true
}

fn default_cache_max_age_hours() -> u64 {
    168
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[derive(Default)]
pub enum OutputFormat {
    #[default]
    Terminal,
    Json,
    Both,
}


impl Default for Config {
    fn default() -> Self {
        Self {
            project_root: PathBuf::from("."),
            refs_dir: PathBuf::from("refs"),
            languages: None,
            skip_libraries: vec![],
            output_format: OutputFormat::Terminal,
            use_cache: true,
            cache_max_age_hours: 168,
            global_refs_dir: None,
            model: ModelConfig::default(),
        }
    }
}

impl Config {
    /// Load from polyref.toml if present, otherwise use defaults
    pub fn load(project_root: &std::path::Path) -> anyhow::Result<Self> {
        let config_path = project_root.join("polyref.toml");
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let mut config: Config = toml::from_str(&content)?;
            config.project_root = project_root.to_path_buf();
            Ok(config)
        } else {
            Ok(Config {
                project_root: project_root.to_path_buf(),
                ..Config::default()
            })
        }
    }

    /// Resolve refs_dir relative to project_root
    pub fn resolved_refs_dir(&self) -> PathBuf {
        if self.refs_dir.is_relative() {
            self.project_root.join(&self.refs_dir)
        } else {
            self.refs_dir.clone()
        }
    }

    /// Resolve global_refs_dir. Resolution order:
    /// 1. Explicit `global_refs_dir` in polyref.toml (relative paths resolve against project_root)
    /// 2. System data directory (e.g. `~/.local/share/polyref/refs` on Linux)
    pub fn resolved_global_refs_dir(&self) -> Option<PathBuf> {
        if let Some(dir) = &self.global_refs_dir {
            let resolved = if dir.is_relative() {
                self.project_root.join(dir)
            } else {
                dir.clone()
            };
            return Some(resolved);
        }

        // Fall back to system data directory
        crate::dirs::global_refs_dir()
    }
}
