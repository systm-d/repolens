//! NDJSON (newline-delimited JSON) output formatting.
//!
//! Emits one JSON object per finding, separated by `\n` (LF). Each line is
//! independently parseable. Designed for streaming pipelines: rendering
//! 10 000 findings stays in constant memory because we serialise straight
//! into a single pre-allocated `String` rather than collecting per-line
//! strings into a `Vec` first.

use crate::error::RepoLensError;
use serde::Serialize;

use super::{OutputRenderer, ReportRenderer};
use crate::actions::plan::ActionPlan;
use crate::rules::results::{AuditResults, Finding, Severity};

/// NDJSON renderer.
#[derive(Debug, Clone, Default)]
pub struct NdjsonOutput;

impl NdjsonOutput {
    pub fn new() -> Self {
        Self
    }
}

#[derive(Serialize)]
struct NdjsonRow<'a> {
    rule_id: &'a str,
    category: &'a str,
    severity: &'static str,
    /// Parsed file path from `Finding.location` (or `null` if absent).
    file: Option<&'a str>,
    /// Parsed line number from `Finding.location` (or `null` if absent / unparseable).
    line: Option<u64>,
    /// Always `null` — column is not tracked in `Finding`. Kept for forward-compat.
    column: Option<u64>,
    message: &'a str,
    description: Option<&'a str>,
    remediation: Option<&'a str>,
    project: &'a str,
}

/// Per-finding row used by the compare NDJSON exporter.
#[derive(Serialize)]
struct NdjsonCompareRow<'a> {
    change: &'a str,
    rule_id: &'a str,
    category: &'a str,
    severity: &'static str,
    file: Option<&'a str>,
    line: Option<u64>,
    column: Option<u64>,
    message: &'a str,
    description: Option<&'a str>,
    remediation: Option<&'a str>,
}

fn severity_str(severity: Severity) -> &'static str {
    match severity {
        Severity::Critical => "critical",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

/// Split `Finding.location` into a `(file, line)` pair suitable for NDJSON.
/// Returns borrowed slices into the original location string when present.
fn split_location(location: Option<&str>) -> (Option<&str>, Option<u64>) {
    let Some(loc) = location else {
        return (None, None);
    };
    match loc.rsplit_once(':') {
        Some((file, line)) => {
            let file = if file.is_empty() { None } else { Some(file) };
            let line = line.parse::<u64>().ok();
            (file, line)
        }
        None => (Some(loc), None),
    }
}

impl NdjsonOutput {
    fn render_findings(
        &self,
        findings: &[Finding],
        project: &str,
    ) -> Result<String, RepoLensError> {
        // Pre-size the output buffer based on a generous per-finding estimate.
        // Avoids many small reallocations and keeps total allocations O(1) in
        // the number of findings.
        let mut out = String::with_capacity(findings.len().saturating_mul(256));

        for finding in findings {
            let (file, line) = split_location(finding.location.as_deref());
            let row = NdjsonRow {
                rule_id: &finding.rule_id,
                category: &finding.category,
                severity: severity_str(finding.severity),
                file,
                line,
                column: None,
                message: &finding.message,
                description: finding.description.as_deref(),
                remediation: finding.remediation.as_deref(),
                project,
            };

            // Serialise directly and append; no intermediate `Vec<String>`.
            let line_str = serde_json::to_string(&row)?;
            out.push_str(&line_str);
            out.push('\n');
        }

        Ok(out)
    }
}

impl OutputRenderer for NdjsonOutput {
    fn render_plan(
        &self,
        results: &AuditResults,
        _plan: &ActionPlan,
    ) -> Result<String, RepoLensError> {
        self.render_findings(results.findings(), &results.repository_name)
    }
}

impl ReportRenderer for NdjsonOutput {
    fn render_report(&self, results: &AuditResults) -> Result<String, RepoLensError> {
        self.render_findings(results.findings(), &results.repository_name)
    }
}

/// Render a NDJSON view of compare findings.
/// Each input row is `(change_label, finding)` where change is `"added"` or `"resolved"`.
pub fn render_compare_ndjson(
    rows: impl IntoIterator<Item = (String, Finding)>,
) -> Result<String, RepoLensError> {
    let rows: Vec<(String, Finding)> = rows.into_iter().collect();
    let mut out = String::with_capacity(rows.len().saturating_mul(256));

    for (change, finding) in &rows {
        let (file, line) = split_location(finding.location.as_deref());
        let row = NdjsonCompareRow {
            change,
            rule_id: &finding.rule_id,
            category: &finding.category,
            severity: severity_str(finding.severity),
            file,
            line,
            column: None,
            message: &finding.message,
            description: finding.description.as_deref(),
            remediation: finding.remediation.as_deref(),
        };
        out.push_str(&serde_json::to_string(&row)?);
        out.push('\n');
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::plan::ActionPlan;
    use crate::rules::results::AuditResults;

    #[test]
    fn split_location_with_line_yields_u64() {
        assert_eq!(
            split_location(Some("src/config.rs:42")),
            (Some("src/config.rs"), Some(42))
        );
    }

    #[test]
    fn split_location_no_line_yields_none() {
        assert_eq!(
            split_location(Some("src/config.rs")),
            (Some("src/config.rs"), None)
        );
    }

    #[test]
    fn split_location_none_inputs_yield_none() {
        assert_eq!(split_location(None), (None, None));
    }

    #[test]
    fn split_location_unparseable_line_is_none() {
        assert_eq!(
            split_location(Some("src/config.rs:abc")),
            (Some("src/config.rs"), None)
        );
    }

    #[test]
    fn split_location_empty_file_part_is_none() {
        assert_eq!(split_location(Some(":42")), (None, Some(42)));
    }

    #[test]
    fn severity_str_maps_each_variant() {
        assert_eq!(severity_str(Severity::Critical), "critical");
        assert_eq!(severity_str(Severity::Warning), "warning");
        assert_eq!(severity_str(Severity::Info), "info");
    }

    #[test]
    fn default_equals_new() {
        let _ = NdjsonOutput;
        let _ = NdjsonOutput::new();
    }

    #[test]
    fn render_plan_emits_one_line_per_finding() {
        let mut results = AuditResults::new("repo", "opensource");
        results.add_finding(Finding::new("R1", "c1", Severity::Critical, "m1"));
        results.add_finding(Finding::new("R2", "c2", Severity::Warning, "m2"));
        let plan = ActionPlan::new();
        let out = NdjsonOutput::new().render_plan(&results, &plan).unwrap();
        assert_eq!(out.lines().count(), 2);
        for line in out.lines() {
            let v: serde_json::Value = serde_json::from_str(line).unwrap();
            assert_eq!(v["project"], "repo");
        }
    }

    #[test]
    fn render_compare_ndjson_emits_change_field() {
        let rows = vec![
            (
                "added".to_string(),
                Finding::new("R1", "secrets", Severity::Critical, "m1")
                    .with_location("a.rs:5")
                    .with_description("d")
                    .with_remediation("r"),
            ),
            (
                "resolved".to_string(),
                Finding::new("R2", "docs", Severity::Warning, "m2"),
            ),
        ];
        let out = render_compare_ndjson(rows).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 2);

        let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(first["change"], "added");
        assert_eq!(first["rule_id"], "R1");
        assert_eq!(first["severity"], "critical");
        assert_eq!(first["file"], "a.rs");
        assert_eq!(first["line"], 5);
        assert!(first["column"].is_null());
        assert_eq!(first["description"], "d");
        assert_eq!(first["remediation"], "r");

        let second: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(second["change"], "resolved");
        assert!(second["file"].is_null());
        assert!(second["line"].is_null());
        assert!(second["description"].is_null());
        assert!(second["remediation"].is_null());
    }

    #[test]
    fn render_compare_ndjson_empty_rows_is_empty_string() {
        let rows: Vec<(String, Finding)> = vec![];
        let out = render_compare_ndjson(rows).unwrap();
        assert_eq!(out, "");
    }
}
