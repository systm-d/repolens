//! JUnit XML output formatting for CI integration
//!
//! Maps audit findings to a JUnit XML report so that CI systems
//! (GitLab CI, Jenkins, Bitbucket Pipelines, ...) can surface RepoLens
//! findings as test failures. Each rule category becomes a `<testsuite>`
//! and each finding a `<testcase>` whose severity controls the inner
//! element: `Critical` -> `<error>`, `Warning` -> `<failure>`,
//! `Info` -> `<system-out>`.

use std::io::Cursor;

use quick_xml::events::{BytesDecl, BytesEnd, BytesStart, BytesText, Event};
use quick_xml::Writer;

use super::{OutputRenderer, ReportRenderer};
use crate::actions::plan::ActionPlan;
use crate::error::{ActionError, RepoLensError};
use crate::rules::results::{AuditResults, Finding, Severity};

/// JUnit XML renderer.
pub struct JunitReport;

impl JunitReport {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JunitReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Build the `name` attribute of a `<testcase>` from a finding.
fn testcase_name(finding: &Finding) -> String {
    match &finding.location {
        Some(loc) => format!("{} [{}]", finding.rule_id, loc),
        None => finding.rule_id.clone(),
    }
}

fn xml_error(err: std::io::Error) -> RepoLensError {
    RepoLensError::Action(ActionError::ExecutionFailed {
        message: format!("Failed to write JUnit XML: {}", err),
    })
}

/// Group findings by their `category`, preserving first-seen order.
fn group_by_category(findings: &[Finding]) -> Vec<(String, Vec<&Finding>)> {
    let mut groups: Vec<(String, Vec<&Finding>)> = Vec::new();
    for finding in findings {
        if let Some(group) = groups.iter_mut().find(|(cat, _)| cat == &finding.category) {
            group.1.push(finding);
        } else {
            groups.push((finding.category.clone(), vec![finding]));
        }
    }
    groups
}

/// Render an arbitrary slice of findings as a JUnit XML document.
///
/// This is the shared core used by `OutputRenderer`, `ReportRenderer`,
/// and the `compare` command's `format_junit`.
pub(crate) fn render_findings(findings: &[Finding]) -> Result<String, RepoLensError> {
    let mut writer = Writer::new(Cursor::new(Vec::new()));

    writer
        .write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))
        .map_err(xml_error)?;

    let groups = group_by_category(findings);

    let total_tests = findings.len();
    let total_failures = findings
        .iter()
        .filter(|f| f.severity == Severity::Warning)
        .count();
    let total_errors = findings
        .iter()
        .filter(|f| f.severity == Severity::Critical)
        .count();

    let mut testsuites = BytesStart::new("testsuites");
    testsuites.push_attribute(("name", "repolens"));
    testsuites.push_attribute(("tests", total_tests.to_string().as_str()));
    testsuites.push_attribute(("failures", total_failures.to_string().as_str()));
    testsuites.push_attribute(("errors", total_errors.to_string().as_str()));
    testsuites.push_attribute(("skipped", "0"));
    testsuites.push_attribute(("time", "0.000"));
    writer
        .write_event(Event::Start(testsuites))
        .map_err(xml_error)?;

    for (suite_idx, (category, items)) in groups.iter().enumerate() {
        let suite_tests = items.len();
        let suite_failures = items
            .iter()
            .filter(|f| f.severity == Severity::Warning)
            .count();
        let suite_errors = items
            .iter()
            .filter(|f| f.severity == Severity::Critical)
            .count();

        let mut suite = BytesStart::new("testsuite");
        suite.push_attribute(("name", category.as_str()));
        suite.push_attribute(("tests", suite_tests.to_string().as_str()));
        suite.push_attribute(("failures", suite_failures.to_string().as_str()));
        suite.push_attribute(("errors", suite_errors.to_string().as_str()));
        suite.push_attribute(("skipped", "0"));
        suite.push_attribute(("time", "0.000"));
        suite.push_attribute(("id", suite_idx.to_string().as_str()));
        writer.write_event(Event::Start(suite)).map_err(xml_error)?;

        for finding in items {
            let name = testcase_name(finding);
            let mut testcase = BytesStart::new("testcase");
            testcase.push_attribute(("name", name.as_str()));
            testcase.push_attribute(("classname", finding.category.as_str()));
            testcase.push_attribute(("time", "0.000"));
            writer
                .write_event(Event::Start(testcase))
                .map_err(xml_error)?;

            match finding.severity {
                Severity::Critical => {
                    let mut error_el = BytesStart::new("error");
                    error_el.push_attribute(("type", "critical"));
                    error_el.push_attribute(("message", finding.message.as_str()));
                    writer
                        .write_event(Event::Start(error_el))
                        .map_err(xml_error)?;
                    writer
                        .write_event(Event::Text(BytesText::new(&finding_body(finding))))
                        .map_err(xml_error)?;
                    writer
                        .write_event(Event::End(BytesEnd::new("error")))
                        .map_err(xml_error)?;
                }
                Severity::Warning => {
                    let mut failure_el = BytesStart::new("failure");
                    failure_el.push_attribute(("type", "warning"));
                    failure_el.push_attribute(("message", finding.message.as_str()));
                    writer
                        .write_event(Event::Start(failure_el))
                        .map_err(xml_error)?;
                    writer
                        .write_event(Event::Text(BytesText::new(&finding_body(finding))))
                        .map_err(xml_error)?;
                    writer
                        .write_event(Event::End(BytesEnd::new("failure")))
                        .map_err(xml_error)?;
                }
                Severity::Info => {
                    writer
                        .write_event(Event::Start(BytesStart::new("system-out")))
                        .map_err(xml_error)?;
                    writer
                        .write_event(Event::Text(BytesText::new(&finding_body(finding))))
                        .map_err(xml_error)?;
                    writer
                        .write_event(Event::End(BytesEnd::new("system-out")))
                        .map_err(xml_error)?;
                }
            }

            writer
                .write_event(Event::End(BytesEnd::new("testcase")))
                .map_err(xml_error)?;
        }

        writer
            .write_event(Event::End(BytesEnd::new("testsuite")))
            .map_err(xml_error)?;
    }

    writer
        .write_event(Event::End(BytesEnd::new("testsuites")))
        .map_err(xml_error)?;

    let bytes = writer.into_inner().into_inner();
    String::from_utf8(bytes).map_err(|e| {
        RepoLensError::Action(ActionError::ExecutionFailed {
            message: format!("JUnit XML produced non-UTF-8 bytes: {}", e),
        })
    })
}

/// Build the textual body inserted inside `<error>`, `<failure>` or
/// `<system-out>`. Combines the finding's message with optional
/// description, location and remediation hints.
fn finding_body(finding: &Finding) -> String {
    let mut body = String::new();
    body.push_str(&format!("{}: {}", finding.rule_id, finding.message));
    if let Some(loc) = &finding.location {
        body.push_str(&format!("\nLocation: {}", loc));
    }
    if let Some(desc) = &finding.description {
        body.push_str(&format!("\nDescription: {}", desc));
    }
    if let Some(rem) = &finding.remediation {
        body.push_str(&format!("\nRemediation: {}", rem));
    }
    body
}

impl OutputRenderer for JunitReport {
    fn render_plan(
        &self,
        results: &AuditResults,
        _plan: &ActionPlan,
    ) -> Result<String, RepoLensError> {
        render_findings(results.findings())
    }
}

impl ReportRenderer for JunitReport {
    fn render_report(&self, results: &AuditResults) -> Result<String, RepoLensError> {
        render_findings(results.findings())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::results::Finding;

    fn fixture_results() -> AuditResults {
        let mut results = AuditResults::new("test-repo", "opensource");
        results.add_finding(
            Finding::new("SEC001", "secrets", Severity::Critical, "Secret exposed")
                .with_location("src/config.rs:42"),
        );
        results.add_finding(
            Finding::new("DOC001", "docs", Severity::Warning, "README missing")
                .with_location("README.md"),
        );
        results.add_finding(Finding::new(
            "INFO001",
            "quality",
            Severity::Info,
            "Consider adding tests",
        ));
        results
    }

    #[test]
    fn test_testcase_name_with_location() {
        let f = Finding::new("SEC001", "secrets", Severity::Critical, "msg")
            .with_location("src/a.rs:10");
        assert_eq!(testcase_name(&f), "SEC001 [src/a.rs:10]");
    }

    #[test]
    fn test_testcase_name_without_location() {
        let f = Finding::new("SEC001", "secrets", Severity::Critical, "msg");
        assert_eq!(testcase_name(&f), "SEC001");
    }

    #[test]
    fn test_render_plan_includes_decl_and_root() {
        let renderer = JunitReport::new();
        let results = fixture_results();
        let plan = ActionPlan::new();
        let xml = renderer.render_plan(&results, &plan).unwrap();
        assert!(xml.starts_with("<?xml"));
        assert!(xml.contains("<testsuites"));
        assert!(xml.contains("</testsuites>"));
    }

    #[test]
    fn test_render_report_matches_render_plan() {
        let renderer = JunitReport::new();
        let results = fixture_results();
        let plan = ActionPlan::new();
        let from_plan = renderer.render_plan(&results, &plan).unwrap();
        let from_report = renderer.render_report(&results).unwrap();
        assert_eq!(from_plan, from_report);
    }

    #[test]
    fn test_severity_mapping() {
        let renderer = JunitReport::new();
        let xml = renderer.render_report(&fixture_results()).unwrap();
        assert!(xml.contains("<error type=\"critical\""));
        assert!(xml.contains("<failure type=\"warning\""));
        assert!(xml.contains("<system-out>"));
    }

    #[test]
    fn test_counters_at_top_level() {
        let renderer = JunitReport::new();
        let xml = renderer.render_report(&fixture_results()).unwrap();
        assert!(xml.contains("tests=\"3\""));
        assert!(xml.contains("failures=\"1\""));
        assert!(xml.contains("errors=\"1\""));
        assert!(xml.contains("skipped=\"0\""));
    }

    #[test]
    fn test_empty_results() {
        let renderer = JunitReport::new();
        let results = AuditResults::new("empty", "opensource");
        let xml = renderer.render_report(&results).unwrap();
        assert!(xml.contains("tests=\"0\""));
        assert!(xml.contains("failures=\"0\""));
        assert!(xml.contains("errors=\"0\""));
        assert!(xml.contains("skipped=\"0\""));
    }
}
