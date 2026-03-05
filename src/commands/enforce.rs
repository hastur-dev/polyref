use crate::check::Checker;
use crate::config::Config;
use crate::coverage;
use crate::detect::Language;
use crate::enforce::{
    build_enforce_result, format_enforce_result, EnforceConfig, EnforceVerdict, OutputFormat,
};
use std::path::Path;

/// Map CLI args to an EnforceConfig.
pub fn build_enforce_config_from_args(
    enforce: bool,
    strict: bool,
    require_coverage: Option<u8>,
    from_stdin: bool,
    output_format: &str,
) -> EnforceConfig {
    EnforceConfig {
        hard_block: enforce,
        strict_unknown_packages: strict,
        require_coverage,
        from_stdin,
        output_format: match output_format {
            "json" => OutputFormat::Json,
            _ => OutputFormat::Human,
        },
    }
}

/// Detect language from file extension or content heuristics.
pub fn detect_language_from_content(content: &str, lang_hint: &str) -> Language {
    match lang_hint {
        "rust" => Language::Rust,
        "python" => Language::Python,
        "typescript" => Language::TypeScript,
        _ => {
            if content.contains("fn ")
                || content.contains("use std::")
                || content.contains("pub fn")
                || content.contains("let mut")
            {
                Language::Rust
            } else if content.contains("def ")
                || content.contains("import ")
                || content.contains("from ")
            {
                Language::Python
            } else if content.contains("function ")
                || content.contains("const ")
                || content.contains("interface ")
                || content.contains("=>")
            {
                Language::TypeScript
            } else {
                Language::Rust
            }
        }
    }
}

/// Read source code from stdin or a file path.
pub fn read_source_input(
    path: Option<&str>,
    from_stdin: bool,
) -> anyhow::Result<(String, String)> {
    if from_stdin {
        let mut buf = String::new();
        std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf)?;
        if buf.trim().is_empty() {
            anyhow::bail!("stdin was empty");
        }
        return Ok((buf, "<stdin>".to_string()));
    }

    let file_path = path.ok_or_else(|| anyhow::anyhow!("no source file specified"))?;
    let content = std::fs::read_to_string(file_path)?;
    if content.trim().is_empty() {
        anyhow::bail!("source file is empty: {}", file_path);
    }
    Ok((content, file_path.to_string()))
}

/// Run the enforce command logic.
#[allow(clippy::too_many_arguments)]
pub fn cmd_enforce(
    project: &str,
    enforce: bool,
    from_stdin: bool,
    strict: bool,
    require_coverage: Option<u8>,
    output_format: &str,
    lang: &str,
    refs_override: Option<&str>,
    global_refs_override: Option<&str>,
    strip_fences: bool,
) -> anyhow::Result<()> {
    let config =
        build_enforce_config_from_args(enforce, strict, require_coverage, from_stdin, output_format);
    config.validate().map_err(|e| anyhow::anyhow!(e))?;

    let (mut content, display_path) =
        read_source_input(if from_stdin { None } else { Some(project) }, from_stdin)?;

    // Check if strip_fences should be enabled from config or CLI
    let project_root = Path::new(project);
    let toml_config = Config::load(project_root).unwrap_or_default();
    let should_strip_fences = strip_fences || toml_config.model.strip_fences == Some(true);

    // Apply fence stripping if enabled
    if should_strip_fences {
        content = crate::model_output::extract_code_from_model_output(&content, lang);
    }

    let language = detect_language_from_content(&content, lang);

    let ref_files =
        load_ref_files_for_enforce(project, refs_override, global_refs_override, language)?;

    let issues = run_checker_on_content(&content, &display_path, &ref_files, language)?;

    let mut result = build_enforce_result(&issues, &config);

    let src_ctx = crate::source_context::build_source_context(&content);
    let cov_report = coverage::compute_coverage(&src_ctx, &ref_files);
    result.coverage_pct = Some(cov_report.coverage_pct);

    if let Some(reason) = coverage::check_coverage_gate(&cov_report, &config) {
        result.verdict = EnforceVerdict::Blocked;
        let cov_issue = crate::enforce::SerializedIssue {
            kind: "coverage_gate".to_string(),
            line: 0,
            message: reason,
            suggestion: None,
            similarity: None,
        };
        result.issues.push(cov_issue);
        result.issue_count = result.issues.len();
    }

    let output = format_enforce_result(&result, &config.output_format);
    println!("{}", output);

    if result.verdict == EnforceVerdict::Blocked {
        std::process::exit(1);
    }

    Ok(())
}

pub fn load_ref_files_for_enforce(
    project: &str,
    refs_override: Option<&str>,
    global_refs_override: Option<&str>,
    language: Language,
) -> anyhow::Result<Vec<crate::generate::ReferenceFile>> {
    let refs_dir = if let Some(dir) = refs_override {
        std::path::PathBuf::from(dir)
    } else {
        let project_root = Path::new(project);
        let cfg = Config::load(project_root).unwrap_or_default();
        cfg.resolved_refs_dir()
    };

    let global_refs = global_refs_override.map(std::path::PathBuf::from);
    let global_refs_path = global_refs.as_deref();

    load_refs_from_dir(&refs_dir, global_refs_path, language)
}

pub fn load_refs_from_dir(
    refs_dir: &Path,
    global_refs_path: Option<&Path>,
    language: Language,
) -> anyhow::Result<Vec<crate::generate::ReferenceFile>> {
    let mut ref_files = Vec::new();
    let lang_subdir = match language {
        Language::Rust => "rust",
        Language::Python => "python",
        Language::TypeScript => "typescript",
    };

    let lang_dir = refs_dir.join(lang_subdir);
    if lang_dir.exists() {
        load_refs_from_language_dir(&lang_dir, language, &mut ref_files)?;
    }

    // Also scan refs/std/ for stdlib reference files
    let std_dir = refs_dir.join("std");
    if std_dir.exists() {
        load_refs_from_language_dir(&std_dir, language, &mut ref_files)?;
    }

    if let Some(global_dir) = global_refs_path {
        if global_dir.exists() {
            load_refs_from_language_dir(global_dir, language, &mut ref_files)?;
        }
    }

    Ok(ref_files)
}

pub fn load_refs_from_language_dir(
    dir: &Path,
    language: Language,
    ref_files: &mut Vec<crate::generate::ReferenceFile>,
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
        let content = std::fs::read_to_string(&path)?;
        let lib_name = extract_lib_name_from_path(&path);
        let version = extract_version_from_content(&content);
        let parsed_entries = parse_ref_entries(&content, language);
        ref_files.push(crate::generate::ReferenceFile {
            library_name: lib_name,
            version,
            language,
            entries: parsed_entries,
            raw_content: content,
            file_path: path,
        });
    }
    Ok(())
}

/// Parse reference file entries using both v1 and v2 parsers, merging results.
pub fn parse_ref_entries(
    content: &str,
    language: Language,
) -> Vec<crate::generate::ReferenceEntry> {
    match language {
        Language::Rust => {
            let v1 = crate::generate::rust::parse_rust_reference(content);
            let v2 = crate::ref_parser_v2::parse_reference_file_v2(content);
            merge_entries(v1, v2)
        }
        _ => crate::ref_parser_v2::parse_reference_file_v2(content),
    }
}

/// Merge v1 and v2 entry lists. Prefer v2 entries (richer type_context).
pub fn merge_entries(
    v1: Vec<crate::generate::ReferenceEntry>,
    v2: Vec<crate::generate::ReferenceEntry>,
) -> Vec<crate::generate::ReferenceEntry> {
    let mut result = Vec::new();
    let mut seen: std::collections::HashSet<(String, Option<String>)> =
        std::collections::HashSet::new();

    for entry in &v2 {
        let key = (entry.name.clone(), entry.type_context.clone());
        if seen.insert(key) {
            result.push(entry.clone());
        }
    }

    for entry in &v1 {
        let key = (entry.name.clone(), entry.type_context.clone());
        if seen.insert(key) {
            result.push(entry.clone());
        }
    }

    result
}

pub fn extract_lib_name_from_path(path: &Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");
    let name = stem
        .strip_prefix("lib_")
        .unwrap_or(stem)
        .replace(".polyref", "");
    name.split('.').next().unwrap_or(&name).to_string()
}

pub fn extract_version_from_content(content: &str) -> String {
    for line in content.lines().take(10) {
        let trimmed = line.trim().trim_start_matches("//").trim();
        if let Some(rest) = trimmed.strip_prefix("Version:") {
            return rest.trim().to_string();
        }
        if let Some(rest) = trimmed.strip_prefix("version:") {
            return rest.trim().to_string();
        }
    }
    "unknown".to_string()
}

pub fn run_checker_on_content(
    content: &str,
    display_path: &str,
    ref_files: &[crate::generate::ReferenceFile],
    language: Language,
) -> anyhow::Result<Vec<crate::check::Issue>> {
    let ext = match language {
        Language::Rust => "rs",
        Language::Python => "py",
        Language::TypeScript => "ts",
    };

    let tmp_dir = std::env::temp_dir();
    let tmp_path = tmp_dir.join(format!("polyref_enforce_{}.{}", std::process::id(), ext));
    std::fs::write(&tmp_path, content)?;

    let result = match language {
        Language::Rust => {
            let checker = crate::check::rust::RustChecker;
            checker.check(std::slice::from_ref(&tmp_path), ref_files)?
        }
        Language::Python => {
            let checker = crate::check::python::PythonChecker;
            checker.check(std::slice::from_ref(&tmp_path), ref_files)?
        }
        Language::TypeScript => {
            let checker = crate::check::typescript::TypeScriptChecker;
            checker.check(std::slice::from_ref(&tmp_path), ref_files)?
        }
    };

    let issues: Vec<crate::check::Issue> = result
        .issues
        .into_iter()
        .map(|mut issue| {
            issue.file = std::path::PathBuf::from(display_path);
            issue
        })
        .collect();

    let _ = std::fs::remove_file(&tmp_path);
    Ok(issues)
}
