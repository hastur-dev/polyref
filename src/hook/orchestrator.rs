use crate::check::{Checker, ValidationResult};
use crate::config::Config;
use crate::detect::Language;
use crate::generate::Generator;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum HookEvent {
    SessionStart,
    PostToolUse {
        tool_name: String,
        file_changed: Option<PathBuf>,
    },
    Stop,
}

#[derive(Debug, Clone)]
pub struct HookResponse {
    pub should_report: bool,
    pub results: Vec<ValidationResult>,
    pub message: String,
}

/// Handle a hook event and return appropriate response
pub fn handle_event(event: HookEvent, config: &Config) -> anyhow::Result<HookResponse> {
    match event {
        HookEvent::SessionStart => handle_session_start(config),
        HookEvent::PostToolUse { file_changed, .. } => {
            handle_post_tool_use(config, file_changed.as_deref())
        }
        HookEvent::Stop => handle_stop(config),
    }
}

fn handle_session_start(config: &Config) -> anyhow::Result<HookResponse> {
    // Detect and generate references
    let detected = crate::detect::detect(&config.project_root)?;
    let refs_dir = config.resolved_refs_dir();
    let global_refs = config.resolved_global_refs_dir();
    let global_refs_path = global_refs.as_deref();

    let mut ref_count = 0;
    for dep in &detected.dependencies {
        match dep.language {
            Language::Rust => {
                let gen = crate::generate::rust::RustGenerator;
                if let Ok(rf) = gen.generate(dep, &refs_dir, global_refs_path) {
                    ref_count += rf.entries.len();
                }
            }
            Language::Python => {
                let gen = crate::generate::python::PythonGenerator;
                if let Ok(rf) = gen.generate(dep, &refs_dir, global_refs_path) {
                    ref_count += rf.entries.len();
                }
            }
            Language::TypeScript => {
                let gen = crate::generate::typescript::TypeScriptGenerator;
                if let Ok(rf) = gen.generate(dep, &refs_dir, global_refs_path) {
                    ref_count += rf.entries.len();
                }
            }
        }
    }

    Ok(HookResponse {
        should_report: false,
        results: vec![],
        message: format!(
            "PolyRef: Generated references ({} entries for {} deps)",
            ref_count,
            detected.dependencies.len()
        ),
    })
}

fn handle_post_tool_use(
    config: &Config,
    file_changed: Option<&Path>,
) -> anyhow::Result<HookResponse> {
    let file = match file_changed {
        Some(f) => f,
        None => {
            return Ok(HookResponse {
                should_report: false,
                results: vec![],
                message: "No file changed".to_string(),
            })
        }
    };

    // Only check source files
    let ext = file.extension().and_then(|e| e.to_str()).unwrap_or("");
    if !matches!(ext, "rs" | "py" | "ts" | "tsx") {
        return Ok(HookResponse {
            should_report: false,
            results: vec![],
            message: format!("Skipping non-source file: {}", file.display()),
        });
    }

    // Run check on the changed file
    let results = run_check_on_files(config, &[file.to_path_buf()])?;
    let total_errors: usize = results.iter().map(|r| r.error_count()).sum();

    Ok(HookResponse {
        should_report: total_errors > 0,
        results,
        message: if total_errors > 0 {
            format!("PolyRef found {} error(s)", total_errors)
        } else {
            "PolyRef: No issues found".to_string()
        },
    })
}

fn handle_stop(config: &Config) -> anyhow::Result<HookResponse> {
    // Full check on all source files
    let detected = crate::detect::detect(&config.project_root)?;
    let source_files = find_source_files(&config.project_root, &detected.languages);
    let results = run_check_on_files(config, &source_files)?;

    let total_errors: usize = results.iter().map(|r| r.error_count()).sum();
    let total_warnings: usize = results.iter().map(|r| r.warning_count()).sum();
    let total_files: usize = results.iter().map(|r| r.files_checked).sum();

    Ok(HookResponse {
        should_report: true,
        results,
        message: format!(
            "PolyRef Summary: {} errors, {} warnings ({} files checked)",
            total_errors, total_warnings, total_files
        ),
    })
}

fn run_check_on_files(
    config: &Config,
    files: &[PathBuf],
) -> anyhow::Result<Vec<ValidationResult>> {
    let refs_dir = config.resolved_refs_dir();
    let global_refs = config.resolved_global_refs_dir();
    let global_refs_path = global_refs.as_deref();
    let mut results = Vec::new();

    // Load reference files
    let detected = crate::detect::detect(&config.project_root)?;
    let mut ref_files = Vec::new();

    for dep in &detected.dependencies {
        match dep.language {
            Language::Rust => {
                let gen = crate::generate::rust::RustGenerator;
                if let Ok(rf) = gen.generate(dep, &refs_dir, global_refs_path) {
                    ref_files.push(rf);
                }
            }
            Language::Python => {
                let gen = crate::generate::python::PythonGenerator;
                if let Ok(rf) = gen.generate(dep, &refs_dir, global_refs_path) {
                    ref_files.push(rf);
                }
            }
            Language::TypeScript => {
                let gen = crate::generate::typescript::TypeScriptGenerator;
                if let Ok(rf) = gen.generate(dep, &refs_dir, global_refs_path) {
                    ref_files.push(rf);
                }
            }
        }
    }

    // Run Rust checker
    let rust_files: Vec<PathBuf> = files.iter().filter(|f| f.extension().and_then(|e| e.to_str()) == Some("rs")).cloned().collect();
    if !rust_files.is_empty() {
        let checker = crate::check::rust::RustChecker;
        results.push(checker.check(&rust_files, &ref_files)?);
    }

    // Run Python checker
    let py_files: Vec<PathBuf> = files.iter().filter(|f| f.extension().and_then(|e| e.to_str()) == Some("py")).cloned().collect();
    if !py_files.is_empty() {
        let checker = crate::check::python::PythonChecker;
        results.push(checker.check(&py_files, &ref_files)?);
    }

    // Run TypeScript checker
    let ts_files: Vec<PathBuf> = files.iter().filter(|f| {
        let ext = f.extension().and_then(|e| e.to_str()).unwrap_or("");
        ext == "ts" || ext == "tsx"
    }).cloned().collect();
    if !ts_files.is_empty() {
        let checker = crate::check::typescript::TypeScriptChecker;
        results.push(checker.check(&ts_files, &ref_files)?);
    }

    Ok(results)
}

fn find_source_files(root: &Path, languages: &[Language]) -> Vec<PathBuf> {
    let mut files = Vec::new();

    let walker = walkdir::WalkDir::new(root)
        .max_depth(10)
        .into_iter()
        .filter_map(|e| e.ok());

    for entry in walker {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        // Skip generated reference files
        if path.starts_with(root.join("refs")) || path.starts_with(root.join("target")) {
            continue;
        }

        match ext {
            "rs" if languages.contains(&Language::Rust) => files.push(path.to_path_buf()),
            "py" if languages.contains(&Language::Python) => files.push(path.to_path_buf()),
            "ts" | "tsx" if languages.contains(&Language::TypeScript) => {
                files.push(path.to_path_buf())
            }
            _ => {}
        }
    }

    files
}
