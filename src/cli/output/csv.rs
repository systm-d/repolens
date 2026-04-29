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
}
