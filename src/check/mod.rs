pub mod common;
pub mod python;
pub mod rust;
pub mod typescript;

use crate::detect::Language;
use std::path::PathBuf;

/// Severity of a validation issue
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum Severity {
    Info,
    Warning,
    Error,
}

/// A single validation issue found in source code
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct Issue {
    pub severity: Severity,
    pub message: String,
    pub file: PathBuf,
    pub line: usize,
    pub column: Option<usize>,
    pub code_snippet: String,
    pub suggestion: Option<String>,
    pub rule: String,
}

/// Result of validating source files
#[derive(Debug, Clone, serde::Serialize)]
pub struct ValidationResult {
    pub language: Language,
    pub files_checked: usize,
    pub issues: Vec<Issue>,
}

impl ValidationResult {
    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Error)
            .count()
    }
    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|i| i.severity == Severity::Warning)
            .count()
    }
    pub fn is_clean(&self) -> bool {
        self.error_count() == 0
    }
}

/// Trait that all language checkers implement
pub trait Checker {
    fn check(
        &self,
        source_files: &[PathBuf],
        reference_files: &[crate::generate::ReferenceFile],
    ) -> anyhow::Result<ValidationResult>;
    fn language(&self) -> Language;
}
