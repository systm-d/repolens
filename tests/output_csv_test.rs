//! Integration tests for `CsvOutput`.
//!
//! Covers RFC 4180 quoting, location parsing, TSV escaping, BOM handling,
//! custom delimiters, and the empty-findings case.

use repolens::cli::output::{CsvOutput, ReportRenderer};
use repolens::rules::results::{AuditResults, Finding, Severity};

const HEADER: &str =
    "rule_id,category,severity,file,line,column,message,description,remediation,project";

fn empty_results() -> AuditResults {
    AuditResults::new("test-repo", "opensource")
}

fn results_with(findings: Vec<Finding>) -> AuditResults {
    let mut r = AuditResults::new("test-repo", "opensource");
    for f in findings {
        r.add_finding(f);
    }
    r
}

#[test]
fn rfc4180_field_with_comma_is_quoted() {
    let r = results_with(vec![Finding::new(
        "SEC001",
        "secrets",
        Severity::Critical,
        "hello, world",
    )]);
    let out = CsvOutput::new().render_report(&r).unwrap();
    // The cell containing a comma must be quoted: `"hello, world"`
    assert!(out.contains("\"hello, world\""), "output was: {out}");
}

#[test]
fn rfc4180_double_quote_is_escaped_by_doubling() {
    let r = results_with(vec![Finding::new(
        "SEC001",
        "secrets",
        Severity::Critical,
        "say \"hi\"",
    )]);
    let out = CsvOutput::new().render_report(&r).unwrap();
    // RFC 4180 escape: " → "" inside a quoted field
    assert!(out.contains("\"say \"\"hi\"\"\""), "output was: {out}");
}

#[test]
fn embedded_newline_replaced_when_keep_newlines_false() {
    let r = results_with(vec![Finding::new(
        "SEC001",
        "secrets",
        Severity::Critical,
        "line1\nline2",
    )]);
    let out = CsvOutput::new().render_report(&r).unwrap();
    // No quoting needed because newline was replaced with a space.
    assert!(out.contains("line1 line2"), "output was: {out}");
    // The data row should not contain a literal newline-in-cell.
    let data_lines: Vec<&str> = out.lines().filter(|l| !l.is_empty()).collect();
    assert_eq!(data_lines.len(), 2); // header + one data row
}

#[test]
fn embedded_newline_quoted_when_keep_newlines_true() {
    let r = results_with(vec![Finding::new(
        "SEC001",
        "secrets",
        Severity::Critical,
        "line1\nline2",
    )]);
    let out = CsvOutput::new()
        .with_keep_newlines(true)
        .render_report(&r)
        .unwrap();
    // RFC 4180: a field containing a newline must be quoted.
    assert!(out.contains("\"line1\nline2\""), "output was: {out}");
}

#[test]
fn location_with_line_parses_into_file_and_line() {
    let r = results_with(vec![Finding::new(
        "SEC001",
        "secrets",
        Severity::Critical,
        "msg",
    )
    .with_location("src/config.rs:42")]);
    let out = CsvOutput::new().render_report(&r).unwrap();
    let row = out.lines().nth(1).unwrap();
    let cols: Vec<&str> = row.split(',').collect();
    // Header: rule_id,category,severity,file,line,column,message,description,remediation,project
    assert_eq!(cols[3], "src/config.rs", "row was: {row}");
    assert_eq!(cols[4], "42", "row was: {row}");
}

#[test]
fn location_without_line_yields_file_and_empty_line() {
    let r = results_with(vec![Finding::new(
        "SEC001",
        "secrets",
        Severity::Critical,
        "msg",
    )
    .with_location("src/config.rs")]);
    let out = CsvOutput::new().render_report(&r).unwrap();
    let row = out.lines().nth(1).unwrap();
    let cols: Vec<&str> = row.split(',').collect();
    assert_eq!(cols[3], "src/config.rs", "row was: {row}");
    assert_eq!(cols[4], "", "row was: {row}");
}

#[test]
fn no_location_yields_empty_file_and_line() {
    let r = results_with(vec![Finding::new(
        "SEC001",
        "secrets",
        Severity::Critical,
        "msg",
    )]);
    let out = CsvOutput::new().render_report(&r).unwrap();
    let row = out.lines().nth(1).unwrap();
    let cols: Vec<&str> = row.split(',').collect();
    assert_eq!(cols[3], "", "row was: {row}");
    assert_eq!(cols[4], "", "row was: {row}");
}

#[test]
fn tsv_replaces_tab_in_field_with_four_spaces() {
    let r = results_with(vec![Finding::new(
        "SEC001",
        "secrets",
        Severity::Critical,
        "left\tright",
    )]);
    let out = CsvOutput::new()
        .with_delimiter(b'\t')
        .render_report(&r)
        .unwrap();
    // The literal \t inside the message must be neutralised to 4 spaces.
    assert!(out.contains("left    right"), "output was: {out:?}");
    // The output must contain exactly 2 lines (header + data) and each line
    // must have the same number of tabs (= 9 separators for 10 columns).
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 2);
    for line in &lines {
        assert_eq!(line.matches('\t').count(), 9, "line was: {line}");
    }
}

#[test]
fn tsv_replaces_newline_in_field_with_one_space() {
    let r = results_with(vec![Finding::new(
        "SEC001",
        "secrets",
        Severity::Critical,
        "line1\nline2",
    )]);
    let out = CsvOutput::new()
        .with_delimiter(b'\t')
        .render_report(&r)
        .unwrap();
    assert!(out.contains("line1 line2"), "output was: {out:?}");
    // Still exactly 2 lines (header + data) — newline replaced.
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 2);
}

#[test]
fn bom_bytes_present_when_csv_bom_enabled() {
    let r = empty_results();
    let out = CsvOutput::new().with_bom(true).render_report(&r).unwrap();
    let bytes = out.as_bytes();
    assert_eq!(
        &bytes[..3],
        b"\xEF\xBB\xBF",
        "first bytes: {:?}",
        &bytes[..3.min(bytes.len())]
    );
}

#[test]
fn tsv_with_bom_does_not_prepend_bom_bytes() {
    let r = empty_results();
    let out = CsvOutput::new()
        .with_delimiter(b'\t')
        .with_bom(true)
        .render_report(&r)
        .unwrap();
    let bytes = out.as_bytes();
    // Either the file starts with the header directly, or at least it must NOT
    // start with the BOM marker.
    assert!(bytes.len() < 3 || &bytes[..3] != b"\xEF\xBB\xBF");
    // Header should still be present (TSV, header still emitted).
    assert!(out.starts_with("rule_id\t"));
}

#[test]
fn custom_delimiter_pipe_is_used_instead_of_comma() {
    let r = results_with(vec![Finding::new(
        "SEC001",
        "secrets",
        Severity::Critical,
        "msg",
    )]);
    let out = CsvOutput::new()
        .with_delimiter(b'|')
        .render_report(&r)
        .unwrap();
    let header = out.lines().next().unwrap();
    assert!(
        header.starts_with("rule_id|category|severity|"),
        "header was: {header}"
    );
    let row = out.lines().nth(1).unwrap();
    assert!(
        row.starts_with("SEC001|secrets|critical|"),
        "row was: {row}"
    );
}

#[test]
fn empty_findings_list_produces_only_header() {
    let r = empty_results();
    let out = CsvOutput::new().render_report(&r).unwrap();
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0], HEADER);
}

#[test]
fn project_column_uses_repository_name() {
    let mut r = AuditResults::new("my-project", "opensource");
    r.add_finding(Finding::new("SEC001", "secrets", Severity::Critical, "msg"));
    let out = CsvOutput::new().render_report(&r).unwrap();
    let row = out.lines().nth(1).unwrap();
    let cols: Vec<&str> = row.split(',').collect();
    assert_eq!(cols.last().copied().unwrap_or(""), "my-project");
}
