use super::{Dependency, Language};
use std::path::Path;

/// Detect TypeScript dependencies from package.json
pub fn detect_typescript(project_root: &Path) -> anyhow::Result<Vec<Dependency>> {
    let package_json_path = project_root.join("package.json");
    if !package_json_path.exists() {
        return Ok(vec![]);
    }

    let content = std::fs::read_to_string(&package_json_path)?;
    let doc: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse package.json: {}", e))?;

    let mut deps = Vec::new();

    // Parse dependencies, devDependencies, peerDependencies
    for section in &["dependencies", "devDependencies", "peerDependencies"] {
        if let Some(obj) = doc.get(section).and_then(|v| v.as_object()) {
            for (name, value) in obj {
                // Filter out @types/* packages — they are type declarations, not real deps
                if name.starts_with("@types/") {
                    continue;
                }

                let version = match value.as_str() {
                    Some(v) => {
                        if v.starts_with("workspace:") {
                            "workspace".to_string()
                        } else {
                            v.to_string()
                        }
                    }
                    None => "*".to_string(),
                };

                deps.push(Dependency {
                    name: name.clone(),
                    version,
                    language: Language::TypeScript,
                    source_file: "package.json".to_string(),
                });
            }
        }
    }

    Ok(deps)
}
