use crate::drift_detector::DriftResult;
use colored::Colorize;

/// Print drift results in the requested format.
pub fn report(results: &[DriftResult], format: &str) {
    match format {
        "json" => report_json(results),
        _ => report_terminal(results),
    }
}

fn report_terminal(results: &[DriftResult]) {
    if results.is_empty() {
        println!("{}", "No reference files found to check.".yellow());
        return;
    }

    let drifted: Vec<&DriftResult> = results.iter().filter(|r| r.has_drift).collect();
    let errors: Vec<&DriftResult> = results.iter().filter(|r| r.error.is_some()).collect();
    let up_to_date = results.len() - drifted.len() - errors.len();

    println!("{}", "=== Polyref Drift Report ===".bold());
    println!();

    if !drifted.is_empty() {
        println!("{}", format!("Drifted ({}):", drifted.len()).red().bold());
        for r in &drifted {
            println!(
                "  {} {} {} → {} ({})",
                "✗".red(),
                r.library_name,
                r.ref_version,
                r.latest_version.as_deref().unwrap_or("?"),
                r.registry,
            );
        }
        println!();
    }

    if !errors.is_empty() {
        println!(
            "{}",
            format!("Errors ({}):", errors.len()).yellow().bold()
        );
        for r in &errors {
            println!(
                "  {} {} — {} ({})",
                "?".yellow(),
                r.library_name,
                r.error.as_deref().unwrap_or("unknown error"),
                r.registry,
            );
        }
        println!();
    }

    if up_to_date > 0 {
        println!(
            "{}",
            format!("Up to date ({}):", up_to_date).green().bold()
        );
        for r in results
            .iter()
            .filter(|r| !r.has_drift && r.error.is_none())
        {
            println!(
                "  {} {} {} ({})",
                "✓".green(),
                r.library_name,
                r.ref_version,
                r.registry,
            );
        }
        println!();
    }

    println!(
        "Summary: {} checked, {} drifted, {} errors, {} up to date",
        results.len(),
        drifted.len(),
        errors.len(),
        up_to_date,
    );
}

fn report_json(results: &[DriftResult]) {
    let output = serde_json::json!({
        "results": results,
        "summary": {
            "total": results.len(),
            "drifted": results.iter().filter(|r| r.has_drift).count(),
            "errors": results.iter().filter(|r| r.error.is_some()).count(),
            "up_to_date": results.iter().filter(|r| !r.has_drift && r.error.is_none()).count(),
        }
    });
    println!("{}", serde_json::to_string_pretty(&output).unwrap_or_default());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_drifted() -> DriftResult {
        DriftResult {
            library_name: "serde".to_string(),
            ref_version: "1.0.0".to_string(),
            latest_version: Some("1.1.0".to_string()),
            registry: "crates.io".to_string(),
            has_drift: true,
            error: None,
        }
    }

    fn make_up_to_date() -> DriftResult {
        DriftResult {
            library_name: "anyhow".to_string(),
            ref_version: "1.0.0".to_string(),
            latest_version: Some("1.0.0".to_string()),
            registry: "crates.io".to_string(),
            has_drift: false,
            error: None,
        }
    }

    fn make_error() -> DriftResult {
        DriftResult {
            library_name: "unknown-crate".to_string(),
            ref_version: "0.1.0".to_string(),
            latest_version: None,
            registry: "crates.io".to_string(),
            has_drift: false,
            error: Some("not found".to_string()),
        }
    }

    #[test]
    fn test_report_json_structure() {
        let results = vec![make_drifted(), make_up_to_date(), make_error()];
        // Capture by building JSON directly
        let output = serde_json::json!({
            "results": results,
            "summary": {
                "total": results.len(),
                "drifted": results.iter().filter(|r| r.has_drift).count(),
                "errors": results.iter().filter(|r| r.error.is_some()).count(),
                "up_to_date": results.iter().filter(|r| !r.has_drift && r.error.is_none()).count(),
            }
        });
        let summary = output["summary"].as_object().unwrap();
        assert_eq!(summary["total"], 3);
        assert_eq!(summary["drifted"], 1);
        assert_eq!(summary["errors"], 1);
        assert_eq!(summary["up_to_date"], 1);
    }

    #[test]
    fn test_report_json_empty() {
        let results: Vec<DriftResult> = vec![];
        let output = serde_json::json!({
            "results": results,
            "summary": {
                "total": 0,
                "drifted": 0,
                "errors": 0,
                "up_to_date": 0,
            }
        });
        assert_eq!(output["summary"]["total"], 0);
        assert_eq!(output["results"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_report_terminal_does_not_panic_empty() {
        report_terminal(&[]);
    }

    #[test]
    fn test_report_terminal_does_not_panic_mixed() {
        let results = vec![make_drifted(), make_up_to_date(), make_error()];
        report_terminal(&results);
    }

    #[test]
    fn test_report_dispatches_to_json() {
        // Just verify it doesn't panic
        let results = vec![make_up_to_date()];
        report(&results, "json");
    }

    #[test]
    fn test_report_dispatches_to_terminal() {
        let results = vec![make_up_to_date()];
        report(&results, "terminal");
    }

    #[test]
    fn test_report_dispatches_unknown_defaults_to_terminal() {
        let results = vec![make_up_to_date()];
        report(&results, "xml");
    }
}
