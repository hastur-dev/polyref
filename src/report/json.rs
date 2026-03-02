use crate::check::ValidationResult;
use crate::report::Reporter;

pub struct JsonReporter;

#[derive(serde::Serialize)]
struct JsonReport {
    results: Vec<JsonLanguageResult>,
    summary: JsonSummary,
}

#[derive(serde::Serialize)]
struct JsonLanguageResult {
    language: String,
    files_checked: usize,
    issues: Vec<JsonIssue>,
    error_count: usize,
    warning_count: usize,
}

#[derive(serde::Serialize)]
struct JsonIssue {
    severity: String,
    rule: String,
    message: String,
    file: String,
    line: usize,
    column: Option<usize>,
    code_snippet: String,
    suggestion: Option<String>,
}

#[derive(serde::Serialize)]
struct JsonSummary {
    total_errors: usize,
    total_warnings: usize,
    total_files: usize,
    is_clean: bool,
}

impl Reporter for JsonReporter {
    fn report(&self, results: &[ValidationResult]) -> anyhow::Result<String> {
        let json_results: Vec<JsonLanguageResult> = results
            .iter()
            .map(|r| JsonLanguageResult {
                language: r.language.to_string(),
                files_checked: r.files_checked,
                issues: r
                    .issues
                    .iter()
                    .map(|i| JsonIssue {
                        severity: format!("{:?}", i.severity).to_lowercase(),
                        rule: i.rule.clone(),
                        message: i.message.clone(),
                        file: i.file.display().to_string(),
                        line: i.line,
                        column: i.column,
                        code_snippet: i.code_snippet.clone(),
                        suggestion: i.suggestion.clone(),
                    })
                    .collect(),
                error_count: r.error_count(),
                warning_count: r.warning_count(),
            })
            .collect();

        let total_errors: usize = results.iter().map(|r| r.error_count()).sum();
        let total_warnings: usize = results.iter().map(|r| r.warning_count()).sum();
        let total_files: usize = results.iter().map(|r| r.files_checked).sum();

        let report = JsonReport {
            results: json_results,
            summary: JsonSummary {
                total_errors,
                total_warnings,
                total_files,
                is_clean: total_errors == 0,
            },
        };

        Ok(serde_json::to_string_pretty(&report)?)
    }
}
