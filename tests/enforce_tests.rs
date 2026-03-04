use polyref::check::{Issue, Severity};
use polyref::enforce::{
    build_enforce_result, build_instruction, format_enforce_result,
    serialize_issue, EnforceConfig, EnforceVerdict, OutputFormat,
};
use std::path::PathBuf;

fn make_issue(rule: &str, line: usize, message: &str) -> Issue {
    Issue {
        severity: Severity::Error,
        message: message.to_string(),
        file: PathBuf::from("test.rs"),
        line,
        column: None,
        code_snippet: String::new(),
        suggestion: Some("use the correct API".to_string()),
        rule: rule.to_string(),
    }
}

#[test]
fn test_enforce_config_default_values() {
    let config = EnforceConfig::default();
    assert!(!config.hard_block, "hard_block should default to false");
    assert!(
        config.require_coverage.is_none(),
        "require_coverage should default to None"
    );
    assert!(!config.strict_unknown_packages);
    assert!(!config.from_stdin);
    assert_eq!(config.output_format, OutputFormat::Human);
}

#[test]
fn test_enforce_config_validate_coverage_range() {
    let mut config = EnforceConfig::default();

    config.require_coverage = Some(0);
    assert!(config.validate().is_err(), "0 should be rejected");
    assert!(config.validate().unwrap_err().contains("between 1 and 100"));

    config.require_coverage = Some(101);
    assert!(config.validate().is_err(), "101 should be rejected");
    assert!(config.validate().unwrap_err().contains("between 1 and 100"));

    config.require_coverage = Some(50);
    assert!(config.validate().is_ok(), "50 should be accepted");
    assert_eq!(config.require_coverage, Some(50));

    config.require_coverage = Some(1);
    assert!(config.validate().is_ok(), "1 should be accepted");

    config.require_coverage = Some(100);
    assert!(config.validate().is_ok(), "100 should be accepted");
}

#[test]
fn test_build_enforce_result_no_issues_approved() {
    let config = EnforceConfig {
        hard_block: true,
        ..EnforceConfig::default()
    };
    let result = build_enforce_result(&[], &config);
    assert_eq!(result.verdict, EnforceVerdict::Approved);
    assert_eq!(result.issue_count, 0);
    assert!(result.issues.is_empty());
    assert!(result.instruction.is_none());
}

#[test]
fn test_build_enforce_result_issues_blocked() {
    let config = EnforceConfig {
        hard_block: true,
        ..EnforceConfig::default()
    };
    let issues = vec![make_issue("unknown_method", 10, "method not found")];
    let result = build_enforce_result(&issues, &config);
    assert_eq!(result.verdict, EnforceVerdict::Blocked);
    assert_eq!(result.issue_count, 1);
    assert!(result.instruction.is_some());
    assert!(result.instruction.as_ref().unwrap().contains("Line 10"));
}

#[test]
fn test_build_enforce_result_issues_not_blocking_when_disabled() {
    let config = EnforceConfig {
        hard_block: false,
        ..EnforceConfig::default()
    };
    let issues = vec![make_issue("unknown_method", 5, "method not found")];
    let result = build_enforce_result(&issues, &config);
    assert_eq!(result.verdict, EnforceVerdict::Approved);
    assert_eq!(result.issue_count, 1);
}

#[test]
fn test_serialized_issue_has_correct_fields() {
    let issue = make_issue("hallucinated_api", 42, "no such method (similarity: 85%)");
    let serialized = serialize_issue(&issue);
    assert_eq!(serialized.kind, "hallucinated_api");
    assert!(serialized.line >= 1);
    assert_eq!(serialized.line, 42);
    assert!(serialized.message.contains("no such method"));
    assert!(serialized.suggestion.is_some());
}

#[test]
fn test_format_enforce_result_json_roundtrip() {
    let config = EnforceConfig {
        hard_block: true,
        ..EnforceConfig::default()
    };
    let issues = vec![make_issue("test_rule", 1, "test message")];
    let result = build_enforce_result(&issues, &config);
    let json_str = format_enforce_result(&result, &OutputFormat::Json);

    let parsed: serde_json::Value =
        serde_json::from_str(&json_str).expect("should be valid JSON");
    assert_eq!(parsed["verdict"], "Blocked");
    assert_eq!(parsed["polyref_enforce"], true);
    assert_eq!(parsed["issue_count"], 1);
}

#[test]
fn test_format_enforce_result_human_nonempty() {
    let config = EnforceConfig::default();
    let issues = vec![make_issue("test", 1, "msg")];
    let result = build_enforce_result(&issues, &config);
    let output = format_enforce_result(&result, &OutputFormat::Human);
    assert!(!output.is_empty());
    assert!(output.contains("1 issue"));
    assert!(output.contains("Verdict"));
}

#[test]
fn test_instruction_lists_all_issues() {
    let issues = vec![
        make_issue("a", 10, "first problem"),
        make_issue("b", 20, "second problem"),
        make_issue("c", 30, "third problem"),
    ];
    let serialized: Vec<_> = issues.iter().map(serialize_issue).collect();
    let instruction = build_instruction(&serialized);
    assert!(instruction.is_some());
    let text = instruction.unwrap();
    assert!(text.contains("Line 10"), "should list line 10");
    assert!(text.contains("Line 20"), "should list line 20");
    assert!(text.contains("Line 30"), "should list line 30");
    assert!(text.contains("first problem"));
    assert!(text.contains("second problem"));
    assert!(text.contains("third problem"));
}
