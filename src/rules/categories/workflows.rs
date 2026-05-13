//! GitHub Actions workflow rules
//!
//! This module provides rules for checking GitHub Actions workflows, including:
//! - Hardcoded secrets in workflow files
//! - Explicit permissions configuration
//! - Pinned action versions for security
//! - Job timeout configuration
//! - Concurrency controls
//! - Reusable workflow suggestions
//! - Artifact retention configuration
//! - pull_request_target security
//! - Linter integration in CI

use crate::error::RepoLensError;
use regex::Regex;

use crate::config::Config;
use crate::rules::engine::RuleCategory;
use crate::rules::results::{Finding, Severity};
use crate::scanner::Scanner;
use crate::utils::language_detection::{Language, detect_languages};

/// Rules for checking GitHub Actions workflows
pub struct WorkflowsRules;

#[async_trait::async_trait]
impl RuleCategory for WorkflowsRules {
    /// Get the category name
    fn name(&self) -> &'static str {
        "workflows"
    }

    /// Run all workflow-related rules
    ///
    /// # Arguments
    ///
    /// * `scanner` - The scanner to access repository files
    /// * `config` - The configuration with enabled rules
    ///
    /// # Returns
    ///
    /// A vector of findings for workflow issues
    async fn run(&self, scanner: &Scanner, config: &Config) -> Result<Vec<Finding>, RepoLensError> {
        let mut findings = Vec::new();

        // Check for workflows directory
        if !scanner.directory_exists(".github/workflows") {
            return Ok(findings);
        }

        // Check workflow security
        if config.is_rule_enabled("workflows/secrets") {
            findings.extend(check_workflow_secrets(scanner).await?);
        }

        // Check permissions
        if config.is_rule_enabled("workflows/permissions") {
            findings.extend(check_workflow_permissions(scanner).await?);
        }

        // Check pinned actions
        if config.is_rule_enabled("workflows/pinned-actions") {
            findings.extend(check_pinned_actions(scanner, config).await?);
        }

        // Check workflow timeout
        if config.is_rule_enabled("workflows/timeout") {
            findings.extend(check_workflow_timeout(scanner).await?);
        }

        // Check workflow concurrency
        if config.is_rule_enabled("workflows/concurrency") {
            findings.extend(check_workflow_concurrency(scanner).await?);
        }

        // Check reusable workflows
        if config.is_rule_enabled("workflows/reusable-workflows") {
            findings.extend(check_reusable_workflows(scanner).await?);
        }

        // Check artifacts retention
        if config.is_rule_enabled("workflows/artifacts-retention") {
            findings.extend(check_artifacts_retention(scanner).await?);
        }

        // Check pull_request_target security
        if config.is_rule_enabled("workflows/pull-request-target") {
            findings.extend(check_pull_request_target(scanner).await?);
        }

        // Check linters in CI
        if config.is_rule_enabled("workflows/linters-in-ci") {
            findings.extend(check_linters_in_ci(scanner).await?);
        }

        Ok(findings)
    }
}

/// Check for hardcoded secrets in workflow files
///
/// Detects patterns that suggest hardcoded passwords, tokens, API keys,
/// or secrets in GitHub Actions workflow files.
///
/// # Arguments
///
/// * `scanner` - The scanner to access repository files
///
/// # Returns
///
/// A vector of findings for hardcoded secrets in workflows
async fn check_workflow_secrets(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    // Patterns that suggest hardcoded secrets in workflows
    let secret_patterns = [
        (r#"password\s*:\s*['"][^'"]+['"]"#, "hardcoded password"),
        (r#"token\s*:\s*['"][^'"]+['"]"#, "hardcoded token"),
        (r#"api[_-]?key\s*:\s*['"][^'"]+['"]"#, "hardcoded API key"),
        (r#"secret\s*:\s*['"][^'"]+['"]"#, "hardcoded secret"),
    ];

    for file in scanner.files_in_directory(".github/workflows") {
        if !file.path.ends_with(".yml") && !file.path.ends_with(".yaml") {
            continue;
        }

        if let Ok(content) = scanner.read_file(&file.path) {
            for (pattern, description) in &secret_patterns {
                let regex = match Regex::new(pattern) {
                    Ok(r) => r,
                    Err(e) => {
                        tracing::warn!("Invalid regex pattern '{}': {}", pattern, e);
                        continue;
                    }
                };
                if regex.is_match(&content) {
                    // Find line number
                    let line_num = content
                        .lines()
                        .enumerate()
                        .find(|(_, line)| regex.is_match(line))
                        .map(|(i, _)| i + 1)
                        .unwrap_or(0);

                    findings.push(
                        Finding::new(
                            "WF001",
                            "workflows",
                            Severity::Critical,
                            format!("Potential {} in workflow", description),
                        )
                        .with_location(format!("{}:{}", file.path, line_num))
                        .with_description("Secrets should never be hardcoded in workflow files.")
                        .with_remediation(
                            "Use GitHub Secrets (secrets.SECRET_NAME) instead of hardcoded values.",
                        ),
                    );
                }
            }
        }
    }

    Ok(findings)
}

/// Check for explicit permissions in workflow files
///
/// Verifies that workflows define explicit permissions to follow
/// the principle of least privilege.
///
/// # Arguments
///
/// * `scanner` - The scanner to access repository files
///
/// # Returns
///
/// A vector of findings for missing permissions
async fn check_workflow_permissions(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    for file in scanner.files_in_directory(".github/workflows") {
        if !file.path.ends_with(".yml") && !file.path.ends_with(".yaml") {
            continue;
        }

        if let Ok(content) = scanner.read_file(&file.path) {
            // Check if permissions are defined
            if !content.contains("permissions:") {
                findings.push(
                    Finding::new(
                        "WF002",
                        "workflows",
                        Severity::Warning,
                        "Workflow missing explicit permissions",
                    )
                    .with_location(&file.path)
                    .with_description(
                        "Workflows without explicit permissions use the default permissions, which may be more permissive than necessary."
                    )
                    .with_remediation(
                        "Add a 'permissions:' block to explicitly define the minimum required permissions."
                    )
                );
            }
        }
    }

    Ok(findings)
}

/// Check for pinned action versions
///
/// In strict mode, verifies that actions are pinned to specific versions
/// instead of using @main, @master, or @latest.
///
/// # Arguments
///
/// * `scanner` - The scanner to access repository files
/// * `config` - The configuration (used to check preset)
///
/// # Returns
///
/// A vector of findings for unpinned actions
async fn check_pinned_actions(
    scanner: &Scanner,
    config: &Config,
) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    // Only check in strict mode
    if config.preset != "strict" {
        return Ok(findings);
    }

    let unpinned_patterns = [
        r"uses:\s+\S+@main\b",
        r"uses:\s+\S+@master\b",
        r"uses:\s+\S+@latest\b",
    ];

    for file in scanner.files_in_directory(".github/workflows") {
        if !file.path.ends_with(".yml") && !file.path.ends_with(".yaml") {
            continue;
        }

        if let Ok(content) = scanner.read_file(&file.path) {
            for pattern in &unpinned_patterns {
                let regex = match Regex::new(pattern) {
                    Ok(r) => r,
                    Err(e) => {
                        tracing::warn!("Invalid regex pattern '{}': {}", pattern, e);
                        continue;
                    }
                };
                for (line_num, line) in content.lines().enumerate() {
                    if regex.is_match(line) {
                        findings.push(
                            Finding::new(
                                "WF003",
                                "workflows",
                                Severity::Warning,
                                "Workflow uses unpinned action reference",
                            )
                            .with_location(format!("{}:{}", file.path, line_num + 1))
                            .with_description(
                                "Using @main, @master, or @latest for actions can introduce breaking changes or security vulnerabilities."
                            )
                            .with_remediation(
                                "Pin actions to a specific version tag (e.g., @v4) or commit SHA for maximum security."
                            )
                        );
                    }
                }
            }
        }
    }

    Ok(findings)
}

/// Check for job timeout configuration in workflow files
///
/// Jobs without `timeout-minutes` can run indefinitely, wasting resources.
///
/// # Arguments
///
/// * `scanner` - The scanner to access repository files
///
/// # Returns
///
/// A vector of findings for jobs missing timeout configuration
async fn check_workflow_timeout(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    for file in scanner.files_in_directory(".github/workflows") {
        if !file.path.ends_with(".yml") && !file.path.ends_with(".yaml") {
            continue;
        }

        if let Ok(content) = scanner.read_file(&file.path) {
            let mut in_jobs = false;
            let mut current_job: Option<String> = None;
            let mut job_has_timeout = false;

            for line in content.lines() {
                let trimmed = line.trim();

                // Detect the jobs: section
                if line == "jobs:" || line.starts_with("jobs:") {
                    in_jobs = true;
                    continue;
                }

                if !in_jobs {
                    continue;
                }

                // A top-level key under jobs (indented by exactly 2 spaces, no more)
                if line.starts_with("  ") && !line.starts_with("    ") && trimmed.ends_with(':') {
                    // Save previous job findings
                    if let Some(ref job_name) = current_job {
                        if !job_has_timeout {
                            findings.push(
                                Finding::new(
                                    "WF004",
                                    "workflows",
                                    Severity::Warning,
                                    format!(
                                        "Job '{}' missing timeout-minutes",
                                        job_name
                                    ),
                                )
                                .with_location(&file.path)
                                .with_description(
                                    "Jobs without timeout-minutes can run indefinitely, consuming resources and potentially incurring costs.",
                                )
                                .with_remediation(
                                    "Add 'timeout-minutes:' to each job to limit maximum execution time.",
                                ),
                            );
                        }
                    }
                    current_job = Some(trimmed.trim_end_matches(':').to_string());
                    job_has_timeout = false;
                    continue;
                }

                if current_job.is_some() && trimmed.starts_with("timeout-minutes:") {
                    job_has_timeout = true;
                }

                // If we encounter a top-level key (no indentation), we're out of jobs
                if !line.starts_with(' ') && !trimmed.is_empty() && trimmed.ends_with(':') {
                    in_jobs = false;
                }
            }

            // Check the last job
            if let Some(ref job_name) = current_job {
                if !job_has_timeout {
                    findings.push(
                        Finding::new(
                            "WF004",
                            "workflows",
                            Severity::Warning,
                            format!("Job '{}' missing timeout-minutes", job_name),
                        )
                        .with_location(&file.path)
                        .with_description(
                            "Jobs without timeout-minutes can run indefinitely, consuming resources and potentially incurring costs.",
                        )
                        .with_remediation(
                            "Add 'timeout-minutes:' to each job to limit maximum execution time.",
                        ),
                    );
                }
            }
        }
    }

    Ok(findings)
}

/// Check for concurrency configuration in workflow files
///
/// Concurrency controls prevent duplicate workflow runs and save resources.
///
/// # Arguments
///
/// * `scanner` - The scanner to access repository files
///
/// # Returns
///
/// A vector of findings for missing concurrency configuration
async fn check_workflow_concurrency(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    let workflow_files: Vec<_> = scanner
        .files_in_directory(".github/workflows")
        .into_iter()
        .filter(|f| f.path.ends_with(".yml") || f.path.ends_with(".yaml"))
        .collect();

    let any_has_concurrency = workflow_files.iter().any(|file| {
        scanner
            .read_file(&file.path)
            .map(|content| content.contains("concurrency:"))
            .unwrap_or(false)
    });

    if !any_has_concurrency && !workflow_files.is_empty() {
        findings.push(
            Finding::new(
                "WF005",
                "workflows",
                Severity::Info,
                "No workflow uses concurrency controls",
            )
            .with_description(
                "Concurrency controls can prevent duplicate workflow runs and save CI resources.",
            )
            .with_remediation(
                "Add a 'concurrency:' block to workflows to cancel redundant runs on the same branch.",
            ),
        );
    }

    Ok(findings)
}

/// Check if reusable workflows should be used
///
/// When there are 3 or more workflow files, suggests using reusable workflows
/// to reduce duplication.
///
/// # Arguments
///
/// * `scanner` - The scanner to access repository files
///
/// # Returns
///
/// A vector of findings for reusable workflow suggestions
async fn check_reusable_workflows(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    let workflow_files: Vec<_> = scanner
        .files_in_directory(".github/workflows")
        .into_iter()
        .filter(|f| f.path.ends_with(".yml") || f.path.ends_with(".yaml"))
        .collect();

    if workflow_files.len() >= 3 {
        let has_workflow_call = workflow_files.iter().any(|file| {
            scanner
                .read_file(&file.path)
                .map(|content| content.contains("workflow_call"))
                .unwrap_or(false)
        });

        if !has_workflow_call {
            findings.push(
                Finding::new(
                    "WF006",
                    "workflows",
                    Severity::Info,
                    "Consider using reusable workflows",
                )
                .with_description(
                    "With multiple workflow files, reusable workflows can reduce duplication and improve maintainability.",
                )
                .with_remediation(
                    "Create reusable workflows with 'workflow_call' trigger to share common steps across workflows.",
                ),
            );
        }
    }

    Ok(findings)
}

/// Check for artifact retention configuration
///
/// upload-artifact actions without retention-days may keep artifacts
/// longer than necessary, increasing storage costs.
///
/// # Arguments
///
/// * `scanner` - The scanner to access repository files
///
/// # Returns
///
/// A vector of findings for missing retention configuration
async fn check_artifacts_retention(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    for file in scanner.files_in_directory(".github/workflows") {
        if !file.path.ends_with(".yml") && !file.path.ends_with(".yaml") {
            continue;
        }

        if let Ok(content) = scanner.read_file(&file.path) {
            if content.contains("upload-artifact") && !content.contains("retention-days") {
                findings.push(
                    Finding::new(
                        "WF007",
                        "workflows",
                        Severity::Warning,
                        "upload-artifact used without retention-days",
                    )
                    .with_location(&file.path)
                    .with_description(
                        "Artifacts without explicit retention-days use the repository default (90 days), which may increase storage costs.",
                    )
                    .with_remediation(
                        "Add 'retention-days' parameter to upload-artifact steps to control artifact storage duration.",
                    ),
                );
            }
        }
    }

    Ok(findings)
}

/// Check for pull_request_target with checkout security risk
///
/// Using `pull_request_target` with `actions/checkout` of the PR head
/// can expose repository secrets to untrusted code.
///
/// # Arguments
///
/// * `scanner` - The scanner to access repository files
///
/// # Returns
///
/// A vector of findings for pull_request_target security risks
async fn check_pull_request_target(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    for file in scanner.files_in_directory(".github/workflows") {
        if !file.path.ends_with(".yml") && !file.path.ends_with(".yaml") {
            continue;
        }

        if let Ok(content) = scanner.read_file(&file.path) {
            if content.contains("pull_request_target") && content.contains("actions/checkout") {
                findings.push(
                    Finding::new(
                        "WF008",
                        "workflows",
                        Severity::Warning,
                        "pull_request_target with checkout detected",
                    )
                    .with_location(&file.path)
                    .with_description(
                        "Using pull_request_target with actions/checkout can expose repository secrets to untrusted PR code. This is a known security risk.",
                    )
                    .with_remediation(
                        "Avoid checking out PR head ref in pull_request_target workflows. If needed, use a separate workflow with limited permissions.",
                    ),
                );
            }
        }
    }

    Ok(findings)
}

/// Check for linter integration in CI workflows
///
/// Detects project languages and checks if appropriate linters are
/// configured in CI workflows.
///
/// # Arguments
///
/// * `scanner` - The scanner to access repository files
///
/// # Returns
///
/// A vector of findings for missing linter integration
async fn check_linters_in_ci(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    let languages = detect_languages(scanner);
    if languages.is_empty() {
        return Ok(findings);
    }

    // Collect all workflow content
    let mut all_workflow_content = String::new();
    for file in scanner.files_in_directory(".github/workflows") {
        if !file.path.ends_with(".yml") && !file.path.ends_with(".yaml") {
            continue;
        }
        if let Ok(content) = scanner.read_file(&file.path) {
            all_workflow_content.push_str(&content);
            all_workflow_content.push('\n');
        }
    }

    let content_lower = all_workflow_content.to_lowercase();

    // Known linter commands/actions by language
    let linter_keywords: Vec<(Language, &[&str], &str)> = vec![
        (
            Language::Rust,
            &["clippy", "cargo fmt", "cargo-fmt", "rustfmt"],
            "clippy and rustfmt (e.g., 'cargo clippy' and 'cargo fmt --check')",
        ),
        (
            Language::JavaScript,
            &["eslint", "prettier", "biome", "oxlint"],
            "ESLint and Prettier (e.g., 'npx eslint .' and 'npx prettier --check .')",
        ),
        (
            Language::Python,
            &["pylint", "flake8", "black", "ruff", "mypy", "pyright"],
            "Ruff, Flake8, or Pylint (e.g., 'ruff check .' or 'flake8')",
        ),
        (
            Language::Go,
            &["golangci-lint", "golangci", "go vet", "staticcheck"],
            "golangci-lint (e.g., 'golangci-lint run')",
        ),
        (
            Language::Ruby,
            &["rubocop", "standardrb"],
            "RuboCop (e.g., 'bundle exec rubocop')",
        ),
        (
            Language::Php,
            &["phpstan", "psalm", "phpcs", "php-cs-fixer"],
            "PHPStan or PHP_CodeSniffer (e.g., 'phpstan analyse')",
        ),
    ];

    for (language, keywords, suggestion) in &linter_keywords {
        if languages.contains(language) {
            let has_linter = keywords.iter().any(|kw| content_lower.contains(kw));
            if !has_linter {
                findings.push(
                    Finding::new(
                        "WF009",
                        "workflows",
                        Severity::Warning,
                        format!("No linter step found in CI for {:?}", language),
                    )
                    .with_description(
                        "Running linters in CI ensures code quality standards are enforced automatically.",
                    )
                    .with_remediation(format!(
                        "Add a linting step to your CI workflow using {}.",
                        suggestion
                    )),
                );
            }
        }
    }

    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::scanner::Scanner;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_check_workflow_secrets_detects_hardcoded_password() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        let workflow_file = workflows_dir.join("ci.yml");
        fs::write(
            &workflow_file,
            "name: CI\non: push\njobs:\n  test:\n    password: 'secret123'",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_workflow_secrets(&scanner).await.unwrap();

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id == "WF001"));
    }

    #[tokio::test]
    async fn test_check_workflow_permissions_missing() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        let workflow_file = workflows_dir.join("ci.yml");
        fs::write(
            &workflow_file,
            "name: CI\non: push\njobs:\n  test:\n    runs-on: ubuntu-latest",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_workflow_permissions(&scanner).await.unwrap();

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id == "WF002"));
    }

    #[tokio::test]
    async fn test_check_pinned_actions_detects_unpinned() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        let workflow_file = workflows_dir.join("ci.yml");
        fs::write(
            &workflow_file,
            "name: CI\njobs:\n  test:\n    uses: actions/checkout@main",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let config = Config {
            preset: "strict".to_string(),
            ..Default::default()
        };
        let findings = check_pinned_actions(&scanner, &config).await.unwrap();

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id == "WF003"));
    }

    #[tokio::test]
    async fn test_check_pinned_actions_not_strict() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        let workflow_file = workflows_dir.join("ci.yml");
        fs::write(
            &workflow_file,
            "name: CI\njobs:\n  test:\n    uses: actions/checkout@main",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let config = Config::default(); // "opensource" preset, not strict

        let findings = check_pinned_actions(&scanner, &config).await.unwrap();

        // Should not check for unpinned actions in non-strict mode
        assert!(findings.is_empty());
    }

    #[tokio::test]
    async fn test_check_workflow_secrets_no_workflows_dir() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        // No .github/workflows directory

        let scanner = Scanner::new(root.to_path_buf());
        let config = Config::default();
        let rules = WorkflowsRules;

        let findings = rules.run(&scanner, &config).await.unwrap();

        // No workflows directory means no findings
        assert!(findings.is_empty());
    }

    #[tokio::test]
    async fn test_workflows_rules_name() {
        let rules = WorkflowsRules;
        assert_eq!(rules.name(), "workflows");
    }

    #[tokio::test]
    async fn test_check_workflow_secrets_no_secrets() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        let workflow_file = workflows_dir.join("ci.yml");
        fs::write(
            &workflow_file,
            "name: CI\non: push\njobs:\n  test:\n    runs-on: ubuntu-latest",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_workflow_secrets(&scanner).await.unwrap();

        // Clean workflow file - no hardcoded secrets
        assert!(findings.iter().all(|f| f.rule_id != "WF001"));
    }

    #[tokio::test]
    async fn test_check_workflow_secrets_detects_token() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        let workflow_file = workflows_dir.join("deploy.yaml");
        fs::write(
            &workflow_file,
            "name: Deploy\njobs:\n  deploy:\n    token: 'ghp_1234567890abcdef'",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_workflow_secrets(&scanner).await.unwrap();

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id == "WF001"));
    }

    #[tokio::test]
    async fn test_check_workflow_permissions_present() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        let workflow_file = workflows_dir.join("ci.yml");
        fs::write(
            &workflow_file,
            "name: CI\non: push\npermissions:\n  contents: read\njobs:\n  test:\n    runs-on: ubuntu-latest",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_workflow_permissions(&scanner).await.unwrap();

        // Has permissions block, so no WF002 finding
        assert!(findings.iter().all(|f| f.rule_id != "WF002"));
    }

    #[tokio::test]
    async fn test_check_workflow_non_yaml_ignored() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        // Non-YAML file should be ignored
        fs::write(
            workflows_dir.join("README.md"),
            "# Workflows\npassword: 'secret123'",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_workflow_secrets(&scanner).await.unwrap();

        // Non-YAML file should not trigger findings
        assert!(findings.is_empty());
    }

    #[tokio::test]
    async fn test_check_pinned_actions_detects_master() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        fs::write(
            workflows_dir.join("ci.yml"),
            "name: CI\njobs:\n  test:\n    uses: actions/setup-node@master",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let config = Config {
            preset: "strict".to_string(),
            ..Default::default()
        };
        let findings = check_pinned_actions(&scanner, &config).await.unwrap();

        assert!(!findings.is_empty());
    }

    #[tokio::test]
    async fn test_check_pinned_actions_detects_latest() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        fs::write(
            workflows_dir.join("ci.yml"),
            "name: CI\njobs:\n  test:\n    uses: actions/checkout@latest",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let config = Config {
            preset: "strict".to_string(),
            ..Default::default()
        };
        let findings = check_pinned_actions(&scanner, &config).await.unwrap();

        assert!(!findings.is_empty());
    }

    #[tokio::test]
    async fn test_workflows_rules_run_with_workflows() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        // Create a workflow file with issues
        fs::write(
            workflows_dir.join("ci.yml"),
            "name: CI\non: push\njobs:\n  test:\n    runs-on: ubuntu-latest\n    password: 'hardcoded123'",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let config = Config::default();

        // Run the full WorkflowsRules::run which dispatches to all sub-checks
        let findings = WorkflowsRules.run(&scanner, &config).await.unwrap();

        // Should find WF001 (hardcoded password) and WF002 (missing permissions)
        assert!(findings.iter().any(|f| f.rule_id == "WF001"));
        assert!(findings.iter().any(|f| f.rule_id == "WF002"));
    }

    #[tokio::test]
    async fn test_workflows_rules_run_strict_with_unpinned() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        fs::write(
            workflows_dir.join("ci.yml"),
            "name: CI\non: push\npermissions:\n  contents: read\njobs:\n  test:\n    uses: actions/checkout@main",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let config = Config {
            preset: "strict".to_string(),
            ..Default::default()
        };

        // Run the full WorkflowsRules::run - should trigger pinned actions check
        let findings = WorkflowsRules.run(&scanner, &config).await.unwrap();

        // Should find WF003 (unpinned action)
        assert!(findings.iter().any(|f| f.rule_id == "WF003"));
    }

    #[tokio::test]
    async fn test_workflows_rules_run_with_disabled_rules() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        fs::write(
            workflows_dir.join("ci.yml"),
            "name: CI\non: push\njobs:\n  test:\n    password: 'secret'",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let mut config = Config::default();
        // Disable the secrets check
        config.rules.insert(
            "workflows/secrets".to_string(),
            crate::config::RuleConfig {
                enabled: false,
                severity: None,
            },
        );

        let findings = WorkflowsRules.run(&scanner, &config).await.unwrap();

        // WF001 should NOT be found because the rule is disabled
        assert!(findings.iter().all(|f| f.rule_id != "WF001"));
        // WF002 should still be found
        assert!(findings.iter().any(|f| f.rule_id == "WF002"));
    }

    // ===== WF004: Workflow Timeout Tests =====

    #[tokio::test]
    async fn test_check_workflow_timeout_missing() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        fs::write(
            workflows_dir.join("ci.yml"),
            "name: CI\non: push\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_workflow_timeout(&scanner).await.unwrap();

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id == "WF004"));
    }

    #[tokio::test]
    async fn test_check_workflow_timeout_present() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        fs::write(
            workflows_dir.join("ci.yml"),
            "name: CI\non: push\njobs:\n  test:\n    runs-on: ubuntu-latest\n    timeout-minutes: 30\n    steps:\n      - uses: actions/checkout@v4",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_workflow_timeout(&scanner).await.unwrap();

        assert!(findings.iter().all(|f| f.rule_id != "WF004"));
    }

    // ===== WF005: Workflow Concurrency Tests =====

    #[tokio::test]
    async fn test_check_workflow_concurrency_missing() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        fs::write(
            workflows_dir.join("ci.yml"),
            "name: CI\non: push\njobs:\n  test:\n    runs-on: ubuntu-latest",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_workflow_concurrency(&scanner).await.unwrap();

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id == "WF005"));
    }

    #[tokio::test]
    async fn test_check_workflow_concurrency_present() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        fs::write(
            workflows_dir.join("ci.yml"),
            "name: CI\non: push\nconcurrency:\n  group: ci-${{ github.ref }}\njobs:\n  test:\n    runs-on: ubuntu-latest",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_workflow_concurrency(&scanner).await.unwrap();

        assert!(findings.iter().all(|f| f.rule_id != "WF005"));
    }

    // ===== WF006: Reusable Workflows Tests =====

    #[tokio::test]
    async fn test_check_reusable_workflows_no_suggestion_few_files() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        fs::write(workflows_dir.join("ci.yml"), "name: CI\non: push").unwrap();
        fs::write(workflows_dir.join("deploy.yml"), "name: Deploy\non: push").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_reusable_workflows(&scanner).await.unwrap();

        // Only 2 workflow files, no suggestion
        assert!(findings.iter().all(|f| f.rule_id != "WF006"));
    }

    #[tokio::test]
    async fn test_check_reusable_workflows_suggests_when_many_files() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        fs::write(workflows_dir.join("ci.yml"), "name: CI\non: push").unwrap();
        fs::write(workflows_dir.join("deploy.yml"), "name: Deploy\non: push").unwrap();
        fs::write(workflows_dir.join("lint.yml"), "name: Lint\non: push").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_reusable_workflows(&scanner).await.unwrap();

        assert!(findings.iter().any(|f| f.rule_id == "WF006"));
    }

    #[tokio::test]
    async fn test_check_reusable_workflows_no_suggestion_when_workflow_call_exists() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        fs::write(workflows_dir.join("ci.yml"), "name: CI\non: push").unwrap();
        fs::write(workflows_dir.join("deploy.yml"), "name: Deploy\non: push").unwrap();
        fs::write(
            workflows_dir.join("shared.yml"),
            "name: Shared\non:\n  workflow_call:\njobs:\n  build:\n    runs-on: ubuntu-latest",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_reusable_workflows(&scanner).await.unwrap();

        assert!(findings.iter().all(|f| f.rule_id != "WF006"));
    }

    // ===== WF007: Artifacts Retention Tests =====

    #[tokio::test]
    async fn test_check_artifacts_retention_missing() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        fs::write(
            workflows_dir.join("ci.yml"),
            "name: CI\non: push\njobs:\n  test:\n    steps:\n      - uses: actions/upload-artifact@v4\n        with:\n          name: build\n          path: dist/",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_artifacts_retention(&scanner).await.unwrap();

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id == "WF007"));
    }

    #[tokio::test]
    async fn test_check_artifacts_retention_present() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        fs::write(
            workflows_dir.join("ci.yml"),
            "name: CI\non: push\njobs:\n  test:\n    steps:\n      - uses: actions/upload-artifact@v4\n        with:\n          name: build\n          path: dist/\n          retention-days: 5",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_artifacts_retention(&scanner).await.unwrap();

        assert!(findings.iter().all(|f| f.rule_id != "WF007"));
    }

    // ===== WF008: pull_request_target Tests =====

    #[tokio::test]
    async fn test_check_pull_request_target_with_checkout() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        fs::write(
            workflows_dir.join("pr.yml"),
            "name: PR\non: pull_request_target\njobs:\n  test:\n    steps:\n      - uses: actions/checkout@v4\n        with:\n          ref: ${{ github.event.pull_request.head.sha }}",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_pull_request_target(&scanner).await.unwrap();

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id == "WF008"));
    }

    #[tokio::test]
    async fn test_check_pull_request_target_without_checkout() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        fs::write(
            workflows_dir.join("pr.yml"),
            "name: PR\non: pull_request_target\njobs:\n  label:\n    steps:\n      - uses: actions/labeler@v4",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_pull_request_target(&scanner).await.unwrap();

        assert!(findings.iter().all(|f| f.rule_id != "WF008"));
    }

    #[tokio::test]
    async fn test_check_pull_request_target_no_trigger() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        fs::write(
            workflows_dir.join("ci.yml"),
            "name: CI\non: push\njobs:\n  test:\n    steps:\n      - uses: actions/checkout@v4",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_pull_request_target(&scanner).await.unwrap();

        assert!(findings.is_empty());
    }

    // ===== WF009: Linters in CI Tests =====

    #[tokio::test]
    async fn test_check_linters_in_ci_missing_for_rust() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        fs::write(root.join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        fs::write(
            workflows_dir.join("ci.yml"),
            "name: CI\non: push\njobs:\n  test:\n    steps:\n      - run: cargo test",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_linters_in_ci(&scanner).await.unwrap();

        assert!(findings.iter().any(|f| f.rule_id == "WF009"));
    }

    #[tokio::test]
    async fn test_check_linters_in_ci_present_for_rust() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        fs::write(root.join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();
        fs::write(
            workflows_dir.join("ci.yml"),
            "name: CI\non: push\njobs:\n  test:\n    steps:\n      - run: cargo clippy\n      - run: cargo test",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_linters_in_ci(&scanner).await.unwrap();

        assert!(findings.iter().all(|f| f.rule_id != "WF009"));
    }

    #[tokio::test]
    async fn test_check_linters_in_ci_no_languages() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        fs::write(
            workflows_dir.join("ci.yml"),
            "name: CI\non: push\njobs:\n  test:\n    runs-on: ubuntu-latest",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_linters_in_ci(&scanner).await.unwrap();

        // No detectable languages, no WF009 findings
        assert!(findings.iter().all(|f| f.rule_id != "WF009"));
    }

    #[tokio::test]
    async fn test_check_linters_in_ci_missing_for_javascript() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        fs::write(root.join("package.json"), r#"{"name": "test"}"#).unwrap();
        fs::write(
            workflows_dir.join("ci.yml"),
            "name: CI\non: push\njobs:\n  test:\n    steps:\n      - run: npm test",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_linters_in_ci(&scanner).await.unwrap();

        assert!(findings.iter().any(|f| f.rule_id == "WF009"));
    }

    #[tokio::test]
    async fn test_check_linters_in_ci_present_for_javascript() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let workflows_dir = root.join(".github").join("workflows");
        fs::create_dir_all(&workflows_dir).unwrap();

        fs::write(root.join("package.json"), r#"{"name": "test"}"#).unwrap();
        fs::write(
            workflows_dir.join("ci.yml"),
            "name: CI\non: push\njobs:\n  lint:\n    steps:\n      - run: npx eslint .\n  test:\n    steps:\n      - run: npm test",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_linters_in_ci(&scanner).await.unwrap();

        assert!(findings.iter().all(|f| f.rule_id != "WF009"));
    }
}
