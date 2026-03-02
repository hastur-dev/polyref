pub mod json;
pub mod terminal;

use crate::check::ValidationResult;

/// Trait for output formatting
pub trait Reporter {
    fn report(&self, results: &[ValidationResult]) -> anyhow::Result<String>;
}
