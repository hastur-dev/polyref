use crate::check::{Checker, Issue, Severity, ValidationResult};
use crate::detect::Language;
use crate::generate::{EntryKind, ReferenceFile};
use std::path::{Path, PathBuf};

pub struct RustChecker;

impl Checker for RustChecker {
    fn language(&self) -> Language {
        Language::Rust
    }

    fn check(
        &self,
        source_files: &[PathBuf],
        reference_files: &[ReferenceFile],
    ) -> anyhow::Result<ValidationResult> {
        let mut issues = Vec::new();
        let mut files_checked = 0;

        for source_file in source_files {
            if !source_file.exists() {
                continue;
            }
            let ext = source_file.extension().and_then(|e| e.to_str());
            if ext != Some("rs") {
                continue;
            }

            files_checked += 1;
            let content = std::fs::read_to_string(source_file)?;
            let file_issues = check_rust_file(source_file, &content, reference_files);
            issues.extend(file_issues);
        }

        Ok(ValidationResult {
            language: Language::Rust,
            files_checked,
            issues,
        })
    }
}

fn check_rust_file(
    file_path: &Path,
    content: &str,
    reference_files: &[ReferenceFile],
) -> Vec<Issue> {
    let mut issues = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let line_num = line_num + 1;
        let trimmed = line.trim();

        // Skip empty lines and pure comments
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        // Check use statements
        if trimmed.starts_with("use ") {
            check_rust_use_statement(file_path, line_num, trimmed, reference_files, &mut issues);
        }

        // Check function/method calls
        check_rust_calls(file_path, line_num, trimmed, reference_files, &mut issues);
    }

    issues
}

fn check_rust_use_statement(
    file_path: &Path,
    line_num: usize,
    line: &str,
    reference_files: &[ReferenceFile],
    issues: &mut Vec<Issue>,
) {
    // Parse: use crate_name::{items} or use crate_name::Item
    let after_use = line.trim_start_matches("use ").trim_end_matches(';').trim();

    // Extract crate name
    let crate_name = if let Some(idx) = after_use.find("::") {
        &after_use[..idx]
    } else {
        return; // Simple use without :: — not a crate import
    };

    // Find matching reference file
    let ref_file = reference_files.iter().find(|rf| {
        rf.library_name == crate_name || rf.library_name.replace('-', "_") == crate_name
    });

    let ref_file = match ref_file {
        Some(rf) => rf,
        None => return, // No reference file for this crate — skip
    };

    // Extract imported names
    let names = crate::check::common::extract_import_names(line, Language::Rust);
    let known_names: Vec<String> = ref_file.entries.iter().map(|e| e.name.clone()).collect();

    for name in &names {
        if name == "self" || name == "super" || name == "crate" || name == "*" {
            continue;
        }
        if !known_names.contains(name) {
            let suggestion = crate::check::common::suggest_correction(name, &known_names);
            issues.push(Issue {
                severity: Severity::Error,
                message: format!("'{}' is not exported by '{}'", name, crate_name),
                file: file_path.to_path_buf(),
                line: line_num,
                column: line.find(name.as_str()),
                code_snippet: line.to_string(),
                suggestion,
                rule: "unknown-import".to_string(),
            });
        }
    }
}

fn check_rust_calls(
    file_path: &Path,
    line_num: usize,
    line: &str,
    reference_files: &[ReferenceFile],
    issues: &mut Vec<Issue>,
) {
    // Skip lines that are inside strings or comments
    if line.trim().starts_with("//") || line.trim().starts_with("/*") {
        return;
    }

    // Check for method calls: expr.method(...)
    // Look for .identifier( patterns
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '.' && i + 1 < chars.len() && chars[i + 1].is_alphabetic() {
            // Check not inside string or comment
            if crate::check::common::is_inside_string(line, i)
                || crate::check::common::is_inside_comment(line, i, Language::Rust)
            {
                i += 1;
                continue;
            }

            let method_start = i + 1;
            let mut method_end = method_start;
            while method_end < chars.len()
                && (chars[method_end].is_alphanumeric() || chars[method_end] == '_')
            {
                method_end += 1;
            }

            if method_end < chars.len() && chars[method_end] == '(' {
                let method_name: String = chars[method_start..method_end].iter().collect();

                // Try to find this method in reference files
                for ref_file in reference_files {
                    let methods: Vec<String> = ref_file
                        .entries
                        .iter()
                        .filter(|e| e.kind == EntryKind::Method)
                        .map(|e| e.name.clone())
                        .collect();

                    // Only flag if we have methods from this crate and the method is close
                    // to a known one (suggesting a typo)
                    if !methods.is_empty() {
                        let known: Vec<&str> = methods.iter().map(|s| s.as_str()).collect();
                        if let Some((suggestion, score)) =
                            crate::check::common::fuzzy_match(&method_name, &known, 0.6)
                        {
                            if score < 1.0 && !methods.contains(&method_name) {
                                issues.push(Issue {
                                    severity: Severity::Error,
                                    message: format!(
                                        "'{}' is not a known method",
                                        method_name
                                    ),
                                    file: file_path.to_path_buf(),
                                    line: line_num,
                                    column: Some(method_start),
                                    code_snippet: line.to_string(),
                                    suggestion: Some(format!("did you mean '{}'?", suggestion)),
                                    rule: "unknown-method".to_string(),
                                });
                            }
                        }
                    }
                }
            }
        }
        i += 1;
    }
}
