//! HTML report output

use crate::error::RepoLensError;
use chrono::Utc;

use super::ReportRenderer;
use crate::rules::results::{AuditResults, Severity};

pub struct HtmlReport {
    detailed: bool,
}

impl HtmlReport {
    pub fn new(detailed: bool) -> Self {
        Self { detailed }
    }
}

impl ReportRenderer for HtmlReport {
    fn render_report(&self, results: &AuditResults) -> Result<String, RepoLensError> {
        let critical_count = results.count_by_severity(Severity::Critical);
        let warning_count = results.count_by_severity(Severity::Warning);
        let info_count = results.count_by_severity(Severity::Info);

        let status_color = if critical_count > 0 {
            "#dc3545"
        } else if warning_count > 0 {
            "#ffc107"
        } else {
            "#28a745"
        };

        let mut findings_html = String::new();

        // Critical findings
        for finding in results.findings_by_severity(Severity::Critical) {
            findings_html.push_str(&format!(
                r#"<div class="finding critical">
                    <div class="finding-header">
                        <span class="badge critical">CRITICAL</span>
                        <span class="rule-id">{}</span>
                    </div>
                    <div class="finding-message">{}</div>
                    {}
                    {}
                </div>"#,
                finding.rule_id,
                finding.message,
                finding.location.as_ref().map_or(String::new(), |l| {
                    format!(r#"<div class="finding-location">Location: <code>{}</code></div>"#, l)
                }),
                if self.detailed {
                    format!(
                        "{}{}",
                        finding.description.as_ref().map_or(String::new(), |d| {
                            format!(r#"<div class="finding-description">{}</div>"#, d)
                        }),
                        finding.remediation.as_ref().map_or(String::new(), |r| {
                            format!(r#"<div class="finding-remediation"><strong>Remediation:</strong> {}</div>"#, r)
                        })
                    )
                } else {
                    String::new()
                }
            ));
        }

        // Warning findings
        for finding in results.findings_by_severity(Severity::Warning) {
            findings_html.push_str(&format!(
                r#"<div class="finding warning">
                    <div class="finding-header">
                        <span class="badge warning">WARNING</span>
                        <span class="rule-id">{}</span>
                    </div>
                    <div class="finding-message">{}</div>
                    {}
                </div>"#,
                finding.rule_id,
                finding.message,
                finding.location.as_ref().map_or(String::new(), |l| {
                    format!(
                        r#"<div class="finding-location">Location: <code>{}</code></div>"#,
                        l
                    )
                })
            ));
        }

        // Info findings
        if self.detailed {
            for finding in results.findings_by_severity(Severity::Info) {
                findings_html.push_str(&format!(
                    r#"<div class="finding info">
                        <div class="finding-header">
                            <span class="badge info">INFO</span>
                            <span class="rule-id">{}</span>
                        </div>
                        <div class="finding-message">{}</div>
                    </div>"#,
                    finding.rule_id, finding.message
                ));
            }
        }

        let html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>RepoLens Audit Report - {}</title>
    <style>
        :root {{
            --critical: #dc3545;
            --warning: #ffc107;
            --info: #17a2b8;
            --success: #28a745;
        }}
        * {{ box-sizing: border-box; margin: 0; padding: 0; }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            line-height: 1.6;
            color: #333;
            max-width: 1200px;
            margin: 0 auto;
            padding: 2rem;
            background: #f8f9fa;
        }}
        header {{
            background: white;
            padding: 2rem;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
            margin-bottom: 2rem;
        }}
        h1 {{ color: #333; margin-bottom: 1rem; }}
        .meta {{ color: #666; font-size: 0.9rem; }}
        .summary {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
            gap: 1rem;
            margin: 2rem 0;
        }}
        .stat {{
            background: white;
            padding: 1.5rem;
            border-radius: 8px;
            text-align: center;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }}
        .stat-value {{ font-size: 2rem; font-weight: bold; }}
        .stat-label {{ color: #666; font-size: 0.9rem; }}
        .stat.critical .stat-value {{ color: var(--critical); }}
        .stat.warning .stat-value {{ color: var(--warning); }}
        .stat.info .stat-value {{ color: var(--info); }}
        .status-indicator {{
            display: inline-block;
            width: 12px;
            height: 12px;
            border-radius: 50%;
            margin-right: 8px;
        }}
        .findings {{ margin-top: 2rem; }}
        .finding {{
            background: white;
            padding: 1.5rem;
            border-radius: 8px;
            margin-bottom: 1rem;
            border-left: 4px solid;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }}
        .finding.critical {{ border-color: var(--critical); }}
        .finding.warning {{ border-color: var(--warning); }}
        .finding.info {{ border-color: var(--info); }}
        .finding-header {{ display: flex; align-items: center; gap: 1rem; margin-bottom: 0.5rem; }}
        .badge {{
            padding: 0.25rem 0.5rem;
            border-radius: 4px;
            font-size: 0.75rem;
            font-weight: bold;
            color: white;
        }}
        .badge.critical {{ background: var(--critical); }}
        .badge.warning {{ background: var(--warning); color: #333; }}
        .badge.info {{ background: var(--info); }}
        .rule-id {{ color: #666; font-family: monospace; }}
        .finding-message {{ font-weight: 500; }}
        .finding-location {{ margin-top: 0.5rem; color: #666; font-size: 0.9rem; }}
        .finding-location code {{ background: #f1f1f1; padding: 0.2rem 0.4rem; border-radius: 4px; }}
        .finding-description {{ margin-top: 0.5rem; color: #555; }}
        .finding-remediation {{ margin-top: 0.5rem; color: #28a745; }}
        footer {{
            margin-top: 3rem;
            text-align: center;
            color: #666;
            font-size: 0.9rem;
        }}
        footer a {{ color: #007bff; text-decoration: none; }}
    </style>
</head>
<body>
    <header>
        <h1>
            <span class="status-indicator" style="background: {}"></span>
            RepoLens Audit Report
        </h1>
        <div class="meta">
            <p><strong>Repository:</strong> {}</p>
            <p><strong>Preset:</strong> {}</p>
            <p><strong>Generated:</strong> {}</p>
            <p><strong>Version:</strong> {}</p>
        </div>
    </header>

    <section class="summary">
        <div class="stat critical">
            <div class="stat-value">{}</div>
            <div class="stat-label">Critical</div>
        </div>
        <div class="stat warning">
            <div class="stat-value">{}</div>
            <div class="stat-label">Warnings</div>
        </div>
        <div class="stat info">
            <div class="stat-value">{}</div>
            <div class="stat-label">Info</div>
        </div>
    </section>

    <section class="findings">
        <h2>Findings</h2>
        {}
    </section>

    <footer>
        <p>Report generated by <a href="https://github.com/systm-d/repolens">RepoLens</a></p>
    </footer>
</body>
</html>"#,
            results.repository_name,
            status_color,
            results.repository_name,
            results.preset,
            Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
            env!("CARGO_PKG_VERSION"),
            critical_count,
            warning_count,
            info_count,
            findings_html
        );

        Ok(html)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::results::Finding;

    fn create_test_results() -> AuditResults {
        let mut results = AuditResults::new("test-repo", "opensource");
        results.add_finding(
            Finding::new("SEC001", "secrets", Severity::Critical, "Secret exposed")
                .with_location("src/config.rs:42")
                .with_description("A secret was found in the code")
                .with_remediation("Move the secret to environment variables"),
        );
        results.add_finding(
            Finding::new("DOC001", "docs", Severity::Warning, "README missing")
                .with_location("README.md"),
        );
        results.add_finding(Finding::new(
            "INFO001",
            "info",
            Severity::Info,
            "Consider adding tests",
        ));
        results
    }

    #[test]
    fn test_html_report_new() {
        let report = HtmlReport::new(false);
        assert!(!report.detailed);
    }

    #[test]
    fn test_render_report_simple() {
        let report = HtmlReport::new(false);
        let results = create_test_results();
        let rendered = report.render_report(&results).unwrap();

        assert!(rendered.contains("<!DOCTYPE html>"));
        assert!(rendered.contains("<title>RepoLens Audit Report - test-repo</title>"));
        assert!(rendered.contains("<strong>Repository:</strong> test-repo"));
        assert!(rendered.contains("<strong>Preset:</strong> opensource"));
        assert!(rendered.contains("SEC001"));
        assert!(rendered.contains("Secret exposed"));
        assert!(rendered.contains("DOC001"));
        assert!(rendered.contains("README missing"));
    }

    #[test]
    fn test_render_report_detailed() {
        let report = HtmlReport::new(true);
        let results = create_test_results();
        let rendered = report.render_report(&results).unwrap();

        // Should include descriptions and remediations for critical
        assert!(rendered.contains("A secret was found in the code"));
        assert!(rendered.contains("<strong>Remediation:</strong>"));

        // Should include info findings when detailed
        assert!(rendered.contains("INFO001"));
        assert!(rendered.contains("Consider adding tests"));
    }

    #[test]
    fn test_render_report_status_colors() {
        // Test critical status (red)
        let report = HtmlReport::new(false);
        let mut results = AuditResults::new("test-repo", "opensource");
        results.add_finding(Finding::new(
            "SEC001",
            "secrets",
            Severity::Critical,
            "Critical",
        ));
        let rendered = report.render_report(&results).unwrap();
        assert!(rendered.contains("#dc3545")); // Critical red

        // Test warning status (yellow)
        let mut results = AuditResults::new("test-repo", "opensource");
        results.add_finding(Finding::new("DOC001", "docs", Severity::Warning, "Warning"));
        let rendered = report.render_report(&results).unwrap();
        assert!(rendered.contains("#ffc107")); // Warning yellow

        // Test clean status (green)
        let results = AuditResults::new("clean-repo", "opensource");
        let rendered = report.render_report(&results).unwrap();
        assert!(rendered.contains("#28a745")); // Success green
    }

    #[test]
    fn test_render_report_contains_css() {
        let report = HtmlReport::new(false);
        let results = AuditResults::new("test-repo", "opensource");
        let rendered = report.render_report(&results).unwrap();

        assert!(rendered.contains("<style>"));
        assert!(rendered.contains("--critical: #dc3545"));
        assert!(rendered.contains("--warning: #ffc107"));
        assert!(rendered.contains("--info: #17a2b8"));
    }

    #[test]
    fn test_render_report_contains_footer() {
        let report = HtmlReport::new(false);
        let results = AuditResults::new("test-repo", "opensource");
        let rendered = report.render_report(&results).unwrap();

        assert!(rendered.contains("<footer>"));
        assert!(rendered.contains("Report generated by"));
        assert!(rendered.contains("https://github.com/systm-d/repolens"));
    }

    #[test]
    fn test_render_report_finding_with_location() {
        let report = HtmlReport::new(false);
        let mut results = AuditResults::new("test-repo", "opensource");
        results.add_finding(
            Finding::new("SEC001", "secrets", Severity::Critical, "Issue")
                .with_location("src/file.rs:10"),
        );
        let rendered = report.render_report(&results).unwrap();

        assert!(rendered.contains("Location:"));
        assert!(rendered.contains("<code>src/file.rs:10</code>"));
    }
}
