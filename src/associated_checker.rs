use regex::Regex;
use std::sync::LazyLock;

use crate::generate::{EntryKind, ReferenceEntry};

/// An associated function call like `Runtime::new()`
#[derive(Debug, Clone, PartialEq)]
pub struct AssociatedCall {
    pub type_name: String,
    pub fn_name: String,
    pub full_call: String,
    pub line_number: usize,
}

/// Issue found when checking associated function calls
#[derive(Debug, Clone, PartialEq)]
pub struct AssociatedIssue {
    pub type_name: String,
    pub fn_name: String,
    pub suggestion: Option<String>,
    pub confidence: f64,
    pub line_number: usize,
}

static ASSOC_CALL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([A-Z][a-zA-Z0-9_]*)::\s*([a-z_][a-zA-Z0-9_]*)\s*\(").expect("valid regex")
});

// Match crate::function( patterns (lowercase crate name)
static CRATE_CALL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([a-z_][a-z0-9_]*)::\s*([a-z_][a-zA-Z0-9_]*)\s*\(").expect("valid regex")
});

// Match Type::Variant patterns (both parts uppercase — enum variant usage)
static ENUM_VARIANT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"([A-Z][a-zA-Z0-9_]*)::\s*([A-Z][a-zA-Z0-9_]*)").expect("valid regex")
});

/// Extract all `Type::method(` patterns from a single line.
///
/// Skips comment lines and `use` statements.
pub fn extract_associated_calls(
    line: &str,
    line_number: usize,
) -> Vec<AssociatedCall> {
    let trimmed = line.trim();
    if trimmed.starts_with("//") || trimmed.starts_with("use ") {
        return Vec::new();
    }

    let mut calls = Vec::new();
    for cap in ASSOC_CALL_RE.captures_iter(line) {
        let type_name = cap[1].to_string();
        let fn_name = cap[2].to_string();

        debug_assert!(
            type_name.starts_with(|c: char| c.is_ascii_uppercase()),
            "type_name must start with uppercase"
        );
        debug_assert!(
            !fn_name.contains(':'),
            "fn_name must not contain colons"
        );

        let full_call = format!("{}::{}", type_name, fn_name);
        calls.push(AssociatedCall {
            type_name,
            fn_name,
            full_call,
            line_number,
        });
    }

    calls
}

/// Check associated calls against known reference entries.
///
/// Returns issues for calls not found in the reference set.
pub fn check_associated_calls(
    calls: &[AssociatedCall],
    refs: &[ReferenceEntry],
) -> Vec<AssociatedIssue> {
    if calls.is_empty() {
        return Vec::new();
    }

    let mut issues = Vec::new();

    for call in calls {
        // Look for exact match: AssociatedFn with matching type_context and name
        let exact = refs.iter().any(|r| {
            r.kind == EntryKind::AssociatedFn
                && r.type_context.as_deref() == Some(&call.type_name)
                && r.name == call.fn_name
        });

        if exact {
            continue;
        }

        // Also check Function entries with matching type_context (some parsers use Function)
        let exact_fn = refs.iter().any(|r| {
            r.kind == EntryKind::Function
                && r.type_context.as_deref() == Some(&call.type_name)
                && r.name == call.fn_name
        });

        if exact_fn {
            continue;
        }

        // Find best fuzzy suggestion among entries for this type
        let type_methods: Vec<&str> = refs
            .iter()
            .filter(|r| {
                (r.kind == EntryKind::AssociatedFn || r.kind == EntryKind::Function)
                    && r.type_context.as_deref() == Some(&call.type_name)
            })
            .map(|r| r.name.as_str())
            .collect();

        let (suggestion, confidence) =
            find_best_suggestion(&call.fn_name, &type_methods);

        debug_assert!(call.line_number > 0, "line_number must be > 0");

        issues.push(AssociatedIssue {
            type_name: call.type_name.clone(),
            fn_name: call.fn_name.clone(),
            suggestion,
            confidence,
            line_number: call.line_number,
        });
    }

    issues
}

fn find_best_suggestion(
    target: &str,
    candidates: &[&str],
) -> (Option<String>, f64) {
    if candidates.is_empty() {
        return (None, 0.0);
    }

    let mut best_name: Option<String> = None;
    let mut best_score: f64 = 0.0;

    for &candidate in candidates {
        let score = strsim::jaro_winkler(target, candidate);
        if score > best_score {
            best_score = score;
            best_name = Some(candidate.to_string());
        }
    }

    if best_score >= 0.35 {
        (best_name, best_score)
    } else {
        (None, best_score)
    }
}

/// Extract crate-level function calls like `tokio::spawn(`.
///
/// Only matches when the crate name starts with a lowercase letter.
pub fn extract_crate_calls(
    line: &str,
    line_number: usize,
) -> Vec<AssociatedCall> {
    let trimmed = line.trim();
    if trimmed.starts_with("//") || trimmed.starts_with("use ") {
        return Vec::new();
    }

    let mut calls = Vec::new();
    for cap in CRATE_CALL_RE.captures_iter(line) {
        let crate_name = cap[1].to_string();
        let fn_name = cap[2].to_string();

        // Skip common keywords that aren't crate names
        if ["let", "mut", "if", "else", "for", "while", "match", "fn", "pub", "mod", "impl", "struct", "enum", "trait", "type", "const", "static", "async", "await", "return", "self", "super", "crate"].contains(&crate_name.as_str()) {
            continue;
        }

        let full_call = format!("{}::{}", crate_name, fn_name);
        calls.push(AssociatedCall {
            type_name: crate_name,
            fn_name,
            full_call,
            line_number,
        });
    }

    calls
}

/// Check crate-level function calls against reference files.
///
/// For example, `tokio::spawn()` checks if `spawn` exists in the tokio reference.
pub fn check_crate_calls(
    calls: &[AssociatedCall],
    refs: &[&crate::generate::ReferenceFile],
) -> Vec<AssociatedIssue> {
    if calls.is_empty() {
        return Vec::new();
    }

    let mut issues = Vec::new();

    for call in calls {
        // Find the reference file for this crate
        let ref_file = refs.iter().find(|rf| {
            rf.library_name == call.type_name
                || rf.library_name.replace('-', "_") == call.type_name
        });

        let ref_file = match ref_file {
            Some(rf) => rf,
            None => continue, // No ref for this crate
        };

        // Check if the function exists in the reference
        let exists = ref_file.entries.iter().any(|e| e.name == call.fn_name);
        if exists {
            continue;
        }

        // Fuzzy suggestion
        let fn_names: Vec<&str> = ref_file
            .entries
            .iter()
            .filter(|e| {
                e.kind == crate::generate::EntryKind::Function
                    || e.kind == crate::generate::EntryKind::Method
                    || e.kind == crate::generate::EntryKind::AssociatedFn
            })
            .map(|e| e.name.as_str())
            .collect();

        let (suggestion, confidence) = find_best_suggestion(&call.fn_name, &fn_names);

        issues.push(AssociatedIssue {
            type_name: call.type_name.clone(),
            fn_name: call.fn_name.clone(),
            suggestion,
            confidence,
            line_number: call.line_number,
        });
    }

    issues
}

/// Extract enum variant usage patterns like `Color::Red`.
pub fn extract_enum_variant_calls(
    line: &str,
    line_number: usize,
) -> Vec<AssociatedCall> {
    let trimmed = line.trim();
    if trimmed.starts_with("//") || trimmed.starts_with("use ") {
        return Vec::new();
    }

    let mut calls = Vec::new();
    for cap in ENUM_VARIANT_RE.captures_iter(line) {
        let type_name = cap[1].to_string();
        let variant_name = cap[2].to_string();
        let full_call = format!("{}::{}", type_name, variant_name);

        calls.push(AssociatedCall {
            type_name,
            fn_name: variant_name,
            full_call,
            line_number,
        });
    }

    calls
}

/// Check enum variant usage against known variants in references.
pub fn check_enum_variant_calls(
    calls: &[AssociatedCall],
    refs: &[ReferenceEntry],
) -> Vec<AssociatedIssue> {
    if calls.is_empty() {
        return Vec::new();
    }

    let mut issues = Vec::new();

    for call in calls {
        // Check if the enum type exists in refs
        let enum_exists = refs.iter().any(|r| {
            r.kind == crate::generate::EntryKind::Enum && r.name == call.type_name
        });

        if !enum_exists {
            continue; // Unknown enum type — not our problem
        }

        // Check if the variant exists
        let variant_exists = refs.iter().any(|r| {
            r.kind == crate::generate::EntryKind::EnumVariant
                && r.parent.as_deref() == Some(&call.type_name)
                && r.name == call.fn_name
        });

        if variant_exists {
            continue;
        }

        // Find best variant suggestion
        let variants: Vec<&str> = refs
            .iter()
            .filter(|r| {
                r.kind == crate::generate::EntryKind::EnumVariant
                    && r.parent.as_deref() == Some(&call.type_name)
            })
            .map(|r| r.name.as_str())
            .collect();

        let (suggestion, confidence) = find_best_suggestion(&call.fn_name, &variants);

        issues.push(AssociatedIssue {
            type_name: call.type_name.clone(),
            fn_name: call.fn_name.clone(),
            suggestion,
            confidence,
            line_number: call.line_number,
        });
    }

    issues
}

/// Format an associated function issue as a human-readable string.
pub fn format_associated_issue(issue: &AssociatedIssue) -> String {
    let result = if let Some(ref suggestion) = issue.suggestion {
        format!(
            "unknown associated function '{}::{}' — did you mean '{}::{}' (similarity: {:.2})?",
            issue.type_name, issue.fn_name, issue.type_name, suggestion, issue.confidence
        )
    } else {
        format!(
            "unknown associated function '{}::{}' — check docs",
            issue.type_name, issue.fn_name
        )
    };

    debug_assert!(!result.is_empty(), "result must be non-empty");
    debug_assert!(
        result.contains(&issue.type_name),
        "result must contain the type name"
    );

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::ReferenceEntry;

    fn make_assoc_ref(type_name: &str, fn_name: &str) -> ReferenceEntry {
        ReferenceEntry {
            name: fn_name.to_string(), kind: EntryKind::AssociatedFn,
            type_context: Some(type_name.to_string()), ..Default::default()
        }
    }

    fn make_call(type_name: &str, fn_name: &str, line: usize) -> AssociatedCall {
        AssociatedCall {
            type_name: type_name.to_string(), fn_name: fn_name.to_string(),
            full_call: format!("{}::{}", type_name, fn_name), line_number: line,
        }
    }

    #[test]
    fn test_extract_single_associated_call() {
        let calls = extract_associated_calls("let rt = Runtime::new();", 1);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].type_name, "Runtime");
        assert_eq!(calls[0].fn_name, "new");
    }

    #[test]
    fn test_extract_multiple_associated_calls() {
        let calls = extract_associated_calls("let v = Vec::new(); let m = HashMap::with_capacity(10);", 1);
        assert_eq!(calls.len(), 2);
        assert!(calls.iter().all(|c| c.type_name.starts_with(|ch: char| ch.is_uppercase())));
    }

    #[test]
    fn test_extract_skips_comment_lines() {
        assert!(extract_associated_calls("// Runtime::new() is a constructor", 1).is_empty());
    }

    #[test]
    fn test_extract_skips_use_statements() {
        assert!(extract_associated_calls("use std::collections::HashMap;", 1).is_empty());
    }

    #[test]
    fn test_extract_ignores_trait_paths() {
        assert!(extract_associated_calls("use tokio::io::AsyncReadExt;", 1).is_empty());
    }

    #[test]
    fn test_check_known_associated_fn_no_issue() {
        let refs = vec![make_assoc_ref("Runtime", "new")];
        assert!(check_associated_calls(&[make_call("Runtime", "new", 1)], &refs).is_empty());
    }

    #[test]
    fn test_check_unknown_associated_fn_emits_issue() {
        let refs = vec![make_assoc_ref("Runtime", "new")];
        let issues = check_associated_calls(&[make_call("Runtime", "create", 5)], &refs);
        assert!(!issues.is_empty());
        assert_eq!(issues[0].type_name, "Runtime");
    }

    #[test]
    fn test_check_suggests_close_match() {
        let refs = vec![make_assoc_ref("JoinSet", "new")];
        let issues = check_associated_calls(&[make_call("JoinSet", "neu", 3)], &refs);
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].suggestion, Some("new".to_string()));
    }

    #[test]
    fn test_check_invented_constructor_flagged() {
        let refs = vec![make_assoc_ref("Runtime", "new")];
        let calls = extract_associated_calls("let rt = Runtime::new_async();", 7);
        let issues = check_associated_calls(&calls, &refs);
        assert!(!issues.is_empty());
        assert_eq!(issues[0].fn_name, "new_async");
    }

    #[test]
    fn test_format_issue_with_suggestion() {
        let issue = AssociatedIssue {
            type_name: "Runtime".into(), fn_name: "neu".into(),
            suggestion: Some("new".into()), confidence: 0.85, line_number: 1,
        };
        let output = format_associated_issue(&issue);
        assert!(output.contains("did you mean") && output.contains("Runtime"));
    }

    #[test]
    fn test_format_issue_no_suggestion() {
        let issue = AssociatedIssue {
            type_name: "Runtime".into(), fn_name: "xyzabc".into(),
            suggestion: None, confidence: 0.0, line_number: 1,
        };
        let output = format_associated_issue(&issue);
        assert!(output.contains("check docs") && output.contains("Runtime"));
    }

    #[test]
    fn test_extract_associated_call_fn_name_has_no_colon() {
        for call in &extract_associated_calls("Runtime::new()", 1) {
            assert!(!call.fn_name.contains(':'));
        }
    }
}
