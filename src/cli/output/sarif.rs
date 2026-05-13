//! SARIF output formatting for GitHub Code Scanning integration

use crate::error::RepoLensError;
use serde::Serialize;

use super::OutputRenderer;
use crate::actions::plan::ActionPlan;
use crate::rules::results::{AuditResults, Finding, Severity};

pub struct SarifOutput;

impl SarifOutput {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SarifOutput {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Serialize)]
struct SarifReport {
    #[serde(rename = "$schema")]
    schema: &'static str,
    version: &'static str,
    runs: Vec<SarifRun>,
}

#[derive(Serialize)]
struct SarifRun {
    tool: SarifTool,
    results: Vec<SarifResult>,
}

#[derive(Serialize)]
struct SarifTool {
    driver: SarifDriver,
}

#[derive(Serialize)]
struct SarifDriver {
    name: &'static str,
    version: &'static str,
    #[serde(rename = "informationUri")]
    information_uri: &'static str,
    rules: Vec<SarifRule>,
}

#[derive(Serialize)]
struct SarifRule {
    id: String,
    name: String,
    #[serde(rename = "shortDescription")]
    short_description: SarifMessage,
    #[serde(rename = "defaultConfiguration")]
    default_configuration: SarifDefaultConfig,
}

#[derive(Serialize)]
struct SarifDefaultConfig {
    level: String,
}

#[derive(Serialize)]
struct SarifResult {
    #[serde(rename = "ruleId")]
    rule_id: String,
    level: String,
    message: SarifMessage,
    locations: Vec<SarifLocation>,
}

#[derive(Serialize)]
struct SarifMessage {
    text: String,
}

#[derive(Serialize)]
struct SarifLocation {
    #[serde(rename = "physicalLocation")]
    physical_location: SarifPhysicalLocation,
}

#[derive(Serialize)]
struct SarifPhysicalLocation {
    #[serde(rename = "artifactLocation")]
    artifact_location: SarifArtifactLocation,
    #[serde(skip_serializing_if = "Option::is_none")]
    region: Option<SarifRegion>,
}

#[derive(Serialize)]
struct SarifArtifactLocation {
    uri: String,
}

#[derive(Serialize)]
struct SarifRegion {
    #[serde(rename = "startLine")]
    start_line: u32,
    #[serde(rename = "startColumn", skip_serializing_if = "Option::is_none")]
    start_column: Option<u32>,
}

impl SarifOutput {
    fn severity_to_level(severity: Severity) -> &'static str {
        match severity {
            Severity::Critical => "error",
            Severity::Warning => "warning",
            Severity::Info => "note",
        }
    }

    fn finding_to_result(finding: &Finding) -> SarifResult {
        let (uri, region) = if let Some(location) = &finding.location {
            // Parse location like "src/config.ts:42"
            let parts: Vec<&str> = location.split(':').collect();
            let uri = parts.first().unwrap_or(&"unknown").to_string();
            let region = parts.get(1).and_then(|line| {
                line.parse::<u32>().ok().map(|l| SarifRegion {
                    start_line: l,
                    start_column: None,
                })
            });
            (uri, region)
        } else {
            ("unknown".to_string(), None)
        };

        SarifResult {
            rule_id: finding.rule_id.clone(),
            level: Self::severity_to_level(finding.severity).to_string(),
            message: SarifMessage {
                text: finding.message.clone(),
            },
            locations: vec![SarifLocation {
                physical_location: SarifPhysicalLocation {
                    artifact_location: SarifArtifactLocation { uri },
                    region,
                },
            }],
        }
    }
}

impl OutputRenderer for SarifOutput {
    fn render_plan(
        &self,
        results: &AuditResults,
        _plan: &ActionPlan,
    ) -> Result<String, RepoLensError> {
        let rules: Vec<SarifRule> = results
            .findings()
            .iter()
            .map(|f| SarifRule {
                id: f.rule_id.clone(),
                name: f.rule_id.clone(),
                short_description: SarifMessage {
                    text: f.message.clone(),
                },
                default_configuration: SarifDefaultConfig {
                    level: Self::severity_to_level(f.severity).to_string(),
                },
            })
            .collect();

        let results_sarif: Vec<SarifResult> = results
            .findings()
            .iter()
            .map(Self::finding_to_result)
            .collect();

        let report = SarifReport {
            schema: "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
            version: "2.1.0",
            runs: vec![SarifRun {
                tool: SarifTool {
                    driver: SarifDriver {
                        name: "repolens",
                        version: env!("CARGO_PKG_VERSION"),
                        information_uri: "https://github.com/systm-d/repolens",
                        rules,
                    },
                },
                results: results_sarif,
            }],
        };

        Ok(serde_json::to_string_pretty(&report)?)
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
                .with_location("src/config.rs:42"),
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
    fn test_sarif_output_new() {
        let _output = SarifOutput::new();
        // SarifOutput is a unit struct
    }

    #[test]
    fn test_sarif_output_default() {
        let _output: SarifOutput = Default::default();
        // Verify Default trait impl works
    }

    #[test]
    fn test_severity_to_level() {
        assert_eq!(SarifOutput::severity_to_level(Severity::Critical), "error");
        assert_eq!(SarifOutput::severity_to_level(Severity::Warning), "warning");
        assert_eq!(SarifOutput::severity_to_level(Severity::Info), "note");
    }

    #[test]
    fn test_finding_to_result_with_location() {
        let finding = Finding::new("SEC001", "secrets", Severity::Critical, "Secret exposed")
            .with_location("src/config.rs:42");

        let result = SarifOutput::finding_to_result(&finding);

        assert_eq!(result.rule_id, "SEC001");
        assert_eq!(result.level, "error");
        assert_eq!(result.message.text, "Secret exposed");
        assert_eq!(result.locations.len(), 1);
        assert_eq!(
            result.locations[0].physical_location.artifact_location.uri,
            "src/config.rs"
        );
        assert!(result.locations[0].physical_location.region.is_some());
        assert_eq!(
            result.locations[0]
                .physical_location
                .region
                .as_ref()
                .unwrap()
                .start_line,
            42
        );
    }

    #[test]
    fn test_finding_to_result_without_location() {
        let finding = Finding::new("DOC001", "docs", Severity::Warning, "README missing");

        let result = SarifOutput::finding_to_result(&finding);

        assert_eq!(result.rule_id, "DOC001");
        assert_eq!(result.level, "warning");
        assert_eq!(
            result.locations[0].physical_location.artifact_location.uri,
            "unknown"
        );
        assert!(result.locations[0].physical_location.region.is_none());
    }

    #[test]
    fn test_finding_to_result_location_without_line() {
        let finding = Finding::new("DOC001", "docs", Severity::Warning, "README missing")
            .with_location("README.md");

        let result = SarifOutput::finding_to_result(&finding);

        assert_eq!(
            result.locations[0].physical_location.artifact_location.uri,
            "README.md"
        );
        assert!(result.locations[0].physical_location.region.is_none());
    }

    #[test]
    fn test_render_plan() {
        let output = SarifOutput::new();
        let results = create_test_results();
        let plan = ActionPlan::new();

        let rendered = output.render_plan(&results, &plan).unwrap();
        let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();

        assert_eq!(json["version"], "2.1.0");
        assert!(json["$schema"].as_str().unwrap().contains("sarif-schema"));
        assert_eq!(json["runs"][0]["tool"]["driver"]["name"], "repolens");
        assert!(json["runs"][0]["results"].as_array().unwrap().len() == 3);
    }

    #[test]
    fn test_render_plan_empty() {
        let output = SarifOutput::new();
        let results = AuditResults::new("empty-repo", "opensource");
        let plan = ActionPlan::new();

        let rendered = output.render_plan(&results, &plan).unwrap();
        let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();

        assert!(json["runs"][0]["results"].as_array().unwrap().is_empty());
        assert!(
            json["runs"][0]["tool"]["driver"]["rules"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
}
