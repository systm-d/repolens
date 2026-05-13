//! JSON output formatting with optional JSON Schema support

use crate::error::RepoLensError;
use serde::Serialize;
use std::collections::HashMap;

use super::{OutputRenderer, ReportRenderer};
use crate::actions::plan::ActionPlan;
use crate::rules::results::{AuditResults, Severity};

/// Embedded JSON Schema for the audit report.
pub const AUDIT_REPORT_SCHEMA: &str = include_str!("../../../schemas/audit-report.schema.json");

/// The schema URI used for the `$schema` field in JSON output.
pub const AUDIT_REPORT_SCHEMA_URI: &str =
    "https://github.com/systm-d/repolens/schemas/audit-report.schema.json";

pub struct JsonOutput {
    /// Whether to include the `$schema` reference in the output.
    include_schema: bool,
    /// Whether to validate the output against the JSON Schema before emitting.
    validate: bool,
}

impl JsonOutput {
    pub fn new() -> Self {
        Self {
            include_schema: false,
            validate: false,
        }
    }

    /// Enable the `$schema` reference field in the JSON output.
    pub fn with_schema(mut self, include_schema: bool) -> Self {
        self.include_schema = include_schema;
        self
    }

    /// Enable validation of the JSON output against the embedded schema.
    pub fn with_validation(mut self, validate: bool) -> Self {
        self.validate = validate;
        self
    }
}

impl Default for JsonOutput {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize)]
struct PlanOutput<'a> {
    version: &'static str,
    repository: &'a str,
    preset: &'a str,
    audit: AuditSummary<'a>,
    actions: Vec<ActionSummary<'a>>,
}

#[derive(Serialize)]
struct AuditSummary<'a> {
    critical_count: usize,
    warning_count: usize,
    info_count: usize,
    findings: &'a [crate::rules::results::Finding],
}

#[derive(Serialize)]
struct ActionSummary<'a> {
    category: &'a str,
    description: &'a str,
    details: &'a [String],
}

/// Enhanced report output with metadata, summary, and optional schema reference.
#[derive(Serialize)]
struct EnhancedReportOutput<'a> {
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    schema: Option<&'static str>,
    repository_name: &'a str,
    preset: &'a str,
    findings: &'a [crate::rules::results::Finding],
    metadata: ReportMetadata,
    summary: ReportSummary,
}

#[derive(Serialize)]
struct ReportMetadata {
    version: String,
    timestamp: String,
    schema_version: String,
}

#[derive(Serialize)]
struct ReportSummary {
    total: usize,
    by_severity: SeverityCounts,
    by_category: HashMap<String, usize>,
}

#[derive(Serialize)]
struct SeverityCounts {
    critical: usize,
    warning: usize,
    info: usize,
}

/// Validate a JSON value against the embedded audit report schema.
///
/// Returns `Ok(())` if the value conforms, or an error message otherwise.
pub fn validate_against_schema(value: &serde_json::Value) -> Result<(), String> {
    let schema_value: serde_json::Value =
        serde_json::from_str(AUDIT_REPORT_SCHEMA).map_err(|e| format!("Invalid schema: {e}"))?;

    let validator = jsonschema::validator_for(&schema_value)
        .map_err(|e| format!("Failed to compile schema: {e}"))?;

    let errors: Vec<String> = validator
        .iter_errors(value)
        .map(|e| format!("{} at {}", e, e.instance_path))
        .collect();

    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "JSON Schema validation failed:\n{}",
            errors.join("\n")
        ))
    }
}

fn build_category_counts(results: &AuditResults) -> HashMap<String, usize> {
    let mut by_category: HashMap<String, usize> = HashMap::new();
    for finding in results.findings() {
        *by_category.entry(finding.category.clone()).or_insert(0) += 1;
    }
    by_category
}

impl OutputRenderer for JsonOutput {
    fn render_plan(
        &self,
        results: &AuditResults,
        plan: &ActionPlan,
    ) -> Result<String, RepoLensError> {
        let output = PlanOutput {
            version: env!("CARGO_PKG_VERSION"),
            repository: &results.repository_name,
            preset: &results.preset,
            audit: AuditSummary {
                critical_count: results.count_by_severity(Severity::Critical),
                warning_count: results.count_by_severity(Severity::Warning),
                info_count: results.count_by_severity(Severity::Info),
                findings: results.findings(),
            },
            actions: plan
                .actions()
                .iter()
                .map(|a| ActionSummary {
                    category: a.category(),
                    description: a.description(),
                    details: a.details(),
                })
                .collect(),
        };

        Ok(serde_json::to_string_pretty(&output)?)
    }
}

impl ReportRenderer for JsonOutput {
    fn render_report(&self, results: &AuditResults) -> Result<String, RepoLensError> {
        let by_category = build_category_counts(results);

        let output = EnhancedReportOutput {
            schema: if self.include_schema {
                Some(AUDIT_REPORT_SCHEMA_URI)
            } else {
                None
            },
            repository_name: &results.repository_name,
            preset: &results.preset,
            findings: results.findings(),
            metadata: ReportMetadata {
                version: env!("CARGO_PKG_VERSION").to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                schema_version: "1.0.0".to_string(),
            },
            summary: ReportSummary {
                total: results.findings().len(),
                by_severity: SeverityCounts {
                    critical: results.count_by_severity(Severity::Critical),
                    warning: results.count_by_severity(Severity::Warning),
                    info: results.count_by_severity(Severity::Info),
                },
                by_category,
            },
        };

        let json_string = serde_json::to_string_pretty(&output)?;

        if self.validate {
            let value: serde_json::Value = serde_json::from_str(&json_string)?;
            validate_against_schema(&value).map_err(|msg| {
                RepoLensError::Rule(crate::error::RuleError::ExecutionFailed { message: msg })
            })?;
        }

        Ok(json_string)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::results::Finding;

    fn create_test_results() -> AuditResults {
        let mut results = AuditResults::new("test-repo", "opensource");
        results.add_finding(Finding::new(
            "SEC001",
            "secrets",
            Severity::Critical,
            "Secret exposed",
        ));
        results.add_finding(Finding::new(
            "DOC001",
            "docs",
            Severity::Warning,
            "README missing",
        ));
        results
    }

    #[test]
    fn test_json_output_new() {
        let _output = JsonOutput::new();
    }

    #[test]
    fn test_json_output_default() {
        let _output: JsonOutput = Default::default();
    }

    #[test]
    fn test_render_plan() {
        use crate::actions::plan::{Action, ActionOperation};

        let output = JsonOutput::new();
        let results = create_test_results();
        let mut plan = ActionPlan::new();
        plan.add(
            Action::new(
                "action1",
                "files",
                "Update gitignore",
                ActionOperation::UpdateGitignore {
                    entries: vec!["*.log".to_string()],
                },
            )
            .with_detail("Adding *.log"),
        );

        let rendered = output.render_plan(&results, &plan).unwrap();
        let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();

        assert_eq!(json["repository"], "test-repo");
        assert_eq!(json["preset"], "opensource");
        assert_eq!(json["audit"]["critical_count"], 1);
        assert_eq!(json["audit"]["warning_count"], 1);
        assert!(!json["actions"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_render_plan_empty() {
        let output = JsonOutput::new();
        let results = AuditResults::new("empty-repo", "strict");
        let plan = ActionPlan::new();

        let rendered = output.render_plan(&results, &plan).unwrap();
        let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();

        assert_eq!(json["repository"], "empty-repo");
        assert_eq!(json["preset"], "strict");
        assert_eq!(json["audit"]["critical_count"], 0);
        assert!(json["actions"].as_array().unwrap().is_empty());
    }

    #[test]
    fn test_render_report() {
        let output = JsonOutput::new();
        let results = create_test_results();

        let rendered = output.render_report(&results).unwrap();
        let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();

        assert_eq!(json["repository_name"], "test-repo");
        assert_eq!(json["preset"], "opensource");
        assert!(json["metadata"]["version"].is_string());
        assert!(json["metadata"]["timestamp"].is_string());
        assert_eq!(json["metadata"]["schema_version"], "1.0.0");
        assert_eq!(json["summary"]["total"], 2);
        assert_eq!(json["summary"]["by_severity"]["critical"], 1);
        assert_eq!(json["summary"]["by_severity"]["warning"], 1);
        assert_eq!(json["summary"]["by_severity"]["info"], 0);
    }

    #[test]
    fn test_render_report_with_schema() {
        let output = JsonOutput::new().with_schema(true);
        let results = create_test_results();

        let rendered = output.render_report(&results).unwrap();
        let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();

        assert_eq!(json["$schema"], AUDIT_REPORT_SCHEMA_URI);
    }

    #[test]
    fn test_render_report_without_schema() {
        let output = JsonOutput::new().with_schema(false);
        let results = create_test_results();

        let rendered = output.render_report(&results).unwrap();
        let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();

        assert!(json.get("$schema").is_none());
    }

    #[test]
    fn test_render_report_with_validation() {
        let output = JsonOutput::new().with_schema(true).with_validation(true);
        let results = create_test_results();

        let result = output.render_report(&results);
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_report_empty_with_validation() {
        let output = JsonOutput::new().with_schema(true).with_validation(true);
        let results = AuditResults::new("empty-repo", "opensource");

        let result = output.render_report(&results);
        assert!(result.is_ok());

        let json: serde_json::Value = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(json["summary"]["total"], 0);
        assert_eq!(json["summary"]["by_severity"]["critical"], 0);
    }

    #[test]
    fn test_validate_against_schema_valid() {
        let output = JsonOutput::new().with_schema(true);
        let results = create_test_results();

        let rendered = output.render_report(&results).unwrap();
        let value: serde_json::Value = serde_json::from_str(&rendered).unwrap();

        assert!(validate_against_schema(&value).is_ok());
    }

    #[test]
    fn test_validate_against_schema_invalid() {
        let invalid_json = serde_json::json!({
            "repository_name": 123,
            "preset": "unknown_preset",
            "findings": "not_an_array"
        });

        let result = validate_against_schema(&invalid_json);
        assert!(result.is_err());
    }

    #[test]
    fn test_embedded_schema_is_valid_json() {
        let result: Result<serde_json::Value, _> = serde_json::from_str(AUDIT_REPORT_SCHEMA);
        assert!(result.is_ok());
    }

    #[test]
    fn test_schema_has_expected_structure() {
        let schema: serde_json::Value = serde_json::from_str(AUDIT_REPORT_SCHEMA).unwrap();

        assert_eq!(schema["$schema"], "http://json-schema.org/draft-07/schema#");
        assert_eq!(schema["title"], "RepoLens Audit Report");
        assert!(schema["properties"]["repository_name"].is_object());
        assert!(schema["properties"]["preset"].is_object());
        assert!(schema["properties"]["findings"].is_object());
        assert!(schema["definitions"]["Finding"].is_object());
        assert!(schema["definitions"]["Severity"].is_object());
        assert!(schema["definitions"]["Metadata"].is_object());
        assert!(schema["definitions"]["Summary"].is_object());
    }

    #[test]
    fn test_report_summary_by_category() {
        let output = JsonOutput::new();
        let mut results = AuditResults::new("test-repo", "opensource");
        results.add_finding(Finding::new(
            "SEC001",
            "secrets",
            Severity::Critical,
            "Secret 1",
        ));
        results.add_finding(Finding::new(
            "SEC002",
            "secrets",
            Severity::Warning,
            "Secret 2",
        ));
        results.add_finding(Finding::new("DOC001", "docs", Severity::Info, "Doc issue"));

        let rendered = output.render_report(&results).unwrap();
        let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();

        assert_eq!(json["summary"]["by_category"]["secrets"], 2);
        assert_eq!(json["summary"]["by_category"]["docs"], 1);
    }

    #[test]
    fn test_with_schema_builder() {
        let output = JsonOutput::new().with_schema(true).with_validation(false);
        assert!(output.include_schema);
        assert!(!output.validate);
    }

    #[test]
    fn test_with_validation_builder() {
        let output = JsonOutput::new().with_schema(false).with_validation(true);
        assert!(!output.include_schema);
        assert!(output.validate);
    }

    #[test]
    fn test_render_report_all_presets() {
        for preset in &["opensource", "enterprise", "strict"] {
            let output = JsonOutput::new().with_schema(true).with_validation(true);
            let results = AuditResults::new("repo", *preset);

            let result = output.render_report(&results);
            assert!(
                result.is_ok(),
                "Failed for preset {preset}: {:?}",
                result.err()
            );
        }
    }

    #[test]
    fn test_schema_uri_constant() {
        assert!(AUDIT_REPORT_SCHEMA_URI.starts_with("https://"));
        assert!(AUDIT_REPORT_SCHEMA_URI.contains("repolens"));
    }

    #[test]
    fn test_build_category_counts() {
        let mut results = AuditResults::new("test-repo", "opensource");
        results.add_finding(Finding::new("A001", "secrets", Severity::Critical, "a"));
        results.add_finding(Finding::new("A002", "secrets", Severity::Warning, "b"));
        results.add_finding(Finding::new("B001", "docs", Severity::Info, "c"));

        let counts = build_category_counts(&results);
        assert_eq!(counts.get("secrets"), Some(&2));
        assert_eq!(counts.get("docs"), Some(&1));
        assert_eq!(counts.get("quality"), None);
    }

    #[test]
    fn test_build_category_counts_empty() {
        let results = AuditResults::new("test-repo", "opensource");
        let counts = build_category_counts(&results);
        assert!(counts.is_empty());
    }

    #[test]
    fn test_render_report_findings_have_correct_fields() {
        let output = JsonOutput::new();
        let mut results = AuditResults::new("test-repo", "opensource");
        results.add_finding(
            Finding::new("SEC001", "secrets", Severity::Critical, "Secret exposed")
                .with_location("src/config.rs:42")
                .with_description("A hardcoded API key was found")
                .with_remediation("Use environment variables"),
        );

        let rendered = output.render_report(&results).unwrap();
        let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();

        let finding = &json["findings"][0];
        assert_eq!(finding["rule_id"], "SEC001");
        assert_eq!(finding["category"], "secrets");
        assert_eq!(finding["severity"], "critical");
        assert_eq!(finding["message"], "Secret exposed");
        assert_eq!(finding["location"], "src/config.rs:42");
        assert_eq!(finding["description"], "A hardcoded API key was found");
        assert_eq!(finding["remediation"], "Use environment variables");
    }
}
