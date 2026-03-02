use super::{Dependency, Language};
use std::path::Path;

/// Detect Python dependencies from requirements.txt, pyproject.toml, or Pipfile
pub fn detect_python(project_root: &Path) -> anyhow::Result<Vec<Dependency>> {
    let mut deps = Vec::new();
    let mut seen_names = std::collections::HashSet::new();

    // Priority 1: pyproject.toml
    let pyproject_path = project_root.join("pyproject.toml");
    if pyproject_path.exists() {
        let pyproject_deps = parse_pyproject_toml(&pyproject_path)?;
        for dep in pyproject_deps {
            if seen_names.insert(dep.name.clone()) {
                deps.push(dep);
            }
        }
    }

    // Priority 2: requirements.txt
    let req_path = project_root.join("requirements.txt");
    if req_path.exists() {
        let req_deps = parse_requirements_txt(&req_path)?;
        for dep in req_deps {
            if seen_names.insert(dep.name.clone()) {
                deps.push(dep);
            }
        }
    }

    // Priority 3: Pipfile
    let pipfile_path = project_root.join("Pipfile");
    if pipfile_path.exists() {
        let pipfile_deps = parse_pipfile(&pipfile_path)?;
        for dep in pipfile_deps {
            if seen_names.insert(dep.name.clone()) {
                deps.push(dep);
            }
        }
    }

    Ok(deps)
}

fn parse_requirements_txt(path: &Path) -> anyhow::Result<Vec<Dependency>> {
    let content = std::fs::read_to_string(path)?;
    let mut deps = Vec::new();

    for line in content.lines() {
        let line = line.trim();

        // Skip empty lines, comments, options, and includes
        if line.is_empty()
            || line.starts_with('#')
            || line.starts_with('-')
            || line.starts_with("--")
        {
            continue;
        }

        if let Some(dep) = parse_requirement_line(line, "requirements.txt") {
            deps.push(dep);
        }
    }

    Ok(deps)
}

fn parse_requirement_line(line: &str, source_file: &str) -> Option<Dependency> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    // Strip extras: requests[security]>=2.0 -> requests>=2.0
    let line = if let Some(bracket_start) = line.find('[') {
        if let Some(bracket_end) = line.find(']') {
            format!("{}{}", &line[..bracket_start], &line[bracket_end + 1..])
        } else {
            line.to_string()
        }
    } else {
        line.to_string()
    };

    // Split on version specifiers: ==, >=, <=, ~=, !=, >, <
    let separators = ["==", ">=", "<=", "~=", "!=", ">", "<"];
    for sep in &separators {
        if let Some(idx) = line.find(sep) {
            let name = line[..idx].trim().to_string();
            let version = line[idx..].trim().to_string();
            if !name.is_empty() {
                return Some(Dependency {
                    name,
                    version,
                    language: Language::Python,
                    source_file: source_file.to_string(),
                });
            }
        }
    }

    // No version specifier — bare package name
    let name = line.trim().to_string();
    if !name.is_empty() {
        return Some(Dependency {
            name,
            version: "*".to_string(),
            language: Language::Python,
            source_file: source_file.to_string(),
        });
    }

    None
}

fn parse_pyproject_toml(path: &Path) -> anyhow::Result<Vec<Dependency>> {
    let content = std::fs::read_to_string(path)?;
    let doc: toml::Value = content.parse().map_err(|e: toml::de::Error| {
        anyhow::anyhow!("Failed to parse pyproject.toml: {}", e)
    })?;

    let mut deps = Vec::new();

    // [project.dependencies] — PEP 621 format (array of strings)
    if let Some(project_deps) = doc
        .get("project")
        .and_then(|p| p.get("dependencies"))
        .and_then(|d| d.as_array())
    {
        for dep_str in project_deps {
            if let Some(s) = dep_str.as_str() {
                if let Some(dep) = parse_requirement_line(s, "pyproject.toml") {
                    deps.push(dep);
                }
            }
        }
    }

    // [tool.poetry.dependencies] — Poetry format (table)
    if let Some(poetry_deps) = doc
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("dependencies"))
        .and_then(|d| d.as_table())
    {
        for (name, value) in poetry_deps {
            if name == "python" {
                continue; // Skip the python version constraint
            }
            let version = match value {
                toml::Value::String(v) => v.clone(),
                toml::Value::Table(t) => t
                    .get("version")
                    .and_then(|v| v.as_str())
                    .unwrap_or("*")
                    .to_string(),
                _ => "*".to_string(),
            };
            deps.push(Dependency {
                name: name.clone(),
                version,
                language: Language::Python,
                source_file: "pyproject.toml".to_string(),
            });
        }
    }

    Ok(deps)
}

fn parse_pipfile(path: &Path) -> anyhow::Result<Vec<Dependency>> {
    let content = std::fs::read_to_string(path)?;
    let doc: toml::Value = content.parse().map_err(|e: toml::de::Error| {
        anyhow::anyhow!("Failed to parse Pipfile: {}", e)
    })?;

    let mut deps = Vec::new();

    for section in &["packages", "dev-packages"] {
        if let Some(table) = doc.get(section).and_then(|v| v.as_table()) {
            for (name, value) in table {
                let version = match value {
                    toml::Value::String(v) => {
                        if v == "*" {
                            "*".to_string()
                        } else {
                            v.clone()
                        }
                    }
                    toml::Value::Table(t) => t
                        .get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("*")
                        .to_string(),
                    _ => "*".to_string(),
                };
                deps.push(Dependency {
                    name: name.clone(),
                    version,
                    language: Language::Python,
                    source_file: "Pipfile".to_string(),
                });
            }
        }
    }

    Ok(deps)
}
