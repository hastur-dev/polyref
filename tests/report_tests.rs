use polyref::check::{Issue, Severity, ValidationResult};
use polyref::detect::Language;
use polyref::report::Reporter;
use std::path::PathBuf;

fn make_issue(severity: Severity, msg: &str, rule: &str, suggestion: Option<&str>) -> Issue {
    Issue {
        severity,
        message: msg.to_string(),
        file: PathBuf::from("test.py"),
        line: 1,
        column: Some(0),
        code_snippet: "test code".to_string(),
        suggestion: suggestion.map(|s| s.to_string()),
        rule: rule.to_string(),
    }
}

// ============================================================================
// Phase 7.1 — Terminal Reporter
// ============================================================================

#[test]
fn test_terminal_report_with_errors() {
    let reporter = polyref::report::terminal::TerminalReporter;
    let results = vec![ValidationResult {
        language: Language::Python,
        files_checked: 2,
        issues: vec![make_issue(
            Severity::Error,
            "'NonExistent' is not exported",
            "unknown-import",
            None,
        )],
    }];

    let output = reporter.report(&results).unwrap();
    assert!(output.contains("test.py:1"));
    assert!(output.contains("unknown-import"));
    assert!(output.contains("not exported"));
}

#[test]
fn test_terminal_report_with_warnings() {
    let reporter = polyref::report::terminal::TerminalReporter;
    let results = vec![ValidationResult {
        language: Language::Rust,
        files_checked: 1,
        issues: vec![make_issue(
            Severity::Warning,
            "deprecated usage",
            "deprecated-usage",
            None,
        )],
    }];

    let output = reporter.report(&results).unwrap();
    assert!(output.contains("warning"));
    assert!(output.contains("deprecated"));
}

#[test]
fn test_terminal_report_with_suggestions() {
    let reporter = polyref::report::terminal::TerminalReporter;
    let results = vec![ValidationResult {
        language: Language::Python,
        files_checked: 1,
        issues: vec![make_issue(
            Severity::Error,
            "unknown function",
            "unknown-function",
            Some("did you mean 'get'?"),
        )],
    }];

    let output = reporter.report(&results).unwrap();
    assert!(output.contains("suggestion"));
    assert!(output.contains("did you mean"));
}

#[test]
fn test_terminal_report_empty() {
    let reporter = polyref::report::terminal::TerminalReporter;
    let results = vec![ValidationResult {
        language: Language::Rust,
        files_checked: 5,
        issues: vec![],
    }];

    let output = reporter.report(&results).unwrap();
    assert!(output.contains("0 error"));
    assert!(output.contains("0 warning"));
}

#[test]
fn test_terminal_report_summary_counts() {
    let reporter = polyref::report::terminal::TerminalReporter;
    let results = vec![
        ValidationResult {
            language: Language::Python,
            files_checked: 3,
            issues: vec![
                make_issue(Severity::Error, "e1", "r1", None),
                make_issue(Severity::Warning, "w1", "r2", None),
            ],
        },
        ValidationResult {
            language: Language::Rust,
            files_checked: 5,
            issues: vec![],
        },
    ];

    let output = reporter.report(&results).unwrap();
    assert!(output.contains("Python"));
    assert!(output.contains("1 error"));
    assert!(output.contains("1 warning"));
    assert!(output.contains("Rust"));
    assert!(output.contains("0 error"));
}

// ============================================================================
// Phase 7.2 — JSON Reporter
// ============================================================================

#[test]
fn test_json_report_structure() {
    let reporter = polyref::report::json::JsonReporter;
    let results = vec![ValidationResult {
        language: Language::Python,
        files_checked: 2,
        issues: vec![make_issue(Severity::Error, "test error", "test-rule", None)],
    }];

    let output = reporter.report(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert!(parsed.get("results").is_some());
    assert!(parsed.get("summary").is_some());
    let summary = parsed.get("summary").unwrap();
    assert!(summary.get("total_errors").is_some());
    assert!(summary.get("total_warnings").is_some());
    assert!(summary.get("total_files").is_some());
    assert!(summary.get("is_clean").is_some());
}

#[test]
fn test_json_report_is_clean_true() {
    let reporter = polyref::report::json::JsonReporter;
    let results = vec![ValidationResult {
        language: Language::Rust,
        files_checked: 5,
        issues: vec![],
    }];

    let output = reporter.report(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["summary"]["is_clean"], true);
}

#[test]
fn test_json_report_is_clean_false() {
    let reporter = polyref::report::json::JsonReporter;
    let results = vec![ValidationResult {
        language: Language::Python,
        files_checked: 1,
        issues: vec![make_issue(Severity::Error, "err", "rule", None)],
    }];

    let output = reporter.report(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    assert_eq!(parsed["summary"]["is_clean"], false);
}

#[test]
fn test_json_report_multiple_languages() {
    let reporter = polyref::report::json::JsonReporter;
    let results = vec![
        ValidationResult {
            language: Language::Rust,
            files_checked: 3,
            issues: vec![],
        },
        ValidationResult {
            language: Language::Python,
            files_checked: 2,
            issues: vec![make_issue(Severity::Error, "err", "rule", None)],
        },
        ValidationResult {
            language: Language::TypeScript,
            files_checked: 1,
            issues: vec![],
        },
    ];

    let output = reporter.report(&results).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
    let results_arr = parsed["results"].as_array().unwrap();
    assert_eq!(results_arr.len(), 3);
    assert_eq!(parsed["summary"]["total_files"], 6);
}
