//! Integration tests for the JUnit XML output format.

use quick_xml::events::Event;
use quick_xml::Reader;

use repolens::actions::plan::ActionPlan;
use repolens::cli::output::{JunitReport, OutputRenderer, ReportRenderer};
use repolens::compare::{compare_results, format_junit};
use repolens::rules::results::{AuditResults, Finding, Severity};

fn mixed_results() -> AuditResults {
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

/// Parse the rendered XML and check it is well-formed.
/// Returns the final tag stack (should be empty if balanced).
fn assert_well_formed(xml: &str) {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut depth: i32 = 0;
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(_)) => depth += 1,
            Ok(Event::End(_)) => depth -= 1,
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(e) => panic!("XML parse error: {}", e),
        }
        buf.clear();
    }
    assert_eq!(depth, 0, "Unbalanced XML tags");
}

#[test]
fn test_mixed_fixture_emits_correct_elements_and_counters() {
    let renderer = JunitReport::new();
    let xml = renderer.render_report(&mixed_results()).unwrap();

    assert!(xml.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    assert!(xml.contains("<error type=\"critical\""));
    assert!(xml.contains("<failure type=\"warning\""));
    assert!(xml.contains("<system-out>"));

    // Top-level counters
    assert!(xml.contains("tests=\"3\""));
    assert!(xml.contains("failures=\"1\""));
    assert!(xml.contains("errors=\"1\""));
    assert!(xml.contains("skipped=\"0\""));

    // Each category becomes its own testsuite
    assert!(xml.contains("<testsuite name=\"secrets\""));
    assert!(xml.contains("<testsuite name=\"docs\""));
    assert!(xml.contains("<testsuite name=\"quality\""));

    // testcase name carries rule_id and location
    assert!(xml.contains("name=\"SEC001 [src/config.rs:42]\""));
    assert!(xml.contains("name=\"DOC001 [README.md]\""));
    assert!(xml.contains("name=\"INFO001\""));

    assert_well_formed(&xml);
}

#[test]
fn test_empty_results_produces_zero_counters() {
    let renderer = JunitReport::new();
    let results = AuditResults::new("empty", "opensource");
    let xml = renderer.render_report(&results).unwrap();

    assert!(xml.starts_with("<?xml"));
    assert!(xml.contains("tests=\"0\""));
    assert!(xml.contains("failures=\"0\""));
    assert!(xml.contains("errors=\"0\""));
    assert!(xml.contains("skipped=\"0\""));

    // No testsuite or testcase elements
    assert!(!xml.contains("<testsuite "));
    assert!(!xml.contains("<testcase "));

    assert_well_formed(&xml);
}

#[test]
fn test_special_characters_are_escaped_and_round_trip() {
    let renderer = JunitReport::new();
    let mut results = AuditResults::new("special", "opensource");
    results.add_finding(
        Finding::new(
            "X<&>1",
            "<weird>",
            Severity::Critical,
            "msg with <tag> & \"quotes\" 'apos'",
        )
        .with_location("path/with <bad> & \"chars\".rs:1"),
    );

    let xml = renderer.render_report(&results).unwrap();

    // Raw special chars must NOT leak unescaped into attribute or text positions.
    // Verify standard XML entities were emitted somewhere.
    assert!(xml.contains("&lt;") || xml.contains("&amp;"));
    assert!(!xml.contains("<tag>"));

    // Round-trip parse: must succeed and recover original message attribute.
    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut found_message = None;
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                if e.name().as_ref() == b"error" {
                    for attr in e.attributes().flatten() {
                        if attr.key.as_ref() == b"message" {
                            let value = attr.unescape_value().unwrap();
                            found_message = Some(value.into_owned());
                        }
                    }
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(err) => panic!("Round-trip parse failed: {}", err),
        }
        buf.clear();
    }

    assert_eq!(
        found_message.as_deref(),
        Some("msg with <tag> & \"quotes\" 'apos'")
    );
}

#[test]
fn test_render_plan_and_render_report_produce_identical_xml() {
    let renderer = JunitReport::new();
    let results = mixed_results();
    let plan = ActionPlan::new();

    let from_plan = renderer.render_plan(&results, &plan).unwrap();
    let from_report = renderer.render_report(&results).unwrap();

    assert_eq!(from_plan, from_report);
}

#[test]
fn test_compare_format_junit_only_includes_added_findings() {
    let mut base = AuditResults::new("repo", "opensource");
    base.add_finding(Finding::new(
        "OLD001",
        "files",
        Severity::Warning,
        "old issue (resolved in head)",
    ));

    let mut head = AuditResults::new("repo", "opensource");
    head.add_finding(Finding::new(
        "NEW001",
        "secrets",
        Severity::Critical,
        "regression",
    ));

    let report = compare_results(&base, &head, "base", "head");
    let xml = format_junit(&report).unwrap();

    // Regression appears
    assert!(xml.contains("NEW001"));
    assert!(xml.contains("regression"));
    assert!(xml.contains("<error type=\"critical\""));

    // Resolved finding is silent
    assert!(!xml.contains("OLD001"));
    assert!(!xml.contains("old issue"));

    // Counters reflect the single regression
    assert!(xml.contains("tests=\"1\""));
    assert!(xml.contains("errors=\"1\""));

    assert_well_formed(&xml);
}

#[test]
fn test_mixed_fixture_snapshot() {
    let renderer = JunitReport::new();
    let xml = renderer.render_report(&mixed_results()).unwrap();
    insta::assert_snapshot!("mixed_fixture", xml);
}
