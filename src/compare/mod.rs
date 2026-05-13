//! Compare module - Compare two audit reports and generate a diff report
//!
//! This module provides functionality to compare two `AuditResults` instances
//! and produce a `CompareReport` that highlights new issues, resolved issues,
//! and score changes between the two audits.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::cli::output::render_junit_findings;
use crate::error::RepoLensError;
use crate::rules::results::{AuditResults, Finding, Severity};

/// A unique key that identifies a finding for comparison purposes.
/// Two findings are considered the same if they share the same rule_id,
/// category, message, and location.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct FindingKey {
    pub rule_id: String,
    pub category: String,
    pub message: String,
    pub location: Option<String>,
}

impl FindingKey {
    /// Create a key from a Finding
    pub fn from_finding(finding: &Finding) -> Self {
        Self {
            rule_id: finding.rule_id.clone(),
            category: finding.category.clone(),
            message: finding.message.clone(),
            location: finding.location.clone(),
        }
    }
}

/// Summary of findings count changes per category
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryDiff {
    /// Category name
    pub category: String,
    /// Number of findings in base report
    pub base_count: usize,
    /// Number of findings in head report
    pub head_count: usize,
    /// Difference (head - base), positive means regression
    pub diff: i64,
}

/// The result of comparing two audit reports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompareReport {
    /// The base reference label
    pub base_ref: String,
    /// The head reference label
    pub head_ref: String,
    /// Findings from the base report
    pub base_findings: Vec<Finding>,
    /// Findings from the head report
    pub head_findings: Vec<Finding>,
    /// Findings present in head but not in base (new issues / regressions)
    pub added_findings: Vec<Finding>,
    /// Findings present in base but not in head (resolved issues / improvements)
    pub removed_findings: Vec<Finding>,
    /// Findings present in both base and head
    pub unchanged_findings: Vec<Finding>,
    /// Score for the base report (lower is better -- count of issues)
    pub base_score: i64,
    /// Score for the head report
    pub head_score: i64,
    /// Score difference (head - base), negative means improvement
    pub score_diff: i64,
    /// Per-category breakdown
    pub category_diffs: Vec<CategoryDiff>,
}

impl CompareReport {
    /// Returns true if there are regressions (new issues added)
    pub fn has_regressions(&self) -> bool {
        !self.added_findings.is_empty()
    }

    /// Returns true if there are improvements (issues resolved).
    ///
    /// Part of the public API - allows external code to check
    /// if a comparison shows improvement in audit results.
    #[allow(dead_code)]
    pub fn has_improvements(&self) -> bool {
        !self.removed_findings.is_empty()
    }
}

/// Compute a weighted score from audit results.
/// Critical = 10 points, Warning = 3 points, Info = 1 point.
pub fn compute_score(results: &AuditResults) -> i64 {
    let critical = results.count_by_severity(Severity::Critical) as i64;
    let warning = results.count_by_severity(Severity::Warning) as i64;
    let info = results.count_by_severity(Severity::Info) as i64;
    critical * 10 + warning * 3 + info
}

/// Compare two audit results and produce a CompareReport.
///
/// Matching of findings is done by (rule_id, category, message, location).
pub fn compare_results(
    base: &AuditResults,
    head: &AuditResults,
    base_ref: &str,
    head_ref: &str,
) -> CompareReport {
    let base_keys: HashSet<FindingKey> = base
        .findings()
        .iter()
        .map(FindingKey::from_finding)
        .collect();
    let head_keys: HashSet<FindingKey> = head
        .findings()
        .iter()
        .map(FindingKey::from_finding)
        .collect();

    let added_findings: Vec<Finding> = head
        .findings()
        .iter()
        .filter(|f| !base_keys.contains(&FindingKey::from_finding(f)))
        .cloned()
        .collect();

    let removed_findings: Vec<Finding> = base
        .findings()
        .iter()
        .filter(|f| !head_keys.contains(&FindingKey::from_finding(f)))
        .cloned()
        .collect();

    let unchanged_findings: Vec<Finding> = head
        .findings()
        .iter()
        .filter(|f| base_keys.contains(&FindingKey::from_finding(f)))
        .cloned()
        .collect();

    let base_score = compute_score(base);
    let head_score = compute_score(head);
    let score_diff = head_score - base_score;

    // Collect all categories from both reports
    let mut all_categories: Vec<String> = Vec::new();
    for f in base.findings().iter().chain(head.findings().iter()) {
        if !all_categories.contains(&f.category) {
            all_categories.push(f.category.clone());
        }
    }
    all_categories.sort();

    let category_diffs: Vec<CategoryDiff> = all_categories
        .into_iter()
        .map(|cat| {
            let base_count = base.findings_by_category(&cat).count();
            let head_count = head.findings_by_category(&cat).count();
            CategoryDiff {
                category: cat,
                base_count,
                head_count,
                diff: head_count as i64 - base_count as i64,
            }
        })
        .collect();

    CompareReport {
        base_ref: base_ref.to_string(),
        head_ref: head_ref.to_string(),
        base_findings: base.findings().to_vec(),
        head_findings: head.findings().to_vec(),
        added_findings,
        removed_findings,
        unchanged_findings,
        base_score,
        head_score,
        score_diff,
        category_diffs,
    }
}

/// Format the compare report as colored terminal output
pub fn format_terminal(report: &CompareReport) -> String {
    use colored::Colorize;

    let mut output = String::new();

    // Header
    output.push_str(&format!(
        "\n{}\n\n",
        "RepoLens Audit Comparison".cyan().bold()
    ));
    output.push_str(&format!(
        "  {} {}\n",
        "Base:".dimmed(),
        report.base_ref.white().bold()
    ));
    output.push_str(&format!(
        "  {} {}\n",
        "Head:".dimmed(),
        report.head_ref.white().bold()
    ));

    // Score summary
    output.push_str(&format!("\n{}\n", "━".repeat(50).dimmed()));
    output.push_str(&format!("  {}\n\n", "SCORE".bold()));

    let score_arrow = if report.score_diff < 0 {
        format!("{}", format!("{} (improved)", report.score_diff).green())
    } else if report.score_diff > 0 {
        format!("{}", format!("+{} (regressed)", report.score_diff).red())
    } else {
        format!("{}", "0 (no change)".dimmed())
    };

    output.push_str(&format!(
        "  {} -> {} ({})\n",
        report.base_score.to_string().white(),
        report.head_score.to_string().white(),
        score_arrow,
    ));

    // New issues (regressions)
    output.push_str(&format!("\n{}\n", "━".repeat(50).dimmed()));
    output.push_str(&format!(
        "  {} ({})\n\n",
        "NEW ISSUES".red().bold(),
        report.added_findings.len()
    ));

    if report.added_findings.is_empty() {
        output.push_str(&format!("  {}\n", "No new issues.".green()));
    } else {
        for finding in &report.added_findings {
            let severity_tag = match finding.severity {
                Severity::Critical => "CRITICAL".red().bold().to_string(),
                Severity::Warning => "WARNING".yellow().bold().to_string(),
                Severity::Info => "INFO".blue().bold().to_string(),
            };
            output.push_str(&format!(
                "  {} [{}] [{}] {}\n",
                "+".red(),
                finding.rule_id.cyan(),
                severity_tag,
                finding.message
            ));
            if let Some(loc) = &finding.location {
                output.push_str(&format!("    {} {}\n", "└─".dimmed(), loc.dimmed()));
            }
        }
    }

    // Resolved issues (improvements)
    output.push_str(&format!("\n{}\n", "━".repeat(50).dimmed()));
    output.push_str(&format!(
        "  {} ({})\n\n",
        "RESOLVED ISSUES".green().bold(),
        report.removed_findings.len()
    ));

    if report.removed_findings.is_empty() {
        output.push_str(&format!("  {}\n", "No resolved issues.".dimmed()));
    } else {
        for finding in &report.removed_findings {
            let severity_tag = match finding.severity {
                Severity::Critical => "CRITICAL".red().bold().to_string(),
                Severity::Warning => "WARNING".yellow().bold().to_string(),
                Severity::Info => "INFO".blue().bold().to_string(),
            };
            output.push_str(&format!(
                "  {} [{}] [{}] {}\n",
                "-".green(),
                finding.rule_id.cyan(),
                severity_tag,
                finding.message
            ));
            if let Some(loc) = &finding.location {
                output.push_str(&format!("    {} {}\n", "└─".dimmed(), loc.dimmed()));
            }
        }
    }

    // Category breakdown
    if !report.category_diffs.is_empty() {
        output.push_str(&format!("\n{}\n", "━".repeat(50).dimmed()));
        output.push_str(&format!("  {}\n\n", "CATEGORY BREAKDOWN".bold()));
        output.push_str(&format!(
            "  {:<15} {:>6} {:>6} {:>8}\n",
            "Category", "Base", "Head", "Diff"
        ));
        output.push_str(&format!("  {}\n", "─".repeat(40)));

        for cat in &report.category_diffs {
            let diff_str = if cat.diff > 0 {
                format!("+{}", cat.diff).red().to_string()
            } else if cat.diff < 0 {
                format!("{}", cat.diff).green().to_string()
            } else {
                "0".dimmed().to_string()
            };
            output.push_str(&format!(
                "  {:<15} {:>6} {:>6} {:>8}\n",
                cat.category, cat.base_count, cat.head_count, diff_str
            ));
        }
    }

    // Unchanged summary
    output.push_str(&format!("\n{}\n", "━".repeat(50).dimmed()));
    output.push_str(&format!(
        "  {} {} unchanged issue(s)\n\n",
        "Unchanged:".dimmed(),
        report.unchanged_findings.len()
    ));

    output
}

/// Format the compare report as JSON
pub fn format_json(report: &CompareReport) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(report)
}

/// Format the compare report as CSV / TSV.
///
/// Adds a `change` column (`added` / `resolved`) and omits `project` (the report
/// already represents two named refs). Header:
/// `change,rule_id,category,severity,file,line,column,message,description,remediation`.
pub fn format_csv(
    report: &CompareReport,
    delimiter: u8,
    bom: bool,
    keep_newlines: bool,
) -> Result<String, RepoLensError> {
    let rows = compare_rows(report);
    crate::cli::output::csv::render_compare_csv(rows, delimiter, bom, keep_newlines)
}

/// Format the compare report as NDJSON.
///
/// Each line is one finding with a `change` field (`added` / `resolved`).
pub fn format_ndjson(report: &CompareReport) -> Result<String, RepoLensError> {
    let rows = compare_rows(report);
    crate::cli::output::ndjson::render_compare_ndjson(rows)
}

/// Format the compare report as JUnit XML.
///
/// Only regressions (`added_findings`) are emitted as `<testcase>` elements.
/// Resolved findings are silent — they have no failing-test analogue.
pub fn format_junit(report: &CompareReport) -> Result<String, RepoLensError> {
    render_junit_findings(&report.added_findings)
}

/// Build the (change_label, finding) row stream consumed by the CSV/NDJSON
/// compare renderers. Added findings come first (regressions), then resolved.
fn compare_rows(report: &CompareReport) -> Vec<(String, Finding)> {
    let mut rows = Vec::with_capacity(report.added_findings.len() + report.removed_findings.len());
    for f in &report.added_findings {
        rows.push(("added".to_string(), f.clone()));
    }
    for f in &report.removed_findings {
        rows.push(("resolved".to_string(), f.clone()));
    }
    rows
}

/// Format the compare report as Markdown
pub fn format_markdown(report: &CompareReport) -> String {
    let mut output = String::new();

    output.push_str("# RepoLens Audit Comparison\n\n");
    output.push_str(&format!("**Base:** {}\n", report.base_ref));
    output.push_str(&format!("**Head:** {}\n\n", report.head_ref));

    // Score
    output.push_str("## Score\n\n");
    let trend = if report.score_diff < 0 {
        format!("{} (improved)", report.score_diff)
    } else if report.score_diff > 0 {
        format!("+{} (regressed)", report.score_diff)
    } else {
        "0 (no change)".to_string()
    };
    output.push_str(&format!(
        "| Base | Head | Diff |\n|------|------|------|\n| {} | {} | {} |\n\n",
        report.base_score, report.head_score, trend
    ));

    // New issues
    output.push_str(&format!(
        "## New Issues ({})\n\n",
        report.added_findings.len()
    ));
    if report.added_findings.is_empty() {
        output.push_str("No new issues.\n\n");
    } else {
        output.push_str("| Rule | Severity | Message | Location |\n");
        output.push_str("|------|----------|---------|----------|\n");
        for f in &report.added_findings {
            let sev = match f.severity {
                Severity::Critical => "Critical",
                Severity::Warning => "Warning",
                Severity::Info => "Info",
            };
            let loc = f.location.as_deref().unwrap_or("-");
            output.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                f.rule_id, sev, f.message, loc
            ));
        }
        output.push('\n');
    }

    // Resolved issues
    output.push_str(&format!(
        "## Resolved Issues ({})\n\n",
        report.removed_findings.len()
    ));
    if report.removed_findings.is_empty() {
        output.push_str("No resolved issues.\n\n");
    } else {
        output.push_str("| Rule | Severity | Message | Location |\n");
        output.push_str("|------|----------|---------|----------|\n");
        for f in &report.removed_findings {
            let sev = match f.severity {
                Severity::Critical => "Critical",
                Severity::Warning => "Warning",
                Severity::Info => "Info",
            };
            let loc = f.location.as_deref().unwrap_or("-");
            output.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                f.rule_id, sev, f.message, loc
            ));
        }
        output.push('\n');
    }

    // Category breakdown
    if !report.category_diffs.is_empty() {
        output.push_str("## Category Breakdown\n\n");
        output.push_str("| Category | Base | Head | Diff |\n");
        output.push_str("|----------|------|------|------|\n");
        for cat in &report.category_diffs {
            let diff_str = if cat.diff > 0 {
                format!("+{}", cat.diff)
            } else {
                format!("{}", cat.diff)
            };
            output.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                cat.category, cat.base_count, cat.head_count, diff_str
            ));
        }
        output.push('\n');
    }

    // Unchanged
    output.push_str(&format!(
        "**Unchanged issues:** {}\n\n",
        report.unchanged_findings.len()
    ));

    output.push_str("---\n\n");
    output.push_str("*Report generated by [RepoLens](https://github.com/systm-d/repolens)*\n");

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::results::{AuditResults, Finding, Severity};

    fn make_finding(rule_id: &str, category: &str, severity: Severity, message: &str) -> Finding {
        Finding::new(rule_id, category, severity, message)
    }

    fn make_finding_with_location(
        rule_id: &str,
        category: &str,
        severity: Severity,
        message: &str,
        location: &str,
    ) -> Finding {
        Finding::new(rule_id, category, severity, message).with_location(location)
    }

    fn make_results(name: &str, findings: Vec<Finding>) -> AuditResults {
        let mut results = AuditResults::new(name, "opensource");
        results.add_findings(findings);
        results
    }

    // --- Test: identical results ---
    #[test]
    fn test_compare_identical_results() {
        let findings = vec![
            make_finding("SEC001", "secrets", Severity::Critical, "Secret found"),
            make_finding("DOC001", "docs", Severity::Warning, "README missing"),
        ];
        let base = make_results("repo", findings.clone());
        let head = make_results("repo", findings);

        let report = compare_results(&base, &head, "v1.0", "v1.1");

        assert!(report.added_findings.is_empty());
        assert!(report.removed_findings.is_empty());
        assert_eq!(report.unchanged_findings.len(), 2);
        assert_eq!(report.score_diff, 0);
        assert!(!report.has_regressions());
        assert!(!report.has_improvements());
    }

    // --- Test: findings added (regressions) ---
    #[test]
    fn test_compare_with_added_findings() {
        let base = make_results(
            "repo",
            vec![make_finding(
                "DOC001",
                "docs",
                Severity::Warning,
                "README missing",
            )],
        );
        let head = make_results(
            "repo",
            vec![
                make_finding("DOC001", "docs", Severity::Warning, "README missing"),
                make_finding("SEC001", "secrets", Severity::Critical, "Secret found"),
            ],
        );

        let report = compare_results(&base, &head, "base", "head");

        assert_eq!(report.added_findings.len(), 1);
        assert_eq!(report.added_findings[0].rule_id, "SEC001");
        assert!(report.removed_findings.is_empty());
        assert_eq!(report.unchanged_findings.len(), 1);
        assert!(report.has_regressions());
        assert!(!report.has_improvements());
        assert!(report.score_diff > 0); // head has more issues
    }

    // --- Test: findings removed (improvements) ---
    #[test]
    fn test_compare_with_removed_findings() {
        let base = make_results(
            "repo",
            vec![
                make_finding("DOC001", "docs", Severity::Warning, "README missing"),
                make_finding("SEC001", "secrets", Severity::Critical, "Secret found"),
            ],
        );
        let head = make_results(
            "repo",
            vec![make_finding(
                "DOC001",
                "docs",
                Severity::Warning,
                "README missing",
            )],
        );

        let report = compare_results(&base, &head, "before", "after");

        assert!(report.added_findings.is_empty());
        assert_eq!(report.removed_findings.len(), 1);
        assert_eq!(report.removed_findings[0].rule_id, "SEC001");
        assert_eq!(report.unchanged_findings.len(), 1);
        assert!(!report.has_regressions());
        assert!(report.has_improvements());
        assert!(report.score_diff < 0); // head has fewer issues
    }

    // --- Test: mixed changes ---
    #[test]
    fn test_compare_with_mixed_changes() {
        let base = make_results(
            "repo",
            vec![
                make_finding("SEC001", "secrets", Severity::Critical, "Secret found"),
                make_finding("DOC001", "docs", Severity::Warning, "README missing"),
                make_finding(
                    "INFO001",
                    "quality",
                    Severity::Info,
                    "Consider adding tests",
                ),
            ],
        );
        let head = make_results(
            "repo",
            vec![
                make_finding("DOC001", "docs", Severity::Warning, "README missing"),
                make_finding("SEC002", "secrets", Severity::Warning, "Weak password"),
                make_finding("WF001", "workflows", Severity::Info, "No CI configured"),
            ],
        );

        let report = compare_results(&base, &head, "v1", "v2");

        // SEC002 and WF001 are new
        assert_eq!(report.added_findings.len(), 2);
        // SEC001 and INFO001 are resolved
        assert_eq!(report.removed_findings.len(), 2);
        // DOC001 is unchanged
        assert_eq!(report.unchanged_findings.len(), 1);
        assert!(report.has_regressions());
        assert!(report.has_improvements());
    }

    // --- Test: score diff calculation ---
    #[test]
    fn test_score_diff() {
        // base: 1 critical (10) + 1 warning (3) = 13
        let base = make_results(
            "repo",
            vec![
                make_finding("SEC001", "secrets", Severity::Critical, "Secret found"),
                make_finding("DOC001", "docs", Severity::Warning, "README missing"),
            ],
        );
        // head: 1 warning (3) + 1 info (1) = 4
        let head = make_results(
            "repo",
            vec![
                make_finding("DOC001", "docs", Severity::Warning, "README missing"),
                make_finding("INFO001", "quality", Severity::Info, "Consider tests"),
            ],
        );

        let report = compare_results(&base, &head, "old", "new");

        assert_eq!(report.base_score, 13);
        assert_eq!(report.head_score, 4);
        assert_eq!(report.score_diff, -9); // improved by 9 points
    }

    // --- Test: compute_score ---
    #[test]
    fn test_compute_score() {
        let results = make_results(
            "repo",
            vec![
                make_finding("C1", "test", Severity::Critical, "Critical"),
                make_finding("C2", "test", Severity::Critical, "Critical 2"),
                make_finding("W1", "test", Severity::Warning, "Warning"),
                make_finding("I1", "test", Severity::Info, "Info"),
            ],
        );
        // 2*10 + 1*3 + 1*1 = 24
        assert_eq!(compute_score(&results), 24);
    }

    #[test]
    fn test_compute_score_empty() {
        let results = make_results("repo", vec![]);
        assert_eq!(compute_score(&results), 0);
    }

    // --- Test: JSON output ---
    #[test]
    fn test_format_json() {
        let base = make_results(
            "repo",
            vec![make_finding(
                "SEC001",
                "secrets",
                Severity::Critical,
                "Secret found",
            )],
        );
        let head = make_results("repo", vec![]);

        let report = compare_results(&base, &head, "v1", "v2");
        let json_str = format_json(&report).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();

        assert_eq!(parsed["base_ref"], "v1");
        assert_eq!(parsed["head_ref"], "v2");
        assert_eq!(parsed["score_diff"], -10);
        assert_eq!(parsed["removed_findings"].as_array().unwrap().len(), 1);
        assert!(parsed["added_findings"].as_array().unwrap().is_empty());
    }

    // --- Test: fail-on-regression logic ---
    #[test]
    fn test_fail_on_regression_true() {
        let base = make_results("repo", vec![]);
        let head = make_results(
            "repo",
            vec![make_finding(
                "SEC001",
                "secrets",
                Severity::Critical,
                "Secret found",
            )],
        );

        let report = compare_results(&base, &head, "base", "head");
        assert!(report.has_regressions());
    }

    #[test]
    fn test_fail_on_regression_false() {
        let base = make_results(
            "repo",
            vec![make_finding(
                "SEC001",
                "secrets",
                Severity::Critical,
                "Secret found",
            )],
        );
        let head = make_results("repo", vec![]);

        let report = compare_results(&base, &head, "base", "head");
        assert!(!report.has_regressions());
    }

    // --- Test: category diffs ---
    #[test]
    fn test_category_diffs() {
        let base = make_results(
            "repo",
            vec![
                make_finding("SEC001", "secrets", Severity::Critical, "Secret 1"),
                make_finding("SEC002", "secrets", Severity::Warning, "Secret 2"),
                make_finding("DOC001", "docs", Severity::Warning, "Doc issue"),
            ],
        );
        let head = make_results(
            "repo",
            vec![
                make_finding("SEC001", "secrets", Severity::Critical, "Secret 1"),
                make_finding("WF001", "workflows", Severity::Info, "No CI"),
            ],
        );

        let report = compare_results(&base, &head, "v1", "v2");

        // Should have categories: docs, secrets, workflows (sorted)
        assert_eq!(report.category_diffs.len(), 3);

        let docs = report
            .category_diffs
            .iter()
            .find(|c| c.category == "docs")
            .unwrap();
        assert_eq!(docs.base_count, 1);
        assert_eq!(docs.head_count, 0);
        assert_eq!(docs.diff, -1);

        let secrets = report
            .category_diffs
            .iter()
            .find(|c| c.category == "secrets")
            .unwrap();
        assert_eq!(secrets.base_count, 2);
        assert_eq!(secrets.head_count, 1);
        assert_eq!(secrets.diff, -1);

        let workflows = report
            .category_diffs
            .iter()
            .find(|c| c.category == "workflows")
            .unwrap();
        assert_eq!(workflows.base_count, 0);
        assert_eq!(workflows.head_count, 1);
        assert_eq!(workflows.diff, 1);
    }

    // --- Test: FindingKey ---
    #[test]
    fn test_finding_key_from_finding() {
        let finding =
            make_finding_with_location("SEC001", "secrets", Severity::Critical, "Secret", "a.rs");
        let key = FindingKey::from_finding(&finding);
        assert_eq!(key.rule_id, "SEC001");
        assert_eq!(key.category, "secrets");
        assert_eq!(key.message, "Secret");
        assert_eq!(key.location, Some("a.rs".to_string()));
    }

    #[test]
    fn test_finding_key_equality() {
        let f1 = make_finding("SEC001", "secrets", Severity::Critical, "Secret");
        let f2 = make_finding("SEC001", "secrets", Severity::Warning, "Secret");
        // Same key regardless of severity
        assert_eq!(FindingKey::from_finding(&f1), FindingKey::from_finding(&f2));
    }

    #[test]
    fn test_finding_key_inequality_different_message() {
        let f1 = make_finding("SEC001", "secrets", Severity::Critical, "Secret A");
        let f2 = make_finding("SEC001", "secrets", Severity::Critical, "Secret B");
        assert_ne!(FindingKey::from_finding(&f1), FindingKey::from_finding(&f2));
    }

    // --- Test: terminal output ---
    #[test]
    fn test_format_terminal_with_changes() {
        let base = make_results(
            "repo",
            vec![make_finding(
                "SEC001",
                "secrets",
                Severity::Critical,
                "Secret found",
            )],
        );
        let head = make_results(
            "repo",
            vec![make_finding(
                "DOC001",
                "docs",
                Severity::Warning,
                "README missing",
            )],
        );

        let report = compare_results(&base, &head, "v1.0", "v2.0");
        let output = format_terminal(&report);

        assert!(output.contains("v1.0"));
        assert!(output.contains("v2.0"));
        assert!(output.contains("NEW ISSUES"));
        assert!(output.contains("RESOLVED ISSUES"));
        assert!(output.contains("DOC001"));
        assert!(output.contains("SEC001"));
    }

    #[test]
    fn test_format_terminal_no_changes() {
        let findings = vec![make_finding(
            "DOC001",
            "docs",
            Severity::Warning,
            "README missing",
        )];
        let base = make_results("repo", findings.clone());
        let head = make_results("repo", findings);

        let report = compare_results(&base, &head, "a", "b");
        let output = format_terminal(&report);

        assert!(output.contains("No new issues"));
        assert!(output.contains("No resolved issues"));
        assert!(output.contains("0 (no change)"));
    }

    #[test]
    fn test_format_terminal_improvement() {
        let base = make_results(
            "repo",
            vec![make_finding(
                "SEC001",
                "secrets",
                Severity::Critical,
                "Secret found",
            )],
        );
        let head = make_results("repo", vec![]);

        let report = compare_results(&base, &head, "before", "after");
        let output = format_terminal(&report);

        assert!(output.contains("improved"));
    }

    #[test]
    fn test_format_terminal_regression() {
        let base = make_results("repo", vec![]);
        let head = make_results(
            "repo",
            vec![make_finding(
                "SEC001",
                "secrets",
                Severity::Critical,
                "Secret found",
            )],
        );

        let report = compare_results(&base, &head, "before", "after");
        let output = format_terminal(&report);

        assert!(output.contains("regressed"));
    }

    #[test]
    fn test_format_terminal_with_location() {
        let base = make_results("repo", vec![]);
        let head = make_results(
            "repo",
            vec![make_finding_with_location(
                "SEC001",
                "secrets",
                Severity::Critical,
                "Secret found",
                "src/config.rs:42",
            )],
        );

        let report = compare_results(&base, &head, "a", "b");
        let output = format_terminal(&report);

        assert!(output.contains("src/config.rs:42"));
    }

    // --- Test: markdown output ---
    #[test]
    fn test_format_markdown_with_changes() {
        let base = make_results(
            "repo",
            vec![make_finding(
                "SEC001",
                "secrets",
                Severity::Critical,
                "Secret found",
            )],
        );
        let head = make_results(
            "repo",
            vec![make_finding(
                "DOC001",
                "docs",
                Severity::Warning,
                "README missing",
            )],
        );

        let report = compare_results(&base, &head, "v1", "v2");
        let output = format_markdown(&report);

        assert!(output.contains("# RepoLens Audit Comparison"));
        assert!(output.contains("**Base:** v1"));
        assert!(output.contains("**Head:** v2"));
        assert!(output.contains("## New Issues (1)"));
        assert!(output.contains("## Resolved Issues (1)"));
        assert!(output.contains("DOC001"));
        assert!(output.contains("SEC001"));
        assert!(output.contains("## Category Breakdown"));
    }

    #[test]
    fn test_format_markdown_no_issues() {
        let base = make_results("repo", vec![]);
        let head = make_results("repo", vec![]);

        let report = compare_results(&base, &head, "a", "b");
        let output = format_markdown(&report);

        assert!(output.contains("No new issues"));
        assert!(output.contains("No resolved issues"));
        assert!(output.contains("0 (no change)"));
    }

    #[test]
    fn test_format_markdown_regression_score() {
        let base = make_results("repo", vec![]);
        let head = make_results(
            "repo",
            vec![make_finding(
                "SEC001",
                "secrets",
                Severity::Critical,
                "Secret found",
            )],
        );

        let report = compare_results(&base, &head, "a", "b");
        let output = format_markdown(&report);

        assert!(output.contains("+10 (regressed)"));
    }

    #[test]
    fn test_format_markdown_improved_score() {
        let base = make_results(
            "repo",
            vec![make_finding(
                "SEC001",
                "secrets",
                Severity::Critical,
                "Secret found",
            )],
        );
        let head = make_results("repo", vec![]);

        let report = compare_results(&base, &head, "a", "b");
        let output = format_markdown(&report);

        assert!(output.contains("-10 (improved)"));
    }

    #[test]
    fn test_format_markdown_with_location() {
        let base = make_results("repo", vec![]);
        let head = make_results(
            "repo",
            vec![make_finding_with_location(
                "SEC001",
                "secrets",
                Severity::Critical,
                "Secret found",
                "src/main.rs:10",
            )],
        );

        let report = compare_results(&base, &head, "a", "b");
        let output = format_markdown(&report);

        assert!(output.contains("src/main.rs:10"));
    }

    #[test]
    fn test_format_markdown_no_location() {
        let base = make_results("repo", vec![]);
        let head = make_results(
            "repo",
            vec![make_finding(
                "SEC001",
                "secrets",
                Severity::Critical,
                "Secret found",
            )],
        );

        let report = compare_results(&base, &head, "a", "b");
        let output = format_markdown(&report);

        // Should use "-" for missing location
        assert!(output.contains("| - |"));
    }

    // --- Test: empty results ---
    #[test]
    fn test_compare_empty_results() {
        let base = make_results("repo", vec![]);
        let head = make_results("repo", vec![]);

        let report = compare_results(&base, &head, "empty1", "empty2");

        assert!(report.added_findings.is_empty());
        assert!(report.removed_findings.is_empty());
        assert!(report.unchanged_findings.is_empty());
        assert_eq!(report.base_score, 0);
        assert_eq!(report.head_score, 0);
        assert_eq!(report.score_diff, 0);
        assert!(report.category_diffs.is_empty());
        assert!(!report.has_regressions());
        assert!(!report.has_improvements());
    }

    // --- Test: CompareReport refs ---
    #[test]
    fn test_compare_report_refs() {
        let base = make_results("repo", vec![]);
        let head = make_results("repo", vec![]);

        let report = compare_results(&base, &head, "my-base", "my-head");

        assert_eq!(report.base_ref, "my-base");
        assert_eq!(report.head_ref, "my-head");
    }

    // --- Test: format_terminal with all severity types in added ---
    #[test]
    fn test_format_terminal_all_severities() {
        let base = make_results("repo", vec![]);
        let head = make_results(
            "repo",
            vec![
                make_finding("C1", "test", Severity::Critical, "Critical issue"),
                make_finding("W1", "test", Severity::Warning, "Warning issue"),
                make_finding("I1", "test", Severity::Info, "Info issue"),
            ],
        );

        let report = compare_results(&base, &head, "a", "b");
        let output = format_terminal(&report);

        assert!(output.contains("CRITICAL"));
        assert!(output.contains("WARNING"));
        assert!(output.contains("INFO"));
    }

    // --- Test: format_terminal with all severity types in resolved ---
    #[test]
    fn test_format_terminal_all_severities_resolved() {
        let base = make_results(
            "repo",
            vec![
                make_finding("C1", "test", Severity::Critical, "Critical issue"),
                make_finding("W1", "test", Severity::Warning, "Warning issue"),
                make_finding("I1", "test", Severity::Info, "Info issue"),
            ],
        );
        let head = make_results("repo", vec![]);

        let report = compare_results(&base, &head, "a", "b");
        let output = format_terminal(&report);

        assert!(output.contains("RESOLVED ISSUES"));
        assert!(output.contains("C1"));
        assert!(output.contains("W1"));
        assert!(output.contains("I1"));
    }

    // --- Test: format_terminal resolved with location ---
    #[test]
    fn test_format_terminal_resolved_with_location() {
        let base = make_results(
            "repo",
            vec![make_finding_with_location(
                "SEC001",
                "secrets",
                Severity::Critical,
                "Secret found",
                "src/lib.rs:5",
            )],
        );
        let head = make_results("repo", vec![]);

        let report = compare_results(&base, &head, "a", "b");
        let output = format_terminal(&report);

        assert!(output.contains("src/lib.rs:5"));
    }

    // --- Test: category_diffs positive diff ---
    #[test]
    fn test_format_terminal_category_positive_diff() {
        let base = make_results("repo", vec![]);
        let head = make_results(
            "repo",
            vec![
                make_finding("SEC001", "secrets", Severity::Critical, "Secret 1"),
                make_finding("SEC002", "secrets", Severity::Warning, "Secret 2"),
            ],
        );

        let report = compare_results(&base, &head, "a", "b");
        let output = format_terminal(&report);

        assert!(output.contains("CATEGORY BREAKDOWN"));
        assert!(output.contains("secrets"));
    }

    // --- Test: markdown with all severity types in new issues ---
    #[test]
    fn test_format_markdown_new_issues_all_severities() {
        let base = make_results("repo", vec![]);
        let head = make_results(
            "repo",
            vec![
                make_finding("C1", "test", Severity::Critical, "Critical issue"),
                make_finding("W1", "test", Severity::Warning, "Warning issue"),
                make_finding("I1", "test", Severity::Info, "Info issue"),
            ],
        );

        let report = compare_results(&base, &head, "a", "b");
        let output = format_markdown(&report);

        assert!(output.contains("| C1 | Critical |"));
        assert!(output.contains("| W1 | Warning |"));
        assert!(output.contains("| I1 | Info |"));
    }

    // --- Test: markdown with all severity types in resolved issues ---
    #[test]
    fn test_format_markdown_resolved_issues_all_severities() {
        let base = make_results(
            "repo",
            vec![
                make_finding("C1", "test", Severity::Critical, "Critical resolved"),
                make_finding("W1", "test", Severity::Warning, "Warning resolved"),
                make_finding("I1", "test", Severity::Info, "Info resolved"),
            ],
        );
        let head = make_results("repo", vec![]);

        let report = compare_results(&base, &head, "a", "b");
        let output = format_markdown(&report);

        // Verify resolved section has all severities
        assert!(output.contains("## Resolved Issues (3)"));
        assert!(output.contains("| C1 | Critical | Critical resolved"));
        assert!(output.contains("| W1 | Warning | Warning resolved"));
        assert!(output.contains("| I1 | Info | Info resolved"));
    }

    // --- Test: format_markdown footer ---
    #[test]
    fn test_format_markdown_footer() {
        let base = make_results("repo", vec![]);
        let head = make_results("repo", vec![]);

        let report = compare_results(&base, &head, "a", "b");
        let output = format_markdown(&report);

        assert!(output.contains("---"));
        assert!(output.contains("*Report generated by [RepoLens]"));
    }
}
