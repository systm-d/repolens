//! Compare command - Compare two audit report JSON files

use colored::Colorize;
use std::path::PathBuf;

use super::CompareArgs;
use crate::compare::{compare_results, format_json, format_markdown, format_terminal};
use crate::error::RepoLensError;
use crate::exit_codes;
use crate::rules::results::AuditResults;

/// Load an AuditResults from a JSON file
fn load_report(path: &PathBuf) -> Result<AuditResults, RepoLensError> {
    let content = std::fs::read_to_string(path).map_err(|e| {
        RepoLensError::Action(crate::error::ActionError::ExecutionFailed {
            message: format!("Failed to read report file '{}': {}", path.display(), e),
        })
    })?;
    let results: AuditResults = serde_json::from_str(&content)?;
    Ok(results)
}

pub async fn execute(args: CompareArgs) -> Result<i32, RepoLensError> {
    // Load base and head reports
    let base_results = load_report(&args.base_file)?;
    let head_results = load_report(&args.head_file)?;

    let base_label = args
        .base_file
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "base".to_string());
    let head_label = args
        .head_file
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "head".to_string());

    let report = compare_results(&base_results, &head_results, &base_label, &head_label);

    // Format output
    let output_str = match args.format {
        super::CompareFormat::Terminal => format_terminal(&report),
        super::CompareFormat::Json => format_json(&report).map_err(|e| {
            RepoLensError::Action(crate::error::ActionError::ExecutionFailed {
                message: format!("Failed to serialize compare report: {}", e),
            })
        })?,
        super::CompareFormat::Markdown => format_markdown(&report),
    };

    // Write output
    if let Some(output_path) = &args.output {
        std::fs::write(output_path, &output_str).map_err(|e| {
            RepoLensError::Action(crate::error::ActionError::FileWrite {
                path: output_path.display().to_string(),
                source: e,
            })
        })?;
        println!(
            "{} Comparison report written to: {}",
            "Success:".green().bold(),
            output_path.display().to_string().cyan()
        );
    } else {
        print!("{}", output_str);
    }

    // Determine exit code
    if args.fail_on_regression && report.has_regressions() {
        eprintln!(
            "{} {} new issue(s) detected (regression).",
            "Error:".red().bold(),
            report.added_findings.len()
        );
        Ok(exit_codes::CRITICAL_ISSUES)
    } else {
        Ok(exit_codes::SUCCESS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::results::{Finding, Severity};
    use tempfile::TempDir;

    fn create_test_report(findings: Vec<Finding>) -> AuditResults {
        let mut results = AuditResults::new("test-repo", "opensource");
        for finding in findings {
            results.add_finding(finding);
        }
        results
    }

    fn write_report_to_file(report: &AuditResults, path: &PathBuf) {
        let json = serde_json::to_string_pretty(report).unwrap();
        std::fs::write(path, json).unwrap();
    }

    #[tokio::test]
    async fn test_execute_no_regression() {
        let temp_dir = TempDir::new().unwrap();

        // Create base report with one finding
        let base_report = create_test_report(vec![Finding::new(
            "TEST001",
            "test",
            Severity::Warning,
            "Test finding",
        )]);
        let base_path = temp_dir.path().join("base.json");
        write_report_to_file(&base_report, &base_path);

        // Create head report with same finding (no regression)
        let head_report = create_test_report(vec![Finding::new(
            "TEST001",
            "test",
            Severity::Warning,
            "Test finding",
        )]);
        let head_path = temp_dir.path().join("head.json");
        write_report_to_file(&head_report, &head_path);

        let args = CompareArgs {
            base_file: base_path,
            head_file: head_path,
            format: super::super::CompareFormat::Terminal,
            output: None,
            fail_on_regression: true,
        };

        let result = execute(args).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), exit_codes::SUCCESS);
    }

    #[tokio::test]
    async fn test_execute_with_regression() {
        let temp_dir = TempDir::new().unwrap();

        // Create base report with no findings
        let base_report = create_test_report(vec![]);
        let base_path = temp_dir.path().join("base.json");
        write_report_to_file(&base_report, &base_path);

        // Create head report with one finding (regression)
        let head_report = create_test_report(vec![Finding::new(
            "TEST001",
            "test",
            Severity::Warning,
            "New test finding",
        )]);
        let head_path = temp_dir.path().join("head.json");
        write_report_to_file(&head_report, &head_path);

        let args = CompareArgs {
            base_file: base_path,
            head_file: head_path,
            format: super::super::CompareFormat::Terminal,
            output: None,
            fail_on_regression: true,
        };

        let result = execute(args).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), exit_codes::CRITICAL_ISSUES);
    }

    #[tokio::test]
    async fn test_execute_regression_without_fail_flag() {
        let temp_dir = TempDir::new().unwrap();

        // Create base report with no findings
        let base_report = create_test_report(vec![]);
        let base_path = temp_dir.path().join("base.json");
        write_report_to_file(&base_report, &base_path);

        // Create head report with one finding (regression)
        let head_report = create_test_report(vec![Finding::new(
            "TEST001",
            "test",
            Severity::Warning,
            "New test finding",
        )]);
        let head_path = temp_dir.path().join("head.json");
        write_report_to_file(&head_report, &head_path);

        let args = CompareArgs {
            base_file: base_path,
            head_file: head_path,
            format: super::super::CompareFormat::Terminal,
            output: None,
            fail_on_regression: false, // Don't fail on regression
        };

        let result = execute(args).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), exit_codes::SUCCESS);
    }

    #[tokio::test]
    async fn test_execute_json_format() {
        let temp_dir = TempDir::new().unwrap();

        let base_report = create_test_report(vec![]);
        let base_path = temp_dir.path().join("base.json");
        write_report_to_file(&base_report, &base_path);

        let head_report = create_test_report(vec![]);
        let head_path = temp_dir.path().join("head.json");
        write_report_to_file(&head_report, &head_path);

        let output_path = temp_dir.path().join("compare.json");

        let args = CompareArgs {
            base_file: base_path,
            head_file: head_path,
            format: super::super::CompareFormat::Json,
            output: Some(output_path.clone()),
            fail_on_regression: false,
        };

        let result = execute(args).await;
        assert!(result.is_ok());

        // Verify output is valid JSON
        let content = std::fs::read_to_string(&output_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed.is_object());
    }

    #[tokio::test]
    async fn test_execute_markdown_format() {
        let temp_dir = TempDir::new().unwrap();

        let base_report = create_test_report(vec![]);
        let base_path = temp_dir.path().join("base.json");
        write_report_to_file(&base_report, &base_path);

        let head_report = create_test_report(vec![]);
        let head_path = temp_dir.path().join("head.json");
        write_report_to_file(&head_report, &head_path);

        let output_path = temp_dir.path().join("compare.md");

        let args = CompareArgs {
            base_file: base_path,
            head_file: head_path,
            format: super::super::CompareFormat::Markdown,
            output: Some(output_path.clone()),
            fail_on_regression: false,
        };

        let result = execute(args).await;
        assert!(result.is_ok());

        // Verify output contains markdown
        let content = std::fs::read_to_string(&output_path).unwrap();
        assert!(content.contains("#") || content.contains("Comparison"));
    }

    #[tokio::test]
    async fn test_execute_missing_base_file() {
        let temp_dir = TempDir::new().unwrap();

        let head_report = create_test_report(vec![]);
        let head_path = temp_dir.path().join("head.json");
        write_report_to_file(&head_report, &head_path);

        let args = CompareArgs {
            base_file: temp_dir.path().join("nonexistent.json"),
            head_file: head_path,
            format: super::super::CompareFormat::Terminal,
            output: None,
            fail_on_regression: false,
        };

        let result = execute(args).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_load_report_valid() {
        let temp_dir = TempDir::new().unwrap();
        let report = create_test_report(vec![]);
        let path = temp_dir.path().join("report.json");
        write_report_to_file(&report, &path);

        let loaded = load_report(&path);
        assert!(loaded.is_ok());
    }

    #[test]
    fn test_load_report_invalid_json() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("invalid.json");
        std::fs::write(&path, "not valid json {{{").unwrap();

        let loaded = load_report(&path);
        assert!(loaded.is_err());
    }

    #[test]
    fn test_load_report_missing_file() {
        let path = PathBuf::from("/nonexistent/file.json");
        let loaded = load_report(&path);
        assert!(loaded.is_err());
    }
}
