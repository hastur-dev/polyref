use crate::check::{Severity, ValidationResult};
use crate::report::Reporter;

pub struct TerminalReporter;

impl Reporter for TerminalReporter {
    fn report(&self, results: &[ValidationResult]) -> anyhow::Result<String> {
        let mut output = String::new();

        for result in results {
            if result.issues.is_empty() && result.files_checked == 0 {
                continue;
            }

            output.push_str(&format!(
                "\n── {} Validation ──────────────────────────────\n",
                result.language
            ));

            for issue in &result.issues {
                let icon = match issue.severity {
                    Severity::Error => "✗",
                    Severity::Warning => "⚠",
                    Severity::Info => "ℹ",
                };

                let severity_str = match issue.severity {
                    Severity::Error => "error",
                    Severity::Warning => "warning",
                    Severity::Info => "info",
                };

                output.push_str(&format!(
                    "{} {}:{} {}[{}]\n",
                    icon,
                    issue.file.display(),
                    issue.line,
                    severity_str,
                    issue.rule,
                ));

                output.push_str(&format!("  {}\n", issue.code_snippet.trim()));
                output.push_str(&format!("  {}\n", issue.message));

                if let Some(suggestion) = &issue.suggestion {
                    output.push_str(&format!("  suggestion: {}\n", suggestion));
                }

                output.push('\n');
            }
        }

        // Summary
        output.push_str("── Summary ─────────────────────────────────────\n");

        for result in results {
            if result.files_checked == 0 {
                continue;
            }
            output.push_str(&format!(
                "{}: {} error(s), {} warning(s) ({} file(s) checked)\n",
                result.language,
                result.error_count(),
                result.warning_count(),
                result.files_checked,
            ));
        }

        let total_errors: usize = results.iter().map(|r| r.error_count()).sum();
        let total_warnings: usize = results.iter().map(|r| r.warning_count()).sum();
        let total_files: usize = results.iter().map(|r| r.files_checked).sum();

        if total_errors == 0 && total_warnings == 0 {
            output.push_str(&format!("All clean! ({} file(s) checked)\n", total_files));
        }

        Ok(output)
    }
}
