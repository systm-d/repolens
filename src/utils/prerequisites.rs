//! Prerequisites checking for RepoLens initialization
//!
//! This module verifies that required tools and configurations are available
//! before running RepoLens commands.

use anyhow::{Context, Result, bail};
use colored::Colorize;
use std::env;
use std::path::Path;
use std::process::Command;

/// Level of importance for a prerequisite check
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckLevel {
    /// Required for operation - failure blocks execution
    Required,
    /// Optional - failure generates a warning
    Optional,
}

/// Status of a prerequisite check
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    /// Check passed successfully
    Ok,
    /// Check failed
    Failed,
    /// Check was skipped (due to dependency failure)
    Skipped,
}

/// Result of a single prerequisite check
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// Name of the check
    pub name: String,
    /// Whether this check is required or optional
    pub level: CheckLevel,
    /// Status of the check
    pub status: CheckStatus,
    /// Human-readable message (shown on failure)
    pub message: Option<String>,
    /// Suggested fix for the issue
    pub fix: Option<String>,
}

impl CheckResult {
    /// Create a successful check result
    pub fn ok(name: &str, level: CheckLevel) -> Self {
        Self {
            name: name.to_string(),
            level,
            status: CheckStatus::Ok,
            message: None,
            fix: None,
        }
    }

    /// Create a failed check result
    pub fn failed(name: &str, level: CheckLevel, message: &str, fix: Option<&str>) -> Self {
        Self {
            name: name.to_string(),
            level,
            status: CheckStatus::Failed,
            message: Some(message.to_string()),
            fix: fix.map(|s| s.to_string()),
        }
    }

    /// Create a skipped check result
    pub fn skipped(name: &str, level: CheckLevel) -> Self {
        Self {
            name: name.to_string(),
            level,
            status: CheckStatus::Skipped,
            message: None,
            fix: None,
        }
    }

    /// Check if this result represents a failure
    #[allow(dead_code)]
    pub fn is_failed(&self) -> bool {
        self.status == CheckStatus::Failed
    }

    /// Check if this is a required check that failed
    pub fn is_required_failure(&self) -> bool {
        self.level == CheckLevel::Required && self.status == CheckStatus::Failed
    }

    /// Check if this is an optional check that failed
    pub fn is_optional_failure(&self) -> bool {
        self.level == CheckLevel::Optional && self.status == CheckStatus::Failed
    }
}

/// Aggregated report of all prerequisite checks
#[derive(Debug, Clone)]
pub struct PrerequisitesReport {
    /// All check results
    pub checks: Vec<CheckResult>,
}

impl PrerequisitesReport {
    /// Create a new empty report
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    /// Add a check result to the report
    pub fn add(&mut self, result: CheckResult) {
        self.checks.push(result);
    }

    /// Check if all required checks passed
    pub fn all_required_passed(&self) -> bool {
        !self.checks.iter().any(|c| c.is_required_failure())
    }

    /// Get all failed required checks
    pub fn required_failures(&self) -> Vec<&CheckResult> {
        self.checks
            .iter()
            .filter(|c| c.is_required_failure())
            .collect()
    }

    /// Get all failed optional checks (warnings)
    pub fn optional_failures(&self) -> Vec<&CheckResult> {
        self.checks
            .iter()
            .filter(|c| c.is_optional_failure())
            .collect()
    }

    /// Check if there are any warnings
    pub fn has_warnings(&self) -> bool {
        self.checks.iter().any(|c| c.is_optional_failure())
    }
}

impl Default for PrerequisitesReport {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Check functions
// ============================================================================

/// Check if git is installed
pub fn check_git_installed() -> CheckResult {
    let output = Command::new("git").arg("--version").output();

    match output {
        Ok(o) if o.status.success() => CheckResult::ok("Git installed", CheckLevel::Required),
        _ => CheckResult::failed(
            "Git installed",
            CheckLevel::Required,
            "Git is not installed",
            Some("Install git: https://git-scm.com/downloads"),
        ),
    }
}

/// Check if the current directory is a git repository
pub fn check_is_git_repo(root: &Path) -> CheckResult {
    let git_dir = root.join(".git");

    if git_dir.exists() {
        CheckResult::ok("Git repository", CheckLevel::Required)
    } else {
        CheckResult::failed(
            "Git repository",
            CheckLevel::Required,
            "Not a git repository",
            Some("Run: git init"),
        )
    }
}

/// Check if GITHUB_TOKEN environment variable is set
pub fn check_github_token() -> CheckResult {
    match env::var("GITHUB_TOKEN") {
        Ok(token) if !token.is_empty() => {
            CheckResult::ok("GitHub Token configured", CheckLevel::Optional)
        }
        _ => CheckResult::failed(
            "GitHub Token configured",
            CheckLevel::Optional,
            "GITHUB_TOKEN environment variable is not set",
            Some("Set GITHUB_TOKEN=<your-token> or use: gh auth login"),
        ),
    }
}

/// Check if GitHub CLI (gh) is installed
pub fn check_gh_installed() -> CheckResult {
    let output = Command::new("gh").arg("--version").output();

    match output {
        Ok(o) if o.status.success() => {
            CheckResult::ok("GitHub CLI installed", CheckLevel::Optional)
        }
        _ => CheckResult::failed(
            "GitHub CLI installed",
            CheckLevel::Optional,
            "GitHub CLI (gh) is not installed",
            Some("Install gh: https://cli.github.com/ (optional if GITHUB_TOKEN is set)"),
        ),
    }
}

/// Check if GitHub CLI is authenticated
pub fn check_gh_authenticated() -> CheckResult {
    let output = Command::new("gh").args(["auth", "status"]).output();

    match output {
        Ok(o) if o.status.success() => {
            CheckResult::ok("GitHub CLI authenticated", CheckLevel::Optional)
        }
        _ => CheckResult::failed(
            "GitHub CLI authenticated",
            CheckLevel::Optional,
            "GitHub CLI is not authenticated",
            Some("Run: gh auth login (optional if GITHUB_TOKEN is set)"),
        ),
    }
}

/// Check if any GitHub authentication method is available
/// Returns Ok if either GITHUB_TOKEN is set or gh CLI is authenticated
pub fn check_github_auth_available() -> CheckResult {
    // Check GITHUB_TOKEN first (preferred)
    if env::var("GITHUB_TOKEN")
        .map(|t| !t.is_empty())
        .unwrap_or(false)
    {
        return CheckResult::ok("GitHub authentication", CheckLevel::Required);
    }

    // Fall back to gh CLI
    let output = Command::new("gh").args(["auth", "status"]).output();
    match output {
        Ok(o) if o.status.success() => {
            CheckResult::ok("GitHub authentication", CheckLevel::Required)
        }
        _ => CheckResult::failed(
            "GitHub authentication",
            CheckLevel::Required,
            "No GitHub authentication available",
            Some("Set GITHUB_TOKEN environment variable or run: gh auth login"),
        ),
    }
}

/// Check if a remote origin is configured
pub fn check_remote_origin(root: &Path) -> CheckResult {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(root)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            CheckResult::ok("Remote origin configured", CheckLevel::Optional)
        }
        _ => CheckResult::failed(
            "Remote origin configured",
            CheckLevel::Optional,
            "No remote origin configured",
            Some("Run: git remote add origin <url>"),
        ),
    }
}

/// Check if the remote origin is a GitHub repository
pub fn check_remote_is_github(root: &Path) -> CheckResult {
    let output = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(root)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let url = String::from_utf8_lossy(&o.stdout);
            if url.contains("github.com") {
                CheckResult::ok("Remote is GitHub", CheckLevel::Optional)
            } else {
                CheckResult::failed(
                    "Remote is GitHub",
                    CheckLevel::Optional,
                    "Remote origin is not a GitHub repository",
                    Some("RepoLens works best with GitHub repositories"),
                )
            }
        }
        _ => CheckResult::skipped("Remote is GitHub", CheckLevel::Optional),
    }
}

// ============================================================================
// Run all checks
// ============================================================================

/// Options for running prerequisite checks
#[derive(Debug, Clone, Default)]
pub struct CheckOptions {
    /// Skip optional checks
    #[allow(dead_code)]
    pub skip_optional: bool,
}

/// Run all prerequisite checks
pub fn run_all_checks(root: &Path, _options: &CheckOptions) -> PrerequisitesReport {
    let mut report = PrerequisitesReport::new();

    // Required checks
    let git_installed = check_git_installed();
    let git_ok = git_installed.status == CheckStatus::Ok;
    report.add(git_installed);

    if git_ok {
        report.add(check_is_git_repo(root));
    } else {
        report.add(CheckResult::skipped("Git repository", CheckLevel::Required));
    }

    // GitHub authentication (required) - either GITHUB_TOKEN or gh CLI
    report.add(check_github_auth_available());

    // GitHub Token check (optional - informational)
    report.add(check_github_token());

    // gh CLI checks (optional - fallback method)
    let gh_installed = check_gh_installed();
    let gh_ok = gh_installed.status == CheckStatus::Ok;
    report.add(gh_installed);

    if gh_ok {
        report.add(check_gh_authenticated());
    } else {
        report.add(CheckResult::skipped(
            "GitHub CLI authenticated",
            CheckLevel::Optional,
        ));
    }

    // Optional checks (only if git repo exists)
    let is_repo = report
        .checks
        .iter()
        .find(|c| c.name == "Git repository")
        .map(|c| c.status == CheckStatus::Ok)
        .unwrap_or(false);

    if is_repo {
        let remote_result = check_remote_origin(root);
        let has_remote = remote_result.status == CheckStatus::Ok;
        report.add(remote_result);

        if has_remote {
            report.add(check_remote_is_github(root));
        } else {
            report.add(CheckResult::skipped(
                "Remote is GitHub",
                CheckLevel::Optional,
            ));
        }
    } else {
        report.add(CheckResult::skipped(
            "Remote origin configured",
            CheckLevel::Optional,
        ));
        report.add(CheckResult::skipped(
            "Remote is GitHub",
            CheckLevel::Optional,
        ));
    }

    report
}

// ============================================================================
// Display functions
// ============================================================================

/// Display the full prerequisites report
pub fn display_report(report: &PrerequisitesReport, _verbose: bool) {
    println!("{}\n", "Checking prerequisites...".bold());

    for check in &report.checks {
        let icon = match check.status {
            CheckStatus::Ok => "✓".green(),
            CheckStatus::Failed if check.level == CheckLevel::Required => "✗".red(),
            CheckStatus::Failed => "!".yellow(),
            CheckStatus::Skipped => "○".dimmed(),
        };

        let name = match check.status {
            CheckStatus::Ok => check.name.normal(),
            CheckStatus::Failed if check.level == CheckLevel::Required => check.name.red(),
            CheckStatus::Failed => check.name.yellow(),
            CheckStatus::Skipped => check.name.dimmed(),
        };

        let suffix = match check.status {
            CheckStatus::Skipped => " (skipped)".dimmed().to_string(),
            CheckStatus::Failed if check.level == CheckLevel::Optional => {
                " (optional)".dimmed().to_string()
            }
            _ => String::new(),
        };

        println!("  {} {}{}", icon, name, suffix);

        // Show message and fix for failures
        if check.status == CheckStatus::Failed {
            if let Some(msg) = &check.message {
                println!("    {}", msg.dimmed());
            }
            if let Some(fix) = &check.fix {
                println!("    {}: {}", "Fix".cyan(), fix);
            }
        }
    }

    println!();
}

/// Display error summary for failed required checks
pub fn display_error_summary(report: &PrerequisitesReport) {
    let failures = report.required_failures();
    if failures.is_empty() {
        return;
    }

    eprintln!(
        "{} {} required prerequisite(s) failed:",
        "Error:".red().bold(),
        failures.len()
    );

    for check in failures {
        eprintln!("  {} {}", "•".red(), check.name);
        if let Some(fix) = &check.fix {
            eprintln!("    {}: {}", "Fix".cyan(), fix);
        }
    }
}

/// Display warnings for failed optional checks
pub fn display_warnings(report: &PrerequisitesReport) {
    let warnings = report.optional_failures();
    if warnings.is_empty() {
        return;
    }

    println!(
        "{} {} optional check(s) failed:",
        "Warning:".yellow().bold(),
        warnings.len()
    );

    for check in warnings {
        if let Some(msg) = &check.message {
            println!("  {} {}", "•".yellow(), msg);
        }
    }

    println!();
}

// ============================================================================
// Centralized utility functions (used by other modules)
// ============================================================================

/// Check if GITHUB_TOKEN environment variable is set
#[allow(dead_code)]
pub fn is_github_token_available() -> bool {
    env::var("GITHUB_TOKEN")
        .map(|t| !t.is_empty())
        .unwrap_or(false)
}

/// Check if gh CLI is available and authenticated
pub fn is_gh_available() -> bool {
    Command::new("gh")
        .args(["auth", "status"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if any GitHub authentication method is available
/// Returns true if GITHUB_TOKEN is set or gh CLI is authenticated
#[allow(dead_code)]
pub fn is_github_auth_available() -> bool {
    is_github_token_available() || is_gh_available()
}

/// Get repository info (owner/name) from GitHub CLI
pub fn get_repo_info() -> Result<String> {
    let output = Command::new("gh")
        .args([
            "repo",
            "view",
            "--json",
            "nameWithOwner",
            "-q",
            ".nameWithOwner",
        ])
        .output()
        .context("Failed to get repository info")?;

    if !output.status.success() {
        bail!("Failed to get repository info. Make sure you're in a git repository.");
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // --- CheckResult tests ---
    #[test]
    fn test_check_result_ok() {
        let result = CheckResult::ok("test", CheckLevel::Required);
        assert_eq!(result.name, "test");
        assert_eq!(result.level, CheckLevel::Required);
        assert_eq!(result.status, CheckStatus::Ok);
        assert!(result.message.is_none());
        assert!(result.fix.is_none());
    }

    #[test]
    fn test_check_result_failed() {
        let result = CheckResult::failed(
            "test",
            CheckLevel::Required,
            "Error message",
            Some("Fix suggestion"),
        );
        assert_eq!(result.name, "test");
        assert_eq!(result.level, CheckLevel::Required);
        assert_eq!(result.status, CheckStatus::Failed);
        assert_eq!(result.message, Some("Error message".to_string()));
        assert_eq!(result.fix, Some("Fix suggestion".to_string()));
    }

    #[test]
    fn test_check_result_failed_no_fix() {
        let result = CheckResult::failed("test", CheckLevel::Optional, "Error message", None);
        assert_eq!(result.fix, None);
    }

    #[test]
    fn test_check_result_skipped() {
        let result = CheckResult::skipped("test", CheckLevel::Optional);
        assert_eq!(result.status, CheckStatus::Skipped);
        assert!(result.message.is_none());
    }

    #[test]
    fn test_check_result_is_failed() {
        let ok = CheckResult::ok("test", CheckLevel::Required);
        let failed = CheckResult::failed("test", CheckLevel::Required, "msg", None);
        let skipped = CheckResult::skipped("test", CheckLevel::Required);

        assert!(!ok.is_failed());
        assert!(failed.is_failed());
        assert!(!skipped.is_failed());
    }

    #[test]
    fn test_check_result_is_required_failure() {
        let required_ok = CheckResult::ok("test", CheckLevel::Required);
        let required_failed = CheckResult::failed("test", CheckLevel::Required, "msg", None);
        let optional_failed = CheckResult::failed("test", CheckLevel::Optional, "msg", None);

        assert!(!required_ok.is_required_failure());
        assert!(required_failed.is_required_failure());
        assert!(!optional_failed.is_required_failure());
    }

    #[test]
    fn test_check_result_is_optional_failure() {
        let optional_ok = CheckResult::ok("test", CheckLevel::Optional);
        let optional_failed = CheckResult::failed("test", CheckLevel::Optional, "msg", None);
        let required_failed = CheckResult::failed("test", CheckLevel::Required, "msg", None);

        assert!(!optional_ok.is_optional_failure());
        assert!(optional_failed.is_optional_failure());
        assert!(!required_failed.is_optional_failure());
    }

    // --- PrerequisitesReport tests ---
    #[test]
    fn test_prerequisites_report_new() {
        let report = PrerequisitesReport::new();
        assert!(report.checks.is_empty());
    }

    #[test]
    fn test_prerequisites_report_default() {
        let report = PrerequisitesReport::default();
        assert!(report.checks.is_empty());
    }

    #[test]
    fn test_prerequisites_report_add() {
        let mut report = PrerequisitesReport::new();
        report.add(CheckResult::ok("test1", CheckLevel::Required));
        report.add(CheckResult::ok("test2", CheckLevel::Optional));
        assert_eq!(report.checks.len(), 2);
    }

    #[test]
    fn test_prerequisites_report_all_required_passed() {
        let mut report = PrerequisitesReport::new();
        report.add(CheckResult::ok("req1", CheckLevel::Required));
        report.add(CheckResult::ok("req2", CheckLevel::Required));
        report.add(CheckResult::failed(
            "opt1",
            CheckLevel::Optional,
            "msg",
            None,
        ));
        assert!(report.all_required_passed());
    }

    #[test]
    fn test_prerequisites_report_required_failures() {
        let mut report = PrerequisitesReport::new();
        report.add(CheckResult::ok("req1", CheckLevel::Required));
        report.add(CheckResult::failed(
            "req2",
            CheckLevel::Required,
            "msg",
            None,
        ));
        report.add(CheckResult::failed(
            "opt1",
            CheckLevel::Optional,
            "msg",
            None,
        ));

        let failures = report.required_failures();
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].name, "req2");
    }

    #[test]
    fn test_prerequisites_report_optional_failures() {
        let mut report = PrerequisitesReport::new();
        report.add(CheckResult::ok("opt1", CheckLevel::Optional));
        report.add(CheckResult::failed(
            "opt2",
            CheckLevel::Optional,
            "msg",
            None,
        ));
        report.add(CheckResult::failed(
            "req1",
            CheckLevel::Required,
            "msg",
            None,
        ));

        let warnings = report.optional_failures();
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].name, "opt2");
    }

    #[test]
    fn test_prerequisites_report_has_warnings() {
        let mut report = PrerequisitesReport::new();
        report.add(CheckResult::ok("test", CheckLevel::Required));
        assert!(!report.has_warnings());

        report.add(CheckResult::failed(
            "opt",
            CheckLevel::Optional,
            "msg",
            None,
        ));
        assert!(report.has_warnings());
    }

    // --- Check functions tests ---
    #[test]
    fn test_check_git_installed() {
        // git should be installed on the test machine
        let result = check_git_installed();
        assert_eq!(result.level, CheckLevel::Required);
        // Result depends on system, but should not panic
        assert!(result.status == CheckStatus::Ok || result.status == CheckStatus::Failed);
    }

    #[test]
    fn test_check_gh_installed() {
        let result = check_gh_installed();
        assert_eq!(result.level, CheckLevel::Optional);
        // gh may or may not be installed
        assert!(result.status == CheckStatus::Ok || result.status == CheckStatus::Failed);
    }

    #[test]
    fn test_check_gh_authenticated() {
        let result = check_gh_authenticated();
        assert_eq!(result.level, CheckLevel::Optional);
        // gh may or may not be authenticated
        assert!(result.status == CheckStatus::Ok || result.status == CheckStatus::Failed);
    }

    #[test]
    fn test_check_remote_origin() {
        let temp_dir = TempDir::new().unwrap();
        let result = check_remote_origin(temp_dir.path());
        assert_eq!(result.level, CheckLevel::Optional);
        // No git repo, should fail
        assert_eq!(result.status, CheckStatus::Failed);
    }

    #[test]
    fn test_check_remote_is_github() {
        let temp_dir = TempDir::new().unwrap();
        let result = check_remote_is_github(temp_dir.path());
        assert_eq!(result.level, CheckLevel::Optional);
        // No git repo, should be skipped
        assert_eq!(result.status, CheckStatus::Skipped);
    }

    #[test]
    fn test_run_all_checks() {
        let temp_dir = TempDir::new().unwrap();
        let options = CheckOptions {
            skip_optional: false,
        };
        let report = run_all_checks(temp_dir.path(), &options);

        // Should have multiple checks
        assert!(!report.checks.is_empty());

        // Should have git installed check
        assert!(report.checks.iter().any(|c| c.name == "Git installed"));
    }

    // --- Utility function tests ---
    #[test]
    fn test_is_github_token_available() {
        // Just verify it doesn't panic
        let _ = is_github_token_available();
    }

    #[test]
    fn test_is_gh_available() {
        // Just verify it doesn't panic
        let _ = is_gh_available();
    }

    #[test]
    fn test_is_github_auth_available() {
        // Just verify it doesn't panic
        let _ = is_github_auth_available();
    }

    #[test]
    fn test_get_repo_info() {
        // This will fail if not in a git repo with gh configured
        // Just verify it doesn't panic
        let _ = get_repo_info();
    }

    // --- GitHub authentication tests ---
    #[test]
    fn test_check_github_token() {
        let result = check_github_token();
        assert_eq!(result.level, CheckLevel::Optional);
        // Token may or may not be set
        assert!(result.status == CheckStatus::Ok || result.status == CheckStatus::Failed);
    }

    #[test]
    fn test_check_github_auth_available() {
        let result = check_github_auth_available();
        assert_eq!(result.level, CheckLevel::Required);
        // Auth may or may not be available
        assert!(result.status == CheckStatus::Ok || result.status == CheckStatus::Failed);
    }

    // --- Additional tests for improved coverage ---

    #[test]
    fn test_check_is_git_repo_not_a_repo() {
        let temp_dir = TempDir::new().unwrap();
        let result = check_is_git_repo(temp_dir.path());
        assert_eq!(result.status, CheckStatus::Failed);
        assert_eq!(result.level, CheckLevel::Required);
        assert!(result.fix.is_some());
    }

    #[test]
    fn test_check_is_git_repo_is_a_repo() {
        let temp_dir = TempDir::new().unwrap();
        // Create .git directory to simulate a git repo
        std::fs::create_dir(temp_dir.path().join(".git")).unwrap();
        let result = check_is_git_repo(temp_dir.path());
        assert_eq!(result.status, CheckStatus::Ok);
        assert_eq!(result.level, CheckLevel::Required);
    }

    #[test]
    fn test_check_remote_origin_with_git_repo() {
        use std::process::Command;
        let temp_dir = TempDir::new().unwrap();
        // Initialize git repo
        let _ = Command::new("git")
            .args(["init"])
            .current_dir(temp_dir.path())
            .output();

        // No remote yet
        let result = check_remote_origin(temp_dir.path());
        assert_eq!(result.status, CheckStatus::Failed);
    }

    #[test]
    fn test_check_remote_origin_with_remote() {
        use std::process::Command;
        let temp_dir = TempDir::new().unwrap();
        // Initialize git repo
        let _ = Command::new("git")
            .args(["init"])
            .current_dir(temp_dir.path())
            .output();
        // Add remote origin
        let _ = Command::new("git")
            .args([
                "remote",
                "add",
                "origin",
                "https://github.com/test/repo.git",
            ])
            .current_dir(temp_dir.path())
            .output();

        let result = check_remote_origin(temp_dir.path());
        assert_eq!(result.status, CheckStatus::Ok);
    }

    #[test]
    fn test_check_remote_is_github_with_github_remote() {
        use std::process::Command;
        let temp_dir = TempDir::new().unwrap();
        // Initialize git repo
        let _ = Command::new("git")
            .args(["init"])
            .current_dir(temp_dir.path())
            .output();
        // Add GitHub remote
        let _ = Command::new("git")
            .args([
                "remote",
                "add",
                "origin",
                "https://github.com/test/repo.git",
            ])
            .current_dir(temp_dir.path())
            .output();

        let result = check_remote_is_github(temp_dir.path());
        assert_eq!(result.status, CheckStatus::Ok);
    }

    #[test]
    fn test_check_remote_is_github_with_non_github_remote() {
        use std::process::Command;
        let temp_dir = TempDir::new().unwrap();
        // Initialize git repo
        let _ = Command::new("git")
            .args(["init"])
            .current_dir(temp_dir.path())
            .output();
        // Add non-GitHub remote
        let _ = Command::new("git")
            .args([
                "remote",
                "add",
                "origin",
                "https://gitlab.com/test/repo.git",
            ])
            .current_dir(temp_dir.path())
            .output();

        let result = check_remote_is_github(temp_dir.path());
        assert_eq!(result.status, CheckStatus::Failed);
        assert_eq!(result.level, CheckLevel::Optional);
    }

    #[test]
    fn test_run_all_checks_with_git_repo() {
        use std::process::Command;
        let temp_dir = TempDir::new().unwrap();
        // Initialize git repo
        let _ = Command::new("git")
            .args(["init"])
            .current_dir(temp_dir.path())
            .output();

        let options = CheckOptions::default();
        let report = run_all_checks(temp_dir.path(), &options);

        // Should have Git repository check that passes
        let git_repo_check = report
            .checks
            .iter()
            .find(|c| c.name == "Git repository")
            .expect("Should have Git repository check");
        assert_eq!(git_repo_check.status, CheckStatus::Ok);

        // Should have Remote origin check (optional, likely failed since no remote)
        assert!(
            report
                .checks
                .iter()
                .any(|c| c.name == "Remote origin configured")
        );
    }

    #[test]
    fn test_run_all_checks_reports_all_check_types() {
        let temp_dir = TempDir::new().unwrap();
        let options = CheckOptions::default();
        let report = run_all_checks(temp_dir.path(), &options);

        // Verify we have checks for all expected items
        let expected_checks = [
            "Git installed",
            "Git repository",
            "GitHub authentication",
            "GitHub Token configured",
            "GitHub CLI installed",
            "Remote origin configured",
            "Remote is GitHub",
        ];

        for expected in expected_checks {
            assert!(
                report.checks.iter().any(|c| c.name == expected),
                "Missing check: {}",
                expected
            );
        }
    }

    #[test]
    fn test_check_options_default() {
        let options = CheckOptions::default();
        assert!(!options.skip_optional);
    }

    #[test]
    fn test_prerequisites_report_all_required_passed_with_failures() {
        let mut report = PrerequisitesReport::new();
        report.add(CheckResult::ok("req1", CheckLevel::Required));
        report.add(CheckResult::failed(
            "req2",
            CheckLevel::Required,
            "msg",
            None,
        ));
        assert!(!report.all_required_passed());
    }

    #[test]
    fn test_prerequisites_report_empty() {
        let report = PrerequisitesReport::new();
        assert!(report.all_required_passed()); // No required checks = all passed
        assert!(!report.has_warnings()); // No warnings
        assert!(report.required_failures().is_empty());
        assert!(report.optional_failures().is_empty());
    }
}
