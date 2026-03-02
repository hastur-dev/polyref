use crate::check::{Checker, Issue, Severity, ValidationResult};
use crate::detect::Language;
use crate::generate::{EntryKind, ReferenceFile};
use std::path::{Path, PathBuf};

pub struct PythonChecker;

impl Checker for PythonChecker {
    fn language(&self) -> Language {
        Language::Python
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
            if ext != Some("py") {
                continue;
            }

            files_checked += 1;
            let content = std::fs::read_to_string(source_file)?;
            let file_issues = check_python_file(source_file, &content, reference_files);
            issues.extend(file_issues);
        }

        Ok(ValidationResult {
            language: Language::Python,
            files_checked,
            issues,
        })
    }
}

fn check_python_file(
    file_path: &Path,
    content: &str,
    reference_files: &[ReferenceFile],
) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut imported_modules: Vec<(String, String)> = Vec::new(); // (alias, module_name)
    let mut imported_names: Vec<(String, String)> = Vec::new(); // (name, from_module)

    for (line_num, line) in content.lines().enumerate() {
        let line_num = line_num + 1;
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // Skip strings
        if trimmed.starts_with('"') || trimmed.starts_with('\'') {
            continue;
        }

        // Track imports
        if let Some(import_rest) = trimmed.strip_prefix("import ") {
            let module = import_rest.trim().to_string();
            let alias = if let Some(as_idx) = module.find(" as ") {
                module[as_idx + 4..].trim().to_string()
            } else {
                module.clone()
            };
            let module_name = if let Some(as_idx) = module.find(" as ") {
                module[..as_idx].trim().to_string()
            } else {
                module
            };
            imported_modules.push((alias, module_name));
            continue;
        }

        if trimmed.starts_with("from ") {
            if let Some(import_idx) = trimmed.find(" import ") {
                let module_name = trimmed[5..import_idx].trim().to_string();
                let names_str = &trimmed[import_idx + 8..];

                // Check each imported name against reference
                let ref_file = find_reference_file(&module_name, reference_files);
                if let Some(ref_file) = ref_file {
                    let known_names: Vec<String> =
                        ref_file.entries.iter().map(|e| e.name.clone()).collect();

                    for name in names_str.split(',') {
                        let name = name.trim().to_string();
                        let actual_name = if let Some(as_idx) = name.find(" as ") {
                            name[..as_idx].trim().to_string()
                        } else {
                            name.clone()
                        };

                        if actual_name.is_empty() || actual_name == "*" {
                            continue;
                        }

                        imported_names.push((actual_name.clone(), module_name.clone()));

                        if !known_names.contains(&actual_name) {
                            let suggestion = crate::check::common::suggest_correction(
                                &actual_name,
                                &known_names,
                            );
                            issues.push(Issue {
                                severity: Severity::Error,
                                message: format!(
                                    "'{}' is not exported by '{}'",
                                    actual_name, module_name
                                ),
                                file: file_path.to_path_buf(),
                                line: line_num,
                                column: line.find(actual_name.as_str()),
                                code_snippet: line.to_string(),
                                suggestion,
                                rule: "unknown-import".to_string(),
                            });
                        }
                    }
                }
            }
            continue;
        }

        // Check module.function() calls
        for (alias, module_name) in &imported_modules {
            let prefix = format!("{}.", alias);
            if let Some(pos) = trimmed.find(&prefix) {
                if crate::check::common::is_inside_string(trimmed, pos)
                    || crate::check::common::is_inside_comment(trimmed, pos, Language::Python)
                {
                    continue;
                }

                let after_dot = &trimmed[pos + prefix.len()..];
                let func_end = after_dot
                    .find(|c: char| !c.is_alphanumeric() && c != '_')
                    .unwrap_or(after_dot.len());
                let func_name = &after_dot[..func_end];

                if func_name.is_empty() {
                    continue;
                }

                let ref_file = find_reference_file(module_name, reference_files);
                if let Some(ref_file) = ref_file {
                    let known_functions: Vec<String> = ref_file
                        .entries
                        .iter()
                        .filter(|e| {
                            e.kind == EntryKind::Function
                                || e.kind == EntryKind::Method
                                || e.kind == EntryKind::Class
                        })
                        .map(|e| e.name.clone())
                        .collect();

                    if !known_functions.contains(&func_name.to_string()) {
                        let suggestion = crate::check::common::suggest_correction(
                            func_name,
                            &known_functions,
                        );
                        issues.push(Issue {
                            severity: Severity::Error,
                            message: format!(
                                "'{}' is not a known function in '{}'",
                                func_name, module_name
                            ),
                            file: file_path.to_path_buf(),
                            line: line_num,
                            column: Some(pos + prefix.len()),
                            code_snippet: line.to_string(),
                            suggestion,
                            rule: "unknown-function".to_string(),
                        });
                    } else {
                        // Check for missing required arguments
                        check_python_call_args(
                            file_path,
                            line_num,
                            line,
                            func_name,
                            &trimmed[pos..],
                            ref_file,
                            &mut issues,
                        );
                    }
                }
            }
        }

        // Check method calls on known objects
        check_python_method_calls(
            file_path,
            line_num,
            line,
            trimmed,
            reference_files,
            &mut issues,
        );
    }

    issues
}

fn find_reference_file<'a>(
    module_name: &str,
    reference_files: &'a [ReferenceFile],
) -> Option<&'a ReferenceFile> {
    reference_files.iter().find(|rf| {
        rf.library_name == module_name
            || rf.library_name.replace('-', "_") == module_name.replace('-', "_")
    })
}

fn check_python_call_args(
    file_path: &Path,
    line_num: usize,
    line: &str,
    func_name: &str,
    call_expr: &str,
    ref_file: &ReferenceFile,
    issues: &mut Vec<Issue>,
) {
    // Find the function entry
    let entry = ref_file.entries.iter().find(|e| e.name == func_name);
    let entry = match entry {
        Some(e) => e,
        None => return,
    };

    // Count args in the call
    if let Some(paren_start) = call_expr.find('(') {
        let call_part = &call_expr[paren_start..];
        let arg_count = crate::check::common::count_arguments(call_part);

        // Parse expected params from signature
        let sig = &entry.signature;
        if let Some(sig_paren) = sig.find('(') {
            let sig_params = &sig[sig_paren..];
            let expected_required = count_required_params(sig_params);

            if arg_count < expected_required {
                issues.push(Issue {
                    severity: Severity::Error,
                    message: format!(
                        "'{}' requires at least {} argument(s), got {}",
                        func_name, expected_required, arg_count
                    ),
                    file: file_path.to_path_buf(),
                    line: line_num,
                    column: line.find(func_name),
                    code_snippet: line.to_string(),
                    suggestion: None,
                    rule: "missing-required-arg".to_string(),
                });
            }
        }
    }
}

fn count_required_params(params_str: &str) -> usize {
    // Simple heuristic: count params before any default (=) or *args/**kwargs
    let inner = if params_str.starts_with('(') {
        let end = params_str.rfind(')').unwrap_or(params_str.len());
        &params_str[1..end]
    } else {
        params_str
    };

    if inner.trim().is_empty() {
        return 0;
    }

    let mut required = 0;
    for param in inner.split(',') {
        let param = param.trim();
        if param.is_empty() || param == "self" || param == "cls" {
            continue;
        }
        if param.starts_with('*') || param.contains('=') {
            break; // Everything after this is optional
        }
        required += 1;
    }

    required
}

fn check_python_method_calls(
    file_path: &Path,
    line_num: usize,
    line: &str,
    trimmed: &str,
    reference_files: &[ReferenceFile],
    issues: &mut Vec<Issue>,
) {
    // Look for .method_name( patterns
    let chars: Vec<char> = trimmed.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '.' && i + 1 < chars.len() && chars[i + 1].is_alphabetic() {
            if crate::check::common::is_inside_string(trimmed, i)
                || crate::check::common::is_inside_comment(trimmed, i, Language::Python)
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

                // Check against all reference files' class methods
                for ref_file in reference_files {
                    let methods: Vec<String> = ref_file
                        .entries
                        .iter()
                        .filter(|e| e.kind == EntryKind::Method || e.kind == EntryKind::Property)
                        .map(|e| e.name.clone())
                        .collect();

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
