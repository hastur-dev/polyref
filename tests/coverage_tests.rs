use polyref::coverage::{
    check_coverage_gate, compute_coverage, format_coverage_report,
};
use polyref::detect::Language;
use polyref::enforce::EnforceConfig;
use polyref::generate::ReferenceFile;
use polyref::source_context::SourceContext;
use polyref::type_inference::TypeContext;
use std::collections::HashMap;
use std::path::PathBuf;

fn make_ctx(crates: &[&str]) -> SourceContext {
    SourceContext {
        imported_crates: crates.iter().map(|s| s.to_string()).collect(),
        imported_items: HashMap::new(),
        type_context: TypeContext::default(),
        active_ref_files: Vec::new(),
    }
}

fn make_ref(name: &str) -> ReferenceFile {
    ReferenceFile {
        library_name: name.to_string(),
        version: "1.0.0".to_string(),
        language: Language::Rust,
        entries: vec![],
        raw_content: String::new(),
        file_path: PathBuf::from(format!("refs/rust/lib_{}.rs", name)),
    }
}

#[test]
fn test_compute_coverage_all_covered() {
    let ctx = make_ctx(&["tokio", "serde"]);
    let refs = vec![make_ref("tokio"), make_ref("serde")];
    let report = compute_coverage(&ctx, &refs);
    assert_eq!(report.coverage_pct, 100.0);
    assert!(report.uncovered_packages.is_empty());
    assert_eq!(report.covered_calls, 2);
    assert_eq!(report.total_api_calls, 2);
}

#[test]
fn test_compute_coverage_none_covered() {
    let ctx = make_ctx(&["reqwest"]);
    let refs = vec![make_ref("tokio")];
    let report = compute_coverage(&ctx, &refs);
    assert_eq!(report.coverage_pct, 0.0);
    assert!(report.uncovered_packages.contains(&"reqwest".to_string()));
    assert_eq!(report.covered_calls, 0);
}

#[test]
fn test_compute_coverage_partial() {
    let ctx = make_ctx(&["tokio", "reqwest", "serde"]);
    let refs = vec![make_ref("tokio"), make_ref("serde")];
    let report = compute_coverage(&ctx, &refs);
    assert!(report.uncovered_packages.contains(&"reqwest".to_string()));
    assert!(report.coverage_pct < 100.0);
    assert!(report.coverage_pct > 0.0);
    assert_eq!(report.covered_calls, 2);
    assert_eq!(report.total_api_calls, 3);
}

#[test]
fn test_coverage_pct_bounded() {
    // No external crates → 100%
    let ctx_empty = make_ctx(&[]);
    let report_empty = compute_coverage(&ctx_empty, &[]);
    assert!(report_empty.coverage_pct >= 0.0);
    assert!(report_empty.coverage_pct <= 100.0);

    // Only builtins → 100%
    let ctx_std = make_ctx(&["std", "core"]);
    let report_std = compute_coverage(&ctx_std, &[]);
    assert!(report_std.coverage_pct >= 0.0);
    assert!(report_std.coverage_pct <= 100.0);

    // Partial → between 0 and 100
    let ctx_partial = make_ctx(&["tokio", "missing"]);
    let report_partial = compute_coverage(&ctx_partial, &[make_ref("tokio")]);
    assert!(report_partial.coverage_pct >= 0.0);
    assert!(report_partial.coverage_pct <= 100.0);
}

#[test]
fn test_check_coverage_gate_strict_blocks() {
    let ctx = make_ctx(&["unknown_lib"]);
    let refs = vec![make_ref("tokio")];
    let report = compute_coverage(&ctx, &refs);

    let config = EnforceConfig {
        strict_unknown_packages: true,
        hard_block: true,
        ..EnforceConfig::default()
    };
    let gate = check_coverage_gate(&report, &config);
    assert!(gate.is_some(), "strict mode should block uncovered packages");
    assert!(gate.unwrap().contains("uncovered"));
}

#[test]
fn test_check_coverage_gate_strict_not_set_passes() {
    let ctx = make_ctx(&["unknown_lib"]);
    let refs = vec![make_ref("tokio")];
    let report = compute_coverage(&ctx, &refs);

    let config = EnforceConfig {
        strict_unknown_packages: false,
        ..EnforceConfig::default()
    };
    let gate = check_coverage_gate(&report, &config);
    assert!(gate.is_none(), "non-strict mode should not block");
}

#[test]
fn test_check_coverage_gate_pct_threshold_blocks() {
    let ctx = make_ctx(&["tokio", "serde", "reqwest"]);
    let refs = vec![make_ref("tokio"), make_ref("serde")];
    let report = compute_coverage(&ctx, &refs);
    // ~66.7% coverage

    let config = EnforceConfig {
        require_coverage: Some(80),
        ..EnforceConfig::default()
    };
    let gate = check_coverage_gate(&report, &config);
    assert!(gate.is_some(), "below-threshold should block");
    assert!(gate.unwrap().contains("below required 80%"));
}

#[test]
fn test_check_coverage_gate_pct_threshold_passes() {
    let ctx = make_ctx(&["tokio", "serde"]);
    let refs = vec![make_ref("tokio"), make_ref("serde")];
    let report = compute_coverage(&ctx, &refs);
    // 100% coverage

    let config = EnforceConfig {
        require_coverage: Some(80),
        ..EnforceConfig::default()
    };
    let gate = check_coverage_gate(&report, &config);
    assert!(gate.is_none(), "above-threshold should pass");
}

#[test]
fn test_format_coverage_report_contains_pct() {
    let ctx = make_ctx(&["tokio", "missing_crate"]);
    let refs = vec![make_ref("tokio")];
    let report = compute_coverage(&ctx, &refs);
    let output = format_coverage_report(&report);
    assert!(output.contains("50.0%"), "should contain percentage");
    assert!(
        output.contains("missing_crate"),
        "should list uncovered package names"
    );
}

#[test]
fn test_missing_ref_suggestions_populated() {
    let ctx = make_ctx(&["pkg_a", "pkg_b"]);
    let refs: Vec<ReferenceFile> = vec![];
    let report = compute_coverage(&ctx, &refs);
    assert_eq!(
        report.missing_ref_suggestions.len(),
        report.uncovered_packages.len(),
        "each uncovered package should have a suggestion"
    );
    assert_eq!(report.missing_ref_suggestions.len(), 2);
}
