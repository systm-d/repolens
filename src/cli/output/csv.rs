//! CSV / TSV output formatting.
//!
//! Emits one row per finding with a fixed header:
//! `rule_id,category,severity,file,line,column,message,description,remediation,project`.
//! TSV mode (`delimiter == b'\t'`) replaces tabs (4 spaces) and newlines (1 space)
//! inside fields, since TSV has no quoting. CSV mode uses RFC 4180 quoting via
//! the `csv` crate.

use crate::error::RepoLensError;

use super::{OutputRenderer, ReportRenderer};
use crate::actions::plan::ActionPlan;
use crate::rules::results::{AuditResults, Finding, Severity};

/// CSV / TSV renderer.
#[derive(Debug, Clone)]
pub struct CsvOutput {
    delimiter: u8,
    bom: bool,
    keep_newlines: bool,
}

impl CsvOutput {
    pub fn new() -> Self {
        Self {
            delimiter: b',',
            bom: false,
            keep_newlines: false,
        }
    }

    /// Set the column delimiter (e.g. `b','`, `b';'`, `b'|'`, `b'\t'`).
    pub fn with_delimiter(mut self, delimiter: u8) -> Self {
        self.delimiter = delimiter;
        self
    }

    /// Prepend a UTF-8 BOM (`EF BB BF`). Ignored in TSV mode (a WARN is emitted).
    pub fn with_bom(mut self, bom: bool) -> Self {
        self.bom = bom;
        self
    }

    /// Keep newlines inside CSV cells (quoted) instead of replacing them with a single space.
    /// Has no effect in TSV mode.
    pub fn with_keep_newlines(mut self, keep_newlines: bool) -> Self {
        self.keep_newlines = keep_newlines;
        self
    }
}

impl Default for CsvOutput {
    fn default() -> Self {
        Self::new()
    }
}

const HEADER: &[&str] = &[
    "rule_id",
    "category",
    "severity",
    "file",
    "line",
    "column",
    "message",
    "description",
    "remediation",
    "project",
];

/// Parse `Finding.location` into `(file, line)` columns.
///
/// `"path/to/file:42"` → `("path/to/file", "42")`.
/// `"path/to/file"` (no colon) → `("path/to/file", "")`.
/// `None` → `("", "")`.
fn parse_location(location: Option<&str>) -> (String, String) {
    match location {
        None => (String::new(), String::new()),
        Some(s) => match s.rsplit_once(':') {
            Some((file, line)) => (file.to_string(), line.to_string()),
            None => (s.to_string(), String::new()),
        },
    }
}

fn severity_str(severity: Severity) -> &'static str {
    match severity {
        Severity::Critical => "critical",
        Severity::Warning => "warning",
        Severity::Info => "info",
    }
}

/// In TSV mode, replace embedded tabs (with 4 spaces) and newlines (with 1 space).
/// In CSV mode, optionally replace embedded newlines with a single space.
fn sanitize_cell(value: &str, is_tsv: bool, keep_newlines: bool) -> String {
    if is_tsv {
        value.replace('\t', "    ").replace(['\r', '\n'], " ")
    } else if keep_newlines {
        value.to_string()
    } else {
        value.replace(['\r', '\n'], " ")
    }
}

impl CsvOutput {
    fn render_findings(
        &self,
        findings: &[Finding],
        project: &str,
    ) -> Result<String, RepoLensError> {
        let is_tsv = self.delimiter == b'\t';

        let mut out: Vec<u8> = Vec::new();

        if self.bom {
            if is_tsv {
                eprintln!("[WARN] --csv-bom is not applicable in TSV mode; ignoring BOM prefix.");
            } else {
                out.extend_from_slice(b"\xEF\xBB\xBF");
            }
        }

        let mut writer = csv::WriterBuilder::new()
            .delimiter(self.delimiter)
            .has_headers(false)
            .from_writer(out);

        writer.write_record(HEADER).map_err(csv_err)?;

        let project_cell = sanitize_cell(project, is_tsv, self.keep_newlines);

        for finding in findings {
            let (file, line) = parse_location(finding.location.as_deref());

            let row = [
                sanitize_cell(&finding.rule_id, is_tsv, self.keep_newlines),
                sanitize_cell(&finding.category, is_tsv, self.keep_newlines),
                severity_str(finding.severity).to_string(),
                sanitize_cell(&file, is_tsv, self.keep_newlines),
                sanitize_cell(&line, is_tsv, self.keep_newlines),
                String::new(), // column — not tracked in Finding
                sanitize_cell(&finding.message, is_tsv, self.keep_newlines),
                sanitize_cell(
                    finding.description.as_deref().unwrap_or(""),
                    is_tsv,
                    self.keep_newlines,
                ),
                sanitize_cell(
                    finding.remediation.as_deref().unwrap_or(""),
                    is_tsv,
                    self.keep_newlines,
                ),
                project_cell.clone(),
            ];

            writer.write_record(&row).map_err(csv_err)?;
        }

        let bytes = writer.into_inner().map_err(|e| csv_err(e.into_error()))?;

        // Output is guaranteed UTF-8: every input string is UTF-8 and the csv crate
        // writes ASCII delimiters / quoting around UTF-8 fields.
        String::from_utf8(bytes).map_err(|e| {
            RepoLensError::Action(crate::error::ActionError::ExecutionFailed {
                message: format!("CSV output produced invalid UTF-8: {e}"),
            })
        })
    }
}

fn csv_err(e: impl std::fmt::Display) -> RepoLensError {
    RepoLensError::Action(crate::error::ActionError::ExecutionFailed {
        message: format!("CSV write failed: {e}"),
    })
}

impl OutputRenderer for CsvOutput {
    fn render_plan(
        &self,
        results: &AuditResults,
        _plan: &ActionPlan,
    ) -> Result<String, RepoLensError> {
        self.render_findings(results.findings(), &results.repository_name)
    }
}

impl ReportRenderer for CsvOutput {
    fn render_report(&self, results: &AuditResults) -> Result<String, RepoLensError> {
        self.render_findings(results.findings(), &results.repository_name)
    }
}

/// Render a CSV/TSV view of compare findings.
/// Header: `change,rule_id,category,severity,file,line,column,message,description,remediation`.
pub fn render_compare_csv(
    rows: impl IntoIterator<Item = (String, Finding)>,
    delimiter: u8,
    bom: bool,
    keep_newlines: bool,
) -> Result<String, RepoLensError> {
    let is_tsv = delimiter == b'\t';

    let mut out: Vec<u8> = Vec::new();

    if bom {
        if is_tsv {
            eprintln!("[WARN] --csv-bom is not applicable in TSV mode; ignoring BOM prefix.");
        } else {
            out.extend_from_slice(b"\xEF\xBB\xBF");
        }
    }

    let mut writer = csv::WriterBuilder::new()
        .delimiter(delimiter)
        .has_headers(false)
        .from_writer(out);

    writer
        .write_record([
            "change",
            "rule_id",
            "category",
            "severity",
            "file",
            "line",
            "column",
            "message",
            "description",
            "remediation",
        ])
        .map_err(csv_err)?;

    for (change, finding) in rows {
        let (file, line) = parse_location(finding.location.as_deref());
        let row = [
            sanitize_cell(&change, is_tsv, keep_newlines),
            sanitize_cell(&finding.rule_id, is_tsv, keep_newlines),
            sanitize_cell(&finding.category, is_tsv, keep_newlines),
            severity_str(finding.severity).to_string(),
            sanitize_cell(&file, is_tsv, keep_newlines),
            sanitize_cell(&line, is_tsv, keep_newlines),
            String::new(),
            sanitize_cell(&finding.message, is_tsv, keep_newlines),
            sanitize_cell(
                finding.description.as_deref().unwrap_or(""),
                is_tsv,
                keep_newlines,
            ),
            sanitize_cell(
                finding.remediation.as_deref().unwrap_or(""),
                is_tsv,
                keep_newlines,
            ),
        ];
        writer.write_record(&row).map_err(csv_err)?;
    }

    let bytes = writer.into_inner().map_err(|e| csv_err(e.into_error()))?;
    String::from_utf8(bytes).map_err(|e| {
        RepoLensError::Action(crate::error::ActionError::ExecutionFailed {
            message: format!("CSV output produced invalid UTF-8: {e}"),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::plan::ActionPlan;
    use crate::rules::results::AuditResults;

    #[test]
    fn parse_location_with_line() {
        assert_eq!(
            parse_location(Some("src/config.rs:42")),
            ("src/config.rs".to_string(), "42".to_string())
        );
    }

    #[test]
    fn parse_location_no_line() {
        assert_eq!(
            parse_location(Some("src/config.rs")),
            ("src/config.rs".to_string(), String::new())
        );
    }

    #[test]
    fn parse_location_none() {
        assert_eq!(parse_location(None), (String::new(), String::new()));
    }

    #[test]
    fn parse_location_keeps_only_last_colon() {
        // Useful for paths that legitimately contain colons (e.g. Windows drive letters
        // are unusual here but we still split on the trailing :line marker).
        assert_eq!(
            parse_location(Some("a:b:10")),
            ("a:b".to_string(), "10".to_string())
        );
    }

    #[test]
    fn header_only_for_empty_findings() {
        let renderer = CsvOutput::new();
        let out = renderer.render_findings(&[], "proj").unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 1);
        assert_eq!(
            lines[0],
            "rule_id,category,severity,file,line,column,message,description,remediation,project"
        );
    }

    #[test]
    fn severity_str_maps_each_variant() {
        assert_eq!(severity_str(Severity::Critical), "critical");
        assert_eq!(severity_str(Severity::Warning), "warning");
        assert_eq!(severity_str(Severity::Info), "info");
    }

    #[test]
    fn sanitize_cell_csv_keep_newlines_preserves_content() {
        let s = sanitize_cell("a\nb\tc", false, true);
        assert_eq!(s, "a\nb\tc");
    }

    #[test]
    fn sanitize_cell_csv_replaces_newlines_when_not_keeping() {
        let s = sanitize_cell("a\r\nb", false, false);
        assert_eq!(s, "a  b");
    }

    #[test]
    fn sanitize_cell_tsv_replaces_tabs_and_newlines() {
        let s = sanitize_cell("a\tb\nc\rd", true, false);
        assert_eq!(s, "a    b c d");
    }

    #[test]
    fn default_builds_same_as_new() {
        let a = CsvOutput::default();
        let b = CsvOutput::new();
        assert_eq!(a.delimiter, b.delimiter);
        assert_eq!(a.bom, b.bom);
        assert_eq!(a.keep_newlines, b.keep_newlines);
    }

    #[test]
    fn builder_methods_apply_options() {
        let r = CsvOutput::new()
            .with_delimiter(b';')
            .with_bom(true)
            .with_keep_newlines(true);
        assert_eq!(r.delimiter, b';');
        assert!(r.bom);
        assert!(r.keep_newlines);
    }

    fn sample_findings() -> Vec<Finding> {
        vec![
            Finding::new("SEC001", "secrets", Severity::Critical, "Secret leaked")
                .with_location("src/main.rs:10")
                .with_description("API key detected")
                .with_remediation("Rotate the key"),
            Finding::new("DOC001", "docs", Severity::Warning, "Missing README"),
            Finding::new("Q001", "quality", Severity::Info, "Hint"),
        ]
    }

    #[test]
    fn render_plan_uses_findings_and_repo_name() {
        let mut results = AuditResults::new("repo-x", "opensource");
        for f in sample_findings() {
            results.add_finding(f);
        }
        let plan = ActionPlan::new();
        let out = CsvOutput::new().render_plan(&results, &plan).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 4); // header + 3 findings
        assert!(out.contains("repo-x"));
        assert!(out.contains("SEC001"));
    }

    #[test]
    fn render_report_emits_all_severities() {
        let mut results = AuditResults::new("repo-y", "opensource");
        for f in sample_findings() {
            results.add_finding(f);
        }
        let out = CsvOutput::new().render_report(&results).unwrap();
        assert!(out.contains(",critical,"));
        assert!(out.contains(",warning,"));
        assert!(out.contains(",info,"));
    }

    #[test]
    fn render_compare_csv_emits_change_column() {
        let rows = vec![
            (
                "added".to_string(),
                Finding::new("R1", "secrets", Severity::Critical, "msg1").with_location("a.rs:1"),
            ),
            (
                "resolved".to_string(),
                Finding::new("R2", "docs", Severity::Warning, "msg2"),
            ),
        ];
        let out = render_compare_csv(rows, b',', false, false).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 3); // header + 2 rows
        assert!(lines[0].starts_with("change,rule_id,category,severity,"));
        assert!(lines[1].starts_with("added,R1,secrets,critical,a.rs,1,"));
        assert!(lines[2].starts_with("resolved,R2,docs,warning,"));
    }

    #[test]
    fn render_compare_csv_with_bom_prepends_marker() {
        let rows: Vec<(String, Finding)> = vec![];
        let out = render_compare_csv(rows, b',', true, false).unwrap();
        assert_eq!(&out.as_bytes()[..3], b"\xEF\xBB\xBF");
    }

    #[test]
    fn render_compare_csv_tsv_ignores_bom() {
        let rows: Vec<(String, Finding)> = vec![];
        let out = render_compare_csv(rows, b'\t', true, false).unwrap();
        let bytes = out.as_bytes();
        assert!(bytes.len() < 3 || &bytes[..3] != b"\xEF\xBB\xBF");
        assert!(out.starts_with("change\trule_id\t"));
    }

    #[test]
    fn render_compare_csv_tsv_replaces_special_chars() {
        let rows = vec![(
            "added".to_string(),
            Finding::new("R", "cat", Severity::Info, "a\tb\nc")
                .with_description("d\te")
                .with_remediation("f\ng"),
        )];
        let out = render_compare_csv(rows, b'\t', false, false).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 2);
        // Each line must have exactly 9 separators (10 columns).
        for line in &lines {
            assert_eq!(line.matches('\t').count(), 9, "line was: {line}");
        }
    }

    #[test]
    fn render_compare_csv_keeps_newlines_when_requested() {
        let rows = vec![(
            "added".to_string(),
            Finding::new("R", "cat", Severity::Info, "a\nb"),
        )];
        let out = render_compare_csv(rows, b',', false, true).unwrap();
        // Newline preserved inside quoted CSV cell.
        assert!(out.contains("\"a\nb\""), "output was: {out}");
    }
}
