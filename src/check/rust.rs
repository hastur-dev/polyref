use crate::check::{Checker, Issue, Severity, ValidationResult};
use crate::detect::Language;
use crate::generate::{EntryKind, ReferenceFile};
use std::path::{Path, PathBuf};

pub struct RustChecker;

/// Bundles common arguments for method checking helpers.
struct CheckCtx<'a> {
    file_path: &'a Path,
    line_num: usize,
    line: &'a str,
    issues: &'a mut Vec<Issue>,
}

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

    let src_ctx = crate::source_context::build_source_context(content);
    let relevant_refs = crate::source_context::select_relevant_ref_files(&src_ctx, reference_files);
    let lines: Vec<&str> = content.lines().collect();
    let type_ctx = crate::type_inference::build_type_context(&lines);
    let all_entries: Vec<_> = relevant_refs.iter().flat_map(|rf| rf.entries.iter()).cloned().collect();

    for (line_num, line) in content.lines().enumerate() {
        let line_num = line_num + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        if trimmed.starts_with("use ") {
            check_rust_use_statement(file_path, line_num, trimmed, &relevant_refs, &mut issues);
        }
        check_rust_calls_enhanced(file_path, line_num, trimmed, &relevant_refs, &type_ctx, &mut issues);
        check_associated_patterns(file_path, trimmed, line_num, &relevant_refs, &all_entries, &mut issues);
    }

    issues
}

fn check_associated_patterns(
    file_path: &Path,
    line: &str,
    line_num: usize,
    relevant_refs: &[&ReferenceFile],
    all_entries: &[crate::generate::ReferenceEntry],
    issues: &mut Vec<Issue>,
) {
    let assoc_calls = crate::associated_checker::extract_associated_calls(line, line_num);
    if !assoc_calls.is_empty() {
        check_associated_fn_calls(file_path, line, &assoc_calls, all_entries, issues);
    }

    let crate_calls = crate::associated_checker::extract_crate_calls(line, line_num);
    if !crate_calls.is_empty() {
        emit_crate_call_issues(file_path, line, &crate_calls, relevant_refs, issues);
    }

    let variant_calls = crate::associated_checker::extract_enum_variant_calls(line, line_num);
    if !variant_calls.is_empty() {
        emit_enum_variant_issues(file_path, line, &variant_calls, all_entries, issues);
    }
}

fn emit_crate_call_issues(
    file_path: &Path,
    line: &str,
    calls: &[crate::associated_checker::AssociatedCall],
    refs: &[&ReferenceFile],
    issues: &mut Vec<Issue>,
) {
    for ci in &crate::associated_checker::check_crate_calls(calls, refs) {
        let msg = crate::associated_checker::format_associated_issue(ci);
        issues.push(Issue {
            severity: Severity::Warning,
            message: msg,
            file: file_path.to_path_buf(),
            line: ci.line_number,
            column: None,
            code_snippet: line.to_string(),
            suggestion: ci.suggestion.as_ref().map(|s| format!("did you mean '{}::{}'?", ci.type_name, s)),
            rule: "unknown-associated-fn".to_string(),
        });
    }
}

fn emit_enum_variant_issues(
    file_path: &Path,
    line: &str,
    calls: &[crate::associated_checker::AssociatedCall],
    all_entries: &[crate::generate::ReferenceEntry],
    issues: &mut Vec<Issue>,
) {
    for vi in &crate::associated_checker::check_enum_variant_calls(calls, all_entries) {
        let suffix = vi.suggestion.as_ref()
            .map(|s| format!(" — did you mean '{}::{}' (similarity: {:.2})?", vi.type_name, s, vi.confidence))
            .unwrap_or_else(|| " — check docs".to_string());
        let msg = format!("unknown enum variant '{}::{}'{}",  vi.type_name, vi.fn_name, suffix);
        issues.push(Issue {
            severity: Severity::Warning,
            message: msg,
            file: file_path.to_path_buf(),
            line: vi.line_number,
            column: None,
            code_snippet: line.to_string(),
            suggestion: vi.suggestion.as_ref().map(|s| format!("did you mean '{}::{}'?", vi.type_name, s)),
            rule: "unknown-enum-variant".to_string(),
        });
    }
}

fn check_associated_fn_calls(
    file_path: &Path,
    line: &str,
    assoc_calls: &[crate::associated_checker::AssociatedCall],
    all_entries: &[crate::generate::ReferenceEntry],
    issues: &mut Vec<Issue>,
) {
    let assoc_issues =
        crate::associated_checker::check_associated_calls(assoc_calls, all_entries);
    for ai in &assoc_issues {
        let msg = crate::associated_checker::format_associated_issue(ai);
        issues.push(Issue {
            severity: Severity::Warning,
            message: msg,
            file: file_path.to_path_buf(),
            line: ai.line_number,
            column: None,
            code_snippet: line.to_string(),
            suggestion: ai.suggestion.as_ref().map(|s| {
                format!("did you mean '{}::{}'?", ai.type_name, s)
            }),
            rule: "unknown-associated-fn".to_string(),
        });
    }
}

fn check_rust_use_statement(
    file_path: &Path,
    line_num: usize,
    line: &str,
    reference_files: &[&ReferenceFile],
    issues: &mut Vec<Issue>,
) {
    let after_use = line.trim_start_matches("use ").trim_end_matches(';').trim();

    let crate_name = if let Some(idx) = after_use.find("::") {
        &after_use[..idx]
    } else {
        return;
    };

    let ref_file = reference_files.iter().find(|rf| {
        rf.library_name == crate_name
            || rf.library_name.replace('-', "_") == crate_name
    });

    let ref_file = match ref_file {
        Some(rf) => rf,
        None => return,
    };

    // Validate module path segments against known modules
    let path_parts: Vec<&str> = after_use.split("::").collect();
    if path_parts.len() > 1 {
        let known_modules: Vec<String> = ref_file
            .entries
            .iter()
            .filter(|e| e.kind == EntryKind::Module)
            .map(|e| e.name.clone())
            .collect();

        // Check intermediate path segments (skip crate name and final item)
        for &segment in &path_parts[1..path_parts.len().saturating_sub(1)] {
            let seg = segment.trim_start_matches('{').trim();
            if seg.is_empty() || seg == "self" {
                continue;
            }
            if !known_modules.is_empty() && !known_modules.contains(&seg.to_string()) {
                // Check if it's a known type/struct/enum instead of a module
                let known_all: Vec<String> = ref_file.entries.iter().map(|e| e.name.clone()).collect();
                if !known_all.contains(&seg.to_string()) {
                    let suggestion = crate::check::common::suggest_correction(seg, &known_modules);
                    issues.push(Issue {
                        severity: Severity::Error,
                        message: format!("module '{}' not found in '{}'", seg, crate_name),
                        file: file_path.to_path_buf(),
                        line: line_num,
                        column: line.find(seg),
                        code_snippet: line.to_string(),
                        suggestion,
                        rule: "unknown-import".to_string(),
                    });
                }
            }
        }
    }

    let names = crate::check::common::extract_import_names(line, Language::Rust);
    let known: Vec<String> = ref_file.entries.iter().map(|e| e.name.clone()).collect();

    for name in &names {
        if name == "self" || name == "super" || name == "crate" || name == "*" {
            continue;
        }
        if !known.contains(name) {
            let suggestion = crate::check::common::suggest_correction(name, &known);
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

/// Enhanced method call checking with lowered threshold (0.35) and universal flagging.
fn check_rust_calls_enhanced(
    file_path: &Path,
    line_num: usize,
    line: &str,
    reference_files: &[&ReferenceFile],
    type_ctx: &crate::type_inference::TypeContext,
    issues: &mut Vec<Issue>,
) {
    if line.trim().starts_with("//") || line.trim().starts_with("/*") {
        return;
    }

    let chars: Vec<char> = line.chars().collect();
    let all_methods: Vec<String> = collect_all_methods(reference_files);
    let all_entries: Vec<_> = reference_files.iter().flat_map(|rf| rf.entries.iter()).collect();

    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '.' && i + 1 < chars.len() && chars[i + 1].is_alphabetic() {
            if crate::check::common::is_inside_string(line, i)
                || crate::check::common::is_inside_comment(line, i, Language::Rust)
            {
                i += 1;
                continue;
            }
            let (method_name, method_start, method_end) = match extract_method_call(&chars, i) {
                Some(m) => m,
                None => { i += 1; continue; }
            };
            let receiver = extract_receiver(&chars, i);
            let entries_owned: Vec<_> = all_entries.iter().copied().cloned().collect();
            let type_result = crate::type_inference::check_method_with_type_context(
                &method_name, &receiver, type_ctx, &entries_owned,
            );
            let mut ctx = CheckCtx { file_path, line_num, line, issues };
            handle_method_check(
                &mut ctx, &method_name, method_start, method_end,
                type_result, &all_methods, &all_entries,
            );
            i = method_end;
        } else {
            i += 1;
        }
    }
}

fn extract_method_call(chars: &[char], dot_pos: usize) -> Option<(String, usize, usize)> {
    let method_start = dot_pos + 1;
    let mut method_end = method_start;
    while method_end < chars.len() && (chars[method_end].is_alphanumeric() || chars[method_end] == '_') {
        method_end += 1;
    }
    if method_end < chars.len() && chars[method_end] == '(' {
        let name: String = chars[method_start..method_end].iter().collect();
        Some((name, method_start, method_end))
    } else {
        None
    }
}

fn handle_method_check(
    ctx: &mut CheckCtx<'_>,
    method_name: &str,
    method_start: usize,
    method_end: usize,
    type_result: crate::type_inference::MethodCheckResult,
    all_methods: &[String],
    all_entries: &[&crate::generate::ReferenceEntry],
) {
    match type_result {
        crate::type_inference::MethodCheckResult::Valid => {
            check_method_arg_count(ctx, method_name, method_end, all_entries);
        }
        crate::type_inference::MethodCheckResult::Invalid { suggestion } => {
            emit_unknown_method(ctx, method_name, method_start, suggestion.map(|s| format!("did you mean '{}'?", s)));
        }
        crate::type_inference::MethodCheckResult::Unknown => {
            handle_global_method_check(ctx, method_name, method_start, method_end, all_methods, all_entries);
        }
    }
}

fn handle_global_method_check(
    ctx: &mut CheckCtx<'_>,
    method_name: &str,
    method_start: usize,
    method_end: usize,
    all_methods: &[String],
    all_entries: &[&crate::generate::ReferenceEntry],
) {
    if all_methods.is_empty() {
        return;
    }
    if is_exact_method_match(method_name, all_methods) {
        check_method_arg_count(ctx, method_name, method_end, all_entries);
        return;
    }
    let suggestion = find_best_method_suggestion(method_name, all_methods);
    let suggestion_text = suggestion.as_ref().map_or_else(
        || "no close match found — verify against docs".to_string(),
        |s| format!("did you mean '{}'? (similarity: {:.2})", s, strsim::jaro_winkler(method_name, s)),
    );
    emit_unknown_method(ctx, method_name, method_start, Some(suggestion_text));
}

fn emit_unknown_method(ctx: &mut CheckCtx<'_>, method_name: &str, method_start: usize, suggestion: Option<String>) {
    ctx.issues.push(Issue {
        severity: Severity::Error,
        message: format!("'{}' is not a known method", method_name),
        file: ctx.file_path.to_path_buf(),
        line: ctx.line_num,
        column: Some(method_start),
        code_snippet: ctx.line.to_string(),
        suggestion,
        rule: "unknown-method".to_string(),
    });
}

fn check_method_arg_count(
    ctx: &mut CheckCtx<'_>,
    method_name: &str,
    method_end: usize,
    all_entries: &[&crate::generate::ReferenceEntry],
) {
    let entry = all_entries.iter().find(|e| {
        e.name == method_name
            && (e.kind == EntryKind::Method || e.kind == EntryKind::Function)
            && (e.min_args.is_some() || e.max_args.is_some())
    });

    if let Some(entry) = entry {
        let call_expr = &ctx.line[method_end.saturating_sub(method_name.len())..];
        if let Some(issue) = crate::arg_checker::check_arg_count(call_expr, entry, ctx.line_num) {
            let msg = crate::arg_checker::format_arg_issue(&issue);
            ctx.issues.push(Issue {
                severity: Severity::Error,
                message: msg.clone(),
                file: ctx.file_path.to_path_buf(),
                line: ctx.line_num,
                column: None,
                code_snippet: ctx.line.to_string(),
                suggestion: Some(msg),
                rule: match &issue {
                    crate::arg_checker::ArgIssue::TooFewArgs { .. } => "too-few-args".to_string(),
                    crate::arg_checker::ArgIssue::TooManyArgs { .. } => "too-many-args".to_string(),
                },
            });
        }
    }
}

/// Extract the receiver variable name from characters before a `.` at position dot_pos.
fn extract_receiver(chars: &[char], dot_pos: usize) -> String {
    let end = dot_pos;
    let mut start = end;

    while start > 0 {
        let c = chars[start - 1];
        if c.is_alphanumeric() || c == '_' {
            start -= 1;
        } else {
            break;
        }
    }

    if start < end {
        chars[start..end].iter().collect()
    } else {
        String::new()
    }
}

/// Check if a method name exactly matches any known method.
pub fn is_exact_method_match(method: &str, all_methods: &[String]) -> bool {
    debug_assert!(!method.is_empty(), "method must be non-empty");
    all_methods.iter().any(|m| m == method)
}

/// Find the best fuzzy suggestion for a method name.
pub fn find_best_method_suggestion(
    method: &str,
    all_methods: &[String],
) -> Option<String> {
    debug_assert!(!method.is_empty(), "method must be non-empty");

    let mut best_score = 0.0f64;
    let mut best_name: Option<String> = None;

    for candidate in all_methods {
        let score = strsim::jaro_winkler(method, candidate);
        if score > best_score {
            best_score = score;
            best_name = Some(candidate.clone());
        }
    }

    if best_score >= 0.35 {
        debug_assert!(
            best_name.as_ref().is_some_and(|n| all_methods.contains(n)),
            "suggestion must be in all_methods"
        );
        best_name
    } else {
        None
    }
}

/// Collect all method and associated function names from all reference files.
pub fn collect_all_methods(refs: &[&ReferenceFile]) -> Vec<String> {
    let mut methods: Vec<String> = refs
        .iter()
        .flat_map(|rf| rf.entries.iter())
        .filter(|e| {
            e.kind == EntryKind::Method
                || e.kind == EntryKind::AssociatedFn
                || e.kind == EntryKind::Function
        })
        .map(|e| e.name.clone())
        .collect();

    methods.sort();
    methods.dedup();

    debug_assert!(
        methods.iter().all(|m| !m.is_empty()),
        "no empty method names"
    );

    methods
}
