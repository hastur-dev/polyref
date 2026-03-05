use crate::config::DriftConfig;
use crate::http_client::HttpClient;
use crate::registry::RegistryClient;

/// Result of checking a single reference file for drift.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DriftResult {
    pub library_name: String,
    pub ref_version: String,
    pub latest_version: Option<String>,
    pub registry: String,
    pub has_drift: bool,
    pub error: Option<String>,
}

/// Scan reference files and compare versions against upstream registries.
pub fn detect_drift(config: &DriftConfig) -> anyhow::Result<Vec<DriftResult>> {
    let client = HttpClient::new(config.http_proxy.clone());
    let mut results = Vec::new();

    for refs_dir in &config.refs_dirs {
        let dir = std::path::Path::new(refs_dir);
        if !dir.exists() {
            continue;
        }
        scan_refs_dir(dir, config, &client, &mut results)?;
    }

    Ok(results)
}

fn scan_refs_dir(
    dir: &std::path::Path,
    config: &DriftConfig,
    client: &HttpClient,
    results: &mut Vec<DriftResult>,
) -> anyhow::Result<()> {
    for subdir in &["rust", "ts", "std"] {
        let lang_dir = dir.join(subdir);
        if lang_dir.exists() {
            scan_language_dir(&lang_dir, subdir, config, client, results)?;
        }
    }

    // Also scan flat layout at refs root
    scan_language_dir(dir, "auto", config, client, results)?;

    Ok(())
}

fn scan_language_dir(
    dir: &std::path::Path,
    lang_hint: &str,
    config: &DriftConfig,
    client: &HttpClient,
    results: &mut Vec<DriftResult>,
) -> anyhow::Result<()> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let lib_name = extract_lib_name(&path);
        if lib_name.starts_with("std_") {
            continue; // Skip stdlib refs
        }
        if config.should_skip(&lib_name) {
            continue;
        }

        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let ref_version = extract_version(&content);

        let registry = detect_registry(lang_hint, &path);
        let drift = check_single_drift(&lib_name, &ref_version, &registry, client);
        results.push(drift);
    }

    Ok(())
}

pub fn extract_lib_name(path: &std::path::Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    stem.strip_prefix("lib_").unwrap_or(stem).to_string()
}

pub fn extract_version(content: &str) -> String {
    for line in content.lines().take(10) {
        let trimmed = line.trim().trim_start_matches("//").trim().trim_start_matches('#').trim();
        if let Some(rest) = trimmed.strip_prefix("Version:") {
            return rest.trim().to_string();
        }
    }
    "unknown".to_string()
}

pub fn detect_registry(lang_hint: &str, path: &std::path::Path) -> String {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    match (lang_hint, ext) {
        ("rust", _) | (_, "rs") => "crates.io".to_string(),
        ("ts", _) => "npm".to_string(),
        _ if ext == "polyref" => {
            // Check @lang inside file
            let content = std::fs::read_to_string(path).unwrap_or_default();
            if content.contains("@lang python") {
                "pypi".to_string()
            } else if content.contains("@lang typescript") {
                "npm".to_string()
            } else {
                "unknown".to_string()
            }
        }
        _ => "unknown".to_string(),
    }
}

fn check_single_drift(
    lib_name: &str,
    ref_version: &str,
    registry: &str,
    _client: &HttpClient,
) -> DriftResult {
    let latest = match registry {
        "crates.io" => {
            let c = crate::registry::crates_io::CratesIoClient::new(HttpClient::new(None));
            c.get_latest_version(lib_name)
        }
        "pypi" => {
            let c = crate::registry::pypi::PypiClient::new(HttpClient::new(None));
            c.get_latest_version(lib_name)
        }
        "npm" => {
            let c = crate::registry::npm::NpmClient::new(HttpClient::new(None));
            c.get_latest_version(lib_name)
        }
        _ => Err(anyhow::anyhow!("unknown registry: {}", registry)),
    };

    match latest {
        Ok(rv) => {
            let has_drift = ref_version != "unknown" && ref_version != rv.version;
            DriftResult {
                library_name: lib_name.to_string(),
                ref_version: ref_version.to_string(),
                latest_version: Some(rv.version),
                registry: registry.to_string(),
                has_drift,
                error: None,
            }
        }
        Err(e) => DriftResult {
            library_name: lib_name.to_string(),
            ref_version: ref_version.to_string(),
            latest_version: None,
            registry: registry.to_string(),
            has_drift: false,
            error: Some(e.to_string()),
        },
    }
}
