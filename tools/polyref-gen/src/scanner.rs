use std::path::{Path, PathBuf};

use crate::python_gen;
use crate::python_source_gen;
use crate::rust_source_gen;
use crate::rustdoc_gen;

#[derive(Debug, Clone, PartialEq)]
pub enum ProjectKind {
    Rust,
    Python,
}

#[derive(Debug, Clone)]
pub struct DiscoveredProject {
    pub name: String,
    pub path: PathBuf,
    pub kind: ProjectKind,
}

#[derive(Debug)]
pub struct GenerationResult {
    pub project: DiscoveredProject,
    pub output_path: PathBuf,
    pub success: bool,
    pub error: Option<String>,
}

/// Scan a directory tree for Rust and Python projects.
pub fn discover_projects(root: &Path) -> anyhow::Result<Vec<DiscoveredProject>> {
    let mut projects = Vec::new();

    if !root.is_dir() {
        anyhow::bail!("{} is not a directory", root.display());
    }

    let mut entries: Vec<_> = std::fs::read_dir(root)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    entries.sort();

    for entry in entries {
        if !entry.is_dir() {
            continue;
        }

        let dir_name = entry
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        // Skip hidden directories and common non-project dirs
        if dir_name.starts_with('.')
            || matches!(
                dir_name,
                "target" | "node_modules" | "__pycache__" | ".git" | "venv" | ".venv"
            )
        {
            continue;
        }

        // Check for Rust project
        if entry.join("Cargo.toml").exists() {
            let name = extract_project_name_rust(&entry);
            projects.push(DiscoveredProject {
                name,
                path: entry.clone(),
                kind: ProjectKind::Rust,
            });
            continue;
        }

        // Check for Python project
        if entry.join("pyproject.toml").exists()
            || entry.join("setup.py").exists()
            || entry.join("setup.cfg").exists()
        {
            let name = extract_project_name_python(&entry);
            projects.push(DiscoveredProject {
                name,
                path: entry.clone(),
                kind: ProjectKind::Python,
            });
            continue;
        }

        // Check for Python project by presence of .py files
        if has_py_files(&entry) {
            let name = dir_name.to_string();
            projects.push(DiscoveredProject {
                name,
                path: entry.clone(),
                kind: ProjectKind::Python,
            });
            continue;
        }

        // Not a project itself — recurse one level deeper to find nested projects
        // (e.g., knowledge-base/backend/, rust-webdev-stuff/services/backend/)
        discover_nested_projects(&entry, &mut projects, 2)?;
    }

    Ok(projects)
}

fn discover_nested_projects(
    dir: &Path,
    projects: &mut Vec<DiscoveredProject>,
    max_depth: u32,
) -> anyhow::Result<()> {
    if max_depth == 0 {
        return Ok(());
    }

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    let mut paths: Vec<_> = entries.filter_map(|e| e.ok()).map(|e| e.path()).collect();
    paths.sort();

    for entry in paths {
        if !entry.is_dir() {
            continue;
        }

        let dir_name = entry.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if dir_name.starts_with('.')
            || matches!(
                dir_name,
                "target" | "node_modules" | "__pycache__" | "venv" | ".venv"
            )
        {
            continue;
        }

        if entry.join("Cargo.toml").exists() {
            let name = extract_project_name_rust(&entry);
            projects.push(DiscoveredProject {
                name,
                path: entry.clone(),
                kind: ProjectKind::Rust,
            });
        } else if entry.join("pyproject.toml").exists()
            || entry.join("setup.py").exists()
            || entry.join("setup.cfg").exists()
        {
            let name = extract_project_name_python(&entry);
            projects.push(DiscoveredProject {
                name,
                path: entry.clone(),
                kind: ProjectKind::Python,
            });
        } else if has_py_files(&entry) {
            let name = dir_name.to_string();
            projects.push(DiscoveredProject {
                name,
                path: entry.clone(),
                kind: ProjectKind::Python,
            });
        } else {
            // Keep recursing
            discover_nested_projects(&entry, projects, max_depth - 1)?;
        }
    }

    Ok(())
}

fn has_py_files(dir: &Path) -> bool {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "py") {
                return true;
            }
        }
    }
    false
}

fn extract_project_name_rust(dir: &Path) -> String {
    let cargo_toml = dir.join("Cargo.toml");
    if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("name") {
                let rest = rest.trim();
                if let Some(rest) = rest.strip_prefix('=') {
                    return rest.trim().trim_matches('"').to_string();
                }
            }
        }
    }
    dir.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string()
}

fn extract_project_name_python(dir: &Path) -> String {
    // Try pyproject.toml first
    let pyproject = dir.join("pyproject.toml");
    if let Ok(content) = std::fs::read_to_string(&pyproject) {
        let mut in_project = false;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed == "[project]" || trimmed == "[tool.poetry]" {
                in_project = true;
                continue;
            }
            if in_project {
                if trimmed.starts_with('[') {
                    in_project = false;
                    continue;
                }
                if let Some(rest) = trimmed.strip_prefix("name") {
                    let rest = rest.trim();
                    if let Some(rest) = rest.strip_prefix('=') {
                        return rest.trim().trim_matches('"').trim_matches('\'').to_string();
                    }
                }
            }
        }
    }

    dir.file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string()
}

/// Generate reference files for all discovered projects.
pub fn generate_all(
    projects: &[DiscoveredProject],
    output_dir: &Path,
) -> Vec<GenerationResult> {
    std::fs::create_dir_all(output_dir).ok();

    let rust_dir = output_dir.join("rust");
    let python_dir = output_dir.join("python");
    std::fs::create_dir_all(&rust_dir).ok();
    std::fs::create_dir_all(&python_dir).ok();

    let mut results = Vec::new();

    for project in projects {
        let result = match project.kind {
            ProjectKind::Rust => generate_rust_ref(project, &rust_dir),
            ProjectKind::Python => generate_python_ref(project, &python_dir),
        };
        results.push(result);
    }

    results
}

fn generate_rust_ref(project: &DiscoveredProject, output_dir: &Path) -> GenerationResult {
    let output_path = output_dir.join(format!("{}.rs", sanitize_name(&project.name)));

    match rust_source_gen::parse_rust_project(&project.path) {
        Ok(doc) => {
            let content = rustdoc_gen::generate_ref_file(&doc);
            match std::fs::write(&output_path, &content) {
                Ok(()) => GenerationResult {
                    project: project.clone(),
                    output_path,
                    success: true,
                    error: None,
                },
                Err(e) => GenerationResult {
                    project: project.clone(),
                    output_path,
                    success: false,
                    error: Some(format!("Write error: {}", e)),
                },
            }
        }
        Err(e) => GenerationResult {
            project: project.clone(),
            output_path,
            success: false,
            error: Some(format!("Parse error: {}", e)),
        },
    }
}

fn generate_python_ref(project: &DiscoveredProject, output_dir: &Path) -> GenerationResult {
    let output_path = output_dir.join(format!("{}.polyref", sanitize_name(&project.name)));

    match python_source_gen::parse_python_project(&project.path) {
        Ok(stub_output) => {
            let content = python_gen::generate_polyref_file(&stub_output);
            match std::fs::write(&output_path, &content) {
                Ok(()) => GenerationResult {
                    project: project.clone(),
                    output_path,
                    success: true,
                    error: None,
                },
                Err(e) => GenerationResult {
                    project: project.clone(),
                    output_path,
                    success: false,
                    error: Some(format!("Write error: {}", e)),
                },
            }
        }
        Err(e) => GenerationResult {
            project: project.clone(),
            output_path,
            success: false,
            error: Some(format!("Parse error: {}", e)),
        },
    }
}

fn sanitize_name(name: &str) -> String {
    name.replace(|c: char| !c.is_alphanumeric() && c != '_' && c != '-', "_")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("my-crate"), "my-crate");
        assert_eq!(sanitize_name("my crate"), "my_crate");
        assert_eq!(sanitize_name("my/crate"), "my_crate");
    }

    #[test]
    fn test_discover_projects_rust() {
        let tmp = tempfile::tempdir().unwrap();
        let proj = tmp.path().join("my-proj");
        fs::create_dir(&proj).unwrap();
        fs::write(
            proj.join("Cargo.toml"),
            "[package]\nname = \"my-proj\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        fs::create_dir(proj.join("src")).unwrap();
        fs::write(proj.join("src/main.rs"), "fn main() {}").unwrap();

        let projects = discover_projects(tmp.path()).unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "my-proj");
        assert_eq!(projects[0].kind, ProjectKind::Rust);
    }

    #[test]
    fn test_discover_projects_python() {
        let tmp = tempfile::tempdir().unwrap();
        let proj = tmp.path().join("my-pkg");
        fs::create_dir(&proj).unwrap();
        fs::write(
            proj.join("pyproject.toml"),
            "[project]\nname = \"my-pkg\"\nversion = \"2.0.0\"\n",
        )
        .unwrap();
        fs::write(proj.join("main.py"), "def hello(): pass\n").unwrap();

        let projects = discover_projects(tmp.path()).unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].name, "my-pkg");
        assert_eq!(projects[0].kind, ProjectKind::Python);
    }

    #[test]
    fn test_discover_projects_mixed() {
        let tmp = tempfile::tempdir().unwrap();

        // Rust project
        let rust_proj = tmp.path().join("rust-app");
        fs::create_dir(&rust_proj).unwrap();
        fs::write(
            rust_proj.join("Cargo.toml"),
            "[package]\nname = \"rust-app\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        fs::create_dir(rust_proj.join("src")).unwrap();
        fs::write(rust_proj.join("src/main.rs"), "fn main() {}").unwrap();

        // Python project
        let py_proj = tmp.path().join("py-app");
        fs::create_dir(&py_proj).unwrap();
        fs::write(py_proj.join("setup.py"), "from setuptools import setup\nsetup(name='py-app')\n").unwrap();
        fs::write(py_proj.join("app.py"), "def run(): pass\n").unwrap();

        let projects = discover_projects(tmp.path()).unwrap();
        assert_eq!(projects.len(), 2);

        let kinds: Vec<&ProjectKind> = projects.iter().map(|p| &p.kind).collect();
        assert!(kinds.contains(&&ProjectKind::Rust));
        assert!(kinds.contains(&&ProjectKind::Python));
    }

    #[test]
    fn test_generate_all_rust() {
        let tmp = tempfile::tempdir().unwrap();
        let proj = tmp.path().join("test-crate");
        fs::create_dir(&proj).unwrap();
        fs::write(
            proj.join("Cargo.toml"),
            "[package]\nname = \"test-crate\"\nversion = \"0.5.0\"\n",
        )
        .unwrap();
        fs::create_dir(proj.join("src")).unwrap();
        fs::write(
            proj.join("src/lib.rs"),
            "/// Does something.\npub fn do_thing(x: i32) -> i32 { x + 1 }\n",
        )
        .unwrap();

        let projects = vec![DiscoveredProject {
            name: "test-crate".to_string(),
            path: proj,
            kind: ProjectKind::Rust,
        }];

        let out_dir = tmp.path().join("refs");
        let results = generate_all(&projects, &out_dir);

        assert_eq!(results.len(), 1);
        assert!(results[0].success, "Error: {:?}", results[0].error);
        assert!(results[0].output_path.exists());

        let content = fs::read_to_string(&results[0].output_path).unwrap();
        assert!(content.contains("do_thing"));
        assert!(content.contains("0.5.0"));
    }

    #[test]
    fn test_generate_all_python() {
        let tmp = tempfile::tempdir().unwrap();
        let proj = tmp.path().join("my-pkg");
        fs::create_dir(&proj).unwrap();
        fs::write(
            proj.join("pyproject.toml"),
            "[project]\nname = \"my-pkg\"\nversion = \"1.0.0\"\n",
        )
        .unwrap();
        fs::write(
            proj.join("app.py"),
            "class MyApp:\n    def run(self, config: dict) -> None:\n        pass\n\ndef main():\n    pass\n",
        )
        .unwrap();

        let projects = vec![DiscoveredProject {
            name: "my-pkg".to_string(),
            path: proj,
            kind: ProjectKind::Python,
        }];

        let out_dir = tmp.path().join("refs");
        let results = generate_all(&projects, &out_dir);

        assert_eq!(results.len(), 1);
        assert!(results[0].success, "Error: {:?}", results[0].error);

        let content = fs::read_to_string(&results[0].output_path).unwrap();
        assert!(content.contains("@lang python"));
        assert!(content.contains("MyApp"));
        assert!(content.contains("run"));
        assert!(content.contains("main"));
    }

    #[test]
    fn test_skips_hidden_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let hidden = tmp.path().join(".hidden-proj");
        fs::create_dir(&hidden).unwrap();
        fs::write(
            hidden.join("Cargo.toml"),
            "[package]\nname = \"hidden\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        let projects = discover_projects(tmp.path()).unwrap();
        assert!(projects.is_empty());
    }

    #[test]
    fn test_python_by_py_files_only() {
        let tmp = tempfile::tempdir().unwrap();
        let proj = tmp.path().join("scripts");
        fs::create_dir(&proj).unwrap();
        fs::write(proj.join("run.py"), "def main(): pass\n").unwrap();
        fs::write(proj.join("utils.py"), "def helper(): pass\n").unwrap();

        let projects = discover_projects(tmp.path()).unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].kind, ProjectKind::Python);
    }
}
