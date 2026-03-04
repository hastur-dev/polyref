use regex::Regex;
use serde::Serialize;
use std::sync::LazyLock;

use crate::check::Issue;

/// Output format for enforcement results.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputFormat {
    Human,
    Json,
}

/// Configuration for the enforcement system.
#[derive(Debug, Clone)]
pub struct EnforceConfig {
    /// When true, issues cause a non-zero exit code.
    pub hard_block: bool,
    /// When true, uncovered packages are treated as blocking.
    pub strict_unknown_packages: bool,
    /// Minimum coverage percentage required (1..=100).
    pub require_coverage: Option<u8>,
    /// Read source from stdin instead of files.
    pub from_stdin: bool,
    /// Output format for results.
    pub output_format: OutputFormat,
}

impl Default for EnforceConfig {
    fn default() -> Self {
        Self {
            hard_block: false,
            strict_unknown_packages: false,
            require_coverage: None,
            from_stdin: false,
            output_format: OutputFormat::Human,
        }
    }
}

impl EnforceConfig {
    /// Validate configuration values.
    pub fn validate(&self) -> Result<(), String> {
        if let Some(pct) = self.require_coverage {
            if pct == 0 || pct > 100 {
                return Err(format!(
                    "require_coverage must be between 1 and 100, got {}",
                    pct
                ));
            }
        }
        Ok(())
    }
}

/// The enforcement verdict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum EnforceVerdict {
    Approved,
    Blocked,
}

/// A serialized issue for JSON output.
#[derive(Debug, Clone, Serialize)]
pub struct SerializedIssue {
    pub kind: String,
    pub line: usize,
    pub message: String,
    pub suggestion: Option<String>,
    pub similarity: Option<f64>,
}

/// The full enforcement result.
#[derive(Debug, Clone, Serialize)]
pub struct EnforceResult {
    pub polyref_enforce: bool,
    pub verdict: EnforceVerdict,
    pub issue_count: usize,
    pub issues: Vec<SerializedIssue>,
    pub coverage_pct: Option<f64>,
    pub instruction: Option<String>,
}

static SIMILARITY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"similarity:\s*([\d.]+)%?").expect("valid regex")
});

/// Convert an `Issue` into a `SerializedIssue`.
pub fn serialize_issue(issue: &Issue) -> SerializedIssue {
    let similarity = SIMILARITY_RE
        .captures(&issue.message)
        .and_then(|cap| cap[1].parse::<f64>().ok())
        .map(|v| if v > 1.0 { v / 100.0 } else { v });

    SerializedIssue {
        kind: issue.rule.clone(),
        line: issue.line,
        message: issue.message.clone(),
        suggestion: issue.suggestion.clone(),
        similarity,
    }
}

/// Build an enforcement result from a list of issues and config.
pub fn build_enforce_result(
    issues: &[Issue],
    config: &EnforceConfig,
) -> EnforceResult {
    let serialized: Vec<SerializedIssue> =
        issues.iter().map(serialize_issue).collect();
    let issue_count = serialized.len();

    let verdict = if config.hard_block && issue_count > 0 {
        EnforceVerdict::Blocked
    } else {
        EnforceVerdict::Approved
    };

    let instruction = build_instruction(&serialized);

    EnforceResult {
        polyref_enforce: true,
        verdict,
        issue_count,
        issues: serialized,
        coverage_pct: None,
        instruction,
    }
}

/// Build a Haiku-style regeneration instruction listing each issue.
pub fn build_instruction(issues: &[SerializedIssue]) -> Option<String> {
    if issues.is_empty() {
        return None;
    }

    let mut lines = Vec::with_capacity(issues.len() + 1);
    lines.push("Fix the following issues:".to_string());
    for issue in issues {
        lines.push(format!("  - Line {}: {}", issue.line, issue.message));
    }
    Some(lines.join("\n"))
}

/// Format an `EnforceResult` according to the given output format.
pub fn format_enforce_result(
    result: &EnforceResult,
    format: &OutputFormat,
) -> String {
    match format {
        OutputFormat::Json => {
            serde_json::to_string_pretty(result)
                .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
        }
        OutputFormat::Human => format_human(result),
    }
}

fn format_human(result: &EnforceResult) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "Verdict: {:?} ({} issue(s))\n",
        result.verdict, result.issue_count
    ));
    if let Some(pct) = result.coverage_pct {
        out.push_str(&format!("Coverage: {:.1}%\n", pct));
    }
    for issue in &result.issues {
        out.push_str(&format!(
            "  [{}] Line {}: {}\n",
            issue.kind, issue.line, issue.message
        ));
        if let Some(ref sug) = issue.suggestion {
            out.push_str(&format!("    Suggestion: {}\n", sug));
        }
    }
    if let Some(ref instr) = result.instruction {
        out.push_str(&format!("\n{}\n", instr));
    }
    out
}
