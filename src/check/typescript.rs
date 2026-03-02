use crate::check::{Checker, Issue, Severity, ValidationResult};
use crate::detect::Language;
use crate::generate::{EntryKind, ReferenceFile};
use std::path::{Path, PathBuf};

pub struct TypeScriptChecker;

impl Checker for TypeScriptChecker {
    fn language(&self) -> Language {
        Language::TypeScript
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
            if ext != Some("ts") && ext != Some("tsx") {
                continue;
            }

            files_checked += 1;
            let content = std::fs::read_to_string(source_file)?;
            let file_issues = check_ts_file(source_file, &content, reference_files);
            issues.extend(file_issues);
        }

        Ok(ValidationResult {
            language: Language::TypeScript,
            files_checked,
            issues,
        })
    }
}

fn check_ts_file(
    file_path: &Path,
    content: &str,
    reference_files: &[ReferenceFile],
) -> Vec<Issue> {
    let mut issues = Vec::new();
    let mut imported_names: Vec<(String, String)> = Vec::new(); // (name, from_module)

    for (line_num, line) in content.lines().enumerate() {
        let line_num = line_num + 1;
        let trimmed = line.trim();

        // Skip empty lines and comments
        if trimmed.is_empty() || trimmed.starts_with("//") || trimmed.starts_with("/*") {
            continue;
        }

        // Parse import statements
        if trimmed.starts_with("import ") {
            check_ts_import(
                file_path,
                line_num,
                line,
                trimmed,
                reference_files,
                &mut issues,
                &mut imported_names,
            );
            continue;
        }

        // Check function calls from imported names
        check_ts_calls(
            file_path,
            line_num,
            line,
            trimmed,
            reference_files,
            &imported_names,
            &mut issues,
        );

        // Check destructuring patterns for hooks
        check_ts_destructuring(
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

fn check_ts_import(
    file_path: &Path,
    line_num: usize,
    line: &str,
    trimmed: &str,
    reference_files: &[ReferenceFile],
    issues: &mut Vec<Issue>,
    imported_names: &mut Vec<(String, String)>,
) {
    // Extract module name from 'module' or "module"
    let module_name = extract_module_name(trimmed);
    let module_name = match module_name {
        Some(m) => m,
        None => return,
    };

    // Find reference file for this module
    let ref_file = reference_files.iter().find(|rf| {
        rf.library_name == module_name
            || rf.library_name.replace('-', "_") == module_name.replace('-', "_")
    });

    let ref_file = match ref_file {
        Some(rf) => rf,
        None => return,
    };

    // Extract named imports
    let names = crate::check::common::extract_import_names(trimmed, Language::TypeScript);
    let known_names: Vec<String> = ref_file.entries.iter().map(|e| e.name.clone()).collect();

    for name in &names {
        imported_names.push((name.clone(), module_name.clone()));

        if !known_names.contains(name) {
            let suggestion = crate::check::common::suggest_correction(name, &known_names);
            issues.push(Issue {
                severity: Severity::Error,
                message: format!("'{}' is not exported by '{}'", name, module_name),
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

fn extract_module_name(line: &str) -> Option<String> {
    // Find 'module' or "module" after "from"
    let from_idx = line.find("from ")?;
    let after_from = &line[from_idx + 5..];
    let quote_char = if after_from.contains('\'') {
        '\''
    } else if after_from.contains('"') {
        '"'
    } else {
        return None;
    };
    let start = after_from.find(quote_char)? + 1;
    let end = after_from[start..].find(quote_char)? + start;
    Some(after_from[start..end].to_string())
}

fn check_ts_calls(
    file_path: &Path,
    line_num: usize,
    line: &str,
    trimmed: &str,
    reference_files: &[ReferenceFile],
    imported_names: &[(String, String)],
    issues: &mut Vec<Issue>,
) {
    // Check for function calls of imported names
    for (name, module_name) in imported_names {
        let pattern = format!("{}(", name);
        if let Some(pos) = trimmed.find(&pattern) {
            if crate::check::common::is_inside_string(trimmed, pos)
                || crate::check::common::is_inside_comment(trimmed, pos, Language::TypeScript)
            {
                continue;
            }

            // Verify the function exists in the reference
            let ref_file = reference_files.iter().find(|rf| {
                rf.library_name == *module_name
                    || rf.library_name.replace('-', "_") == module_name.replace('-', "_")
            });

            if let Some(ref_file) = ref_file {
                let known_functions: Vec<String> = ref_file
                    .entries
                    .iter()
                    .filter(|e| {
                        e.kind == EntryKind::Function
                            || e.kind == EntryKind::Hook
                            || e.kind == EntryKind::Component
                    })
                    .map(|e| e.name.clone())
                    .collect();

                if !known_functions.contains(name) {
                    let suggestion =
                        crate::check::common::suggest_correction(name, &known_functions);
                    issues.push(Issue {
                        severity: Severity::Error,
                        message: format!(
                            "'{}' is not a known function in '{}'",
                            name, module_name
                        ),
                        file: file_path.to_path_buf(),
                        line: line_num,
                        column: line.find(name.as_str()),
                        code_snippet: line.to_string(),
                        suggestion,
                        rule: "unknown-function".to_string(),
                    });
                }
            }
        }
    }
}

fn check_ts_destructuring(
    file_path: &Path,
    line_num: usize,
    line: &str,
    trimmed: &str,
    _reference_files: &[ReferenceFile],
    issues: &mut Vec<Issue>,
) {
    // Check: const [a, b, c] = useState(...)  — useState returns 2-tuple
    if !trimmed.contains("const [") && !trimmed.contains("let [") {
        return;
    }

    // Extract the destructured names count
    if let Some(bracket_start) = trimmed.find('[') {
        if let Some(bracket_end) = trimmed.find(']') {
            let names = &trimmed[bracket_start + 1..bracket_end];
            let count = names.split(',').count();

            // Find what function is being called
            if let Some(eq_idx) = trimmed.find('=') {
                let rhs = trimmed[eq_idx + 1..].trim();

                // Check if it's a useState call
                if rhs.starts_with("useState") && count != 2 {
                    issues.push(Issue {
                        severity: Severity::Error,
                        message: format!(
                            "useState returns [state, setState] (2 elements), but {} elements destructured",
                            count
                        ),
                        file: file_path.to_path_buf(),
                        line: line_num,
                        column: trimmed.find('['),
                        code_snippet: line.to_string(),
                        suggestion: Some("useState returns [state, setState]".to_string()),
                        rule: "wrong-destructure".to_string(),
                    });
                }

                // Check if it's a useReducer call
                if rhs.starts_with("useReducer") && count != 2 {
                    issues.push(Issue {
                        severity: Severity::Error,
                        message: format!(
                            "useReducer returns [state, dispatch] (2 elements), but {} elements destructured",
                            count
                        ),
                        file: file_path.to_path_buf(),
                        line: line_num,
                        column: trimmed.find('['),
                        code_snippet: line.to_string(),
                        suggestion: Some("useReducer returns [state, dispatch]".to_string()),
                        rule: "wrong-destructure".to_string(),
                    });
                }
            }
        }
    }
}
