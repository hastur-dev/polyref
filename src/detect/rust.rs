use super::{Dependency, Language};
use std::path::Path;

/// Detect Rust dependencies from Cargo.toml
pub fn detect_rust(project_root: &Path) -> anyhow::Result<Vec<Dependency>> {
    let cargo_path = project_root.join("Cargo.toml");
    if !cargo_path.exists() {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(&cargo_path)?;
    let doc: toml::Value = content.parse().map_err(|e: toml::de::Error| {
        anyhow::anyhow!("Failed to parse Cargo.toml: {}", e)
    })?;

    let mut deps = Vec::new();

    // Parse [dependencies], [dev-dependencies], [build-dependencies]
    for section in &["dependencies", "dev-dependencies", "build-dependencies"] {
        if let Some(table) = doc.get(section).and_then(|v| v.as_table()) {
            for (name, value) in table {
                let version = extract_version(value);
                deps.push(Dependency {
                    name: name.clone(),
                    version,
                    language: Language::Rust,
                    source_file: "Cargo.toml".to_string(),
                });
            }
        }
    }

    Ok(deps)
}

fn extract_version(value: &toml::Value) -> String {
    match value {
        toml::Value::String(v) => v.clone(),
        toml::Value::Table(t) => {
            if let Some(v) = t.get("version").and_then(|v| v.as_str()) {
                v.to_string()
            } else if t.contains_key("workspace") {
                "workspace".to_string()
            } else if t.contains_key("git") {
                "git".to_string()
            } else if t.contains_key("path") {
                "path".to_string()
            } else {
                "*".to_string()
            }
        }
        _ => "*".to_string(),
    }
}
