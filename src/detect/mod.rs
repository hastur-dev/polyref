pub mod python;
pub mod rust;
pub mod typescript;

use std::path::{Path, PathBuf};

/// Supported languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Language {
    Rust,
    Python,
    TypeScript,
}

impl std::fmt::Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Language::Rust => write!(f, "Rust"),
            Language::Python => write!(f, "Python"),
            Language::TypeScript => write!(f, "TypeScript"),
        }
    }
}

/// A detected dependency
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Dependency {
    pub name: String,
    pub version: String,
    pub language: Language,
    /// Where the dependency was declared (e.g., "Cargo.toml", "requirements.txt")
    pub source_file: String,
}

/// Result of scanning a project directory
#[derive(Debug, Clone, serde::Serialize)]
pub struct DetectedProject {
    pub root: PathBuf,
    pub languages: Vec<Language>,
    pub dependencies: Vec<Dependency>,
    pub manifest_files: Vec<PathBuf>,
}

/// Detect languages and dependencies in a project directory
pub fn detect(project_root: &Path) -> anyhow::Result<DetectedProject> {
    detect_with_skip(project_root, &[])
}

/// Detect languages and dependencies, skipping named libraries
pub fn detect_with_skip(project_root: &Path, skip_libraries: &[String]) -> anyhow::Result<DetectedProject> {
    let mut languages = Vec::new();
    let mut dependencies = Vec::new();
    let mut manifest_files = Vec::new();

    // Rust detection
    if project_root.join("Cargo.toml").exists() {
        let rust_deps = rust::detect_rust(project_root)?;
        if !rust_deps.is_empty() {
            if !languages.contains(&Language::Rust) {
                languages.push(Language::Rust);
            }
            manifest_files.push(project_root.join("Cargo.toml"));
            dependencies.extend(rust_deps);
        }
    }

    // Python detection
    let py_manifests = ["pyproject.toml", "requirements.txt", "Pipfile"];
    let has_python = py_manifests.iter().any(|f| project_root.join(f).exists());
    if has_python {
        let python_deps = python::detect_python(project_root)?;
        if !python_deps.is_empty() || has_python {
            if !languages.contains(&Language::Python) {
                languages.push(Language::Python);
            }
            for f in &py_manifests {
                let p = project_root.join(f);
                if p.exists() && !manifest_files.contains(&p) {
                    manifest_files.push(p);
                }
            }
            dependencies.extend(python_deps);
        }
    }

    // TypeScript detection
    if project_root.join("package.json").exists() {
        let ts_deps = typescript::detect_typescript(project_root)?;
        if !ts_deps.is_empty() || project_root.join("package.json").exists() {
            if !languages.contains(&Language::TypeScript) {
                languages.push(Language::TypeScript);
            }
            manifest_files.push(project_root.join("package.json"));
            if project_root.join("tsconfig.json").exists() {
                manifest_files.push(project_root.join("tsconfig.json"));
            }
            dependencies.extend(ts_deps);
        }
    }

    // Deduplicate: same name + language → keep first
    let mut seen = std::collections::HashSet::new();
    dependencies.retain(|dep| seen.insert((dep.name.clone(), dep.language)));

    // Apply skip list
    if !skip_libraries.is_empty() {
        dependencies.retain(|dep| !skip_libraries.contains(&dep.name));
    }

    Ok(DetectedProject {
        root: project_root.to_path_buf(),
        languages,
        dependencies,
        manifest_files,
    })
}
