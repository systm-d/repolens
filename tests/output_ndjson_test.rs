//! Integration tests for `NdjsonOutput`.

use repolens::cli::output::{NdjsonOutput, ReportRenderer};
use repolens::rules::results::{AuditResults, Finding, Severity};

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
fn single_finding_emits_one_lf_terminated_json_line_with_all_fields() {
    let r = results_with(vec![Finding::new(
        "SEC001",
        "secrets",
        Severity::Critical,
        "Secret exposed",
    )
    .with_location("src/config.rs:42")
    .with_description("hardcoded API key")
    .with_remediation("use env vars")]);

    let out = NdjsonOutput::new().render_report(&r).unwrap();
    assert!(out.ends_with('\n'), "output should end with LF");

    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines.len(), 1);

    let v: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(v["rule_id"], "SEC001");
    assert_eq!(v["category"], "secrets");
    assert_eq!(v["severity"], "critical");
    assert_eq!(v["file"], "src/config.rs");
    assert_eq!(v["line"], 42);
    assert!(v["column"].is_null());
    assert_eq!(v["message"], "Secret exposed");
    assert_eq!(v["description"], "hardcoded API key");
    assert_eq!(v["remediation"], "use env vars");
    assert_eq!(v["project"], "test-repo");
}

#[test]
fn null_fields_for_absent_location_description_remediation() {
    let r = results_with(vec![Finding::new(
        "DOC001",
        "docs",
        Severity::Warning,
        "README missing",
    )]);

    let out = NdjsonOutput::new().render_report(&r).unwrap();
    let line = out.lines().next().unwrap();
    let v: serde_json::Value = serde_json::from_str(line).unwrap();

    assert!(
        v["file"].is_null(),
        "file should be null, got {:?}",
        v["file"]
    );
    assert!(
        v["line"].is_null(),
        "line should be null, got {:?}",
        v["line"]
    );
    assert!(v["column"].is_null());
    assert!(v["description"].is_null());
    assert!(v["remediation"].is_null());

    // Ensure they are NOT empty strings (the bug we're guarding against).
    assert_ne!(v["file"], serde_json::Value::String(String::new()));
    assert_ne!(v["description"], serde_json::Value::String(String::new()));
    assert_ne!(v["remediation"], serde_json::Value::String(String::new()));
}

#[test]
fn lf_separator_no_crlf() {
    let r = results_with(vec![
        Finding::new("A001", "secrets", Severity::Critical, "a"),
        Finding::new("B001", "docs", Severity::Warning, "b"),
        Finding::new("C001", "quality", Severity::Info, "c"),
    ]);
    let out = NdjsonOutput::new().render_report(&r).unwrap();
    assert!(!out.contains('\r'), "NDJSON must use LF only, no CR found");
    // Three findings → three LF terminators
    assert_eq!(out.matches('\n').count(), 3);
}

#[test]
fn each_line_is_independently_valid_json() {
    let r = results_with(vec![
        Finding::new("A001", "secrets", Severity::Critical, "a"),
        Finding::new("B001", "docs", Severity::Warning, "b"),
    ]);
    let out = NdjsonOutput::new().render_report(&r).unwrap();
    for line in out.lines() {
        let parsed: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("line is not valid JSON: {e}\nline: {line}"));
        assert!(parsed.is_object());
    }
}

#[test]
fn empty_findings_produces_empty_output() {
    let out = NdjsonOutput::new().render_report(&empty_results()).unwrap();
    assert_eq!(out, "");
}

#[cfg(target_os = "linux")]
fn current_rss_bytes() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            // Format: "VmRSS:  12345 kB"
            let kb: u64 = rest.split_whitespace().next()?.parse().ok()?;
            return Some(kb * 1024);
        }
    }
    None
}

#[cfg(target_os = "linux")]
#[test]
fn render_10_000_findings_stays_within_50_mib_rss() {
    const FINDINGS: usize = 10_000;

    let mut results = AuditResults::new("perf-repo", "opensource");
    for i in 0..FINDINGS {
        results.add_finding(
            Finding::new(
                format!("SEC{i:04}"),
                "secrets",
                Severity::Critical,
                format!("finding number {i}"),
            )
            .with_location(format!("src/file{i}.rs:{}", i % 200))
            .with_description("a description for the finding")
            .with_remediation("a remediation suggestion"),
        );
    }

    let baseline = current_rss_bytes().expect("VmRSS is available on Linux");

    let out = NdjsonOutput::new().render_report(&results).unwrap();

    let after = current_rss_bytes().expect("VmRSS is available on Linux");

    let line_count = out.matches('\n').count();
    assert_eq!(line_count, FINDINGS);

    // The acceptance criterion is "peak memory ≤ 50 MB". We approximate peak by
    // the post-render RSS — sufficient because the renderer only allocates the
    // output `String`. Compare the *delta* against 50 MiB so the test isn't
    // tripped up by the steady-state RSS of the test harness.
    let delta = after.saturating_sub(baseline);
    let limit_bytes: u64 = 50 * 1024 * 1024;
    assert!(
        delta < limit_bytes,
        "RSS delta {delta} bytes exceeds 50 MiB ({limit_bytes} bytes); baseline={baseline} after={after}"
    );
}
