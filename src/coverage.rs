use crate::enforce::EnforceConfig;
use crate::generate::ReferenceFile;
use crate::source_context::SourceContext;

/// Coverage analysis report for a source file against available references.
#[derive(Debug, Clone)]
pub struct CoverageReport {
    /// Total number of external API calls / imported packages.
    pub total_api_calls: usize,
    /// Number of imports covered by reference files.
    pub covered_calls: usize,
    /// Package names imported but not covered by any reference file.
    pub uncovered_packages: Vec<String>,
    /// Coverage percentage (0.0..=100.0).
    pub coverage_pct: f64,
    /// Suggestions for generating missing reference files.
    pub missing_ref_suggestions: Vec<String>,
}

/// Compute coverage of imported crates against available reference files.
pub fn compute_coverage(
    ctx: &SourceContext,
    available_refs: &[ReferenceFile],
) -> CoverageReport {
    let ref_names: Vec<String> = available_refs
        .iter()
        .flat_map(|rf| {
            let canonical = rf.library_name.replace('-', "_");
            vec![rf.library_name.clone(), canonical]
        })
        .collect();

    // Filter out std/core/alloc which don't need external refs
    let external_crates: Vec<&String> = ctx
        .imported_crates
        .iter()
        .filter(|c| !is_builtin_crate(c))
        .collect();

    let total = external_crates.len();
    let mut covered = 0usize;
    let mut uncovered = Vec::new();

    for crate_name in &external_crates {
        let normalized = crate_name.replace('-', "_");
        if ref_names.contains(crate_name)
            || ref_names.contains(&normalized)
        {
            covered += 1;
        } else {
            uncovered.push(crate_name.to_string());
        }
    }

    let coverage_pct = if total == 0 {
        100.0
    } else {
        ((covered as f64) / (total as f64) * 100.0).clamp(0.0, 100.0)
    };

    let missing_ref_suggestions = uncovered
        .iter()
        .map(|pkg| {
            format!(
                "polyref generate --project . (ensure {} is in dependencies)",
                pkg
            )
        })
        .collect();

    CoverageReport {
        total_api_calls: total,
        covered_calls: covered,
        uncovered_packages: uncovered,
        coverage_pct,
        missing_ref_suggestions,
    }
}

/// Check if coverage meets enforcement requirements.
/// Returns `Some(reason)` if the gate should block.
pub fn check_coverage_gate(
    report: &CoverageReport,
    config: &EnforceConfig,
) -> Option<String> {
    if config.strict_unknown_packages && !report.uncovered_packages.is_empty() {
        return Some(format!(
            "Strict mode: {} uncovered package(s): {}",
            report.uncovered_packages.len(),
            report.uncovered_packages.join(", ")
        ));
    }

    if let Some(threshold) = config.require_coverage {
        let threshold_f64 = f64::from(threshold);
        if report.coverage_pct < threshold_f64 {
            return Some(format!(
                "Coverage {:.1}% is below required {}%",
                report.coverage_pct, threshold
            ));
        }
    }

    None
}

/// Format a coverage report as a human-readable string.
pub fn format_coverage_report(report: &CoverageReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "Coverage: {:.1}% ({}/{} packages covered)\n",
        report.coverage_pct, report.covered_calls, report.total_api_calls
    ));

    if !report.uncovered_packages.is_empty() {
        out.push_str("Uncovered packages:\n");
        for pkg in &report.uncovered_packages {
            out.push_str(&format!("  - {}\n", pkg));
        }
    }

    if !report.missing_ref_suggestions.is_empty() {
        out.push_str("Suggestions:\n");
        for sug in &report.missing_ref_suggestions {
            out.push_str(&format!("  {}\n", sug));
        }
    }

    out
}

fn is_builtin_crate(name: &str) -> bool {
    matches!(name, "std" | "core" | "alloc" | "self" | "super" | "crate")
}
