//! Tests for utility modules

use repolens::utils::prerequisites::{
    CheckLevel, CheckOptions, CheckStatus, PrerequisitesReport, check_gh_authenticated,
    check_gh_installed, check_git_installed, check_is_git_repo, check_remote_is_github,
    check_remote_origin, run_all_checks,
};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_check_result_ok() {
    let result = repolens::utils::prerequisites::CheckResult::ok("Test", CheckLevel::Required);
    assert_eq!(result.name, "Test");
    assert_eq!(result.level, CheckLevel::Required);
    assert_eq!(result.status, CheckStatus::Ok);
    assert!(!result.is_failed());
    assert!(!result.is_required_failure());
}

#[test]
fn test_check_result_failed() {
    let result = repolens::utils::prerequisites::CheckResult::failed(
        "Test",
        CheckLevel::Required,
        "Error message",
        Some("Fix suggestion"),
    );
    assert_eq!(result.name, "Test");
    assert_eq!(result.status, CheckStatus::Failed);
    assert!(result.is_failed());
    assert!(result.is_required_failure());
    assert_eq!(result.message, Some("Error message".to_string()));
    assert_eq!(result.fix, Some("Fix suggestion".to_string()));
}

#[test]
fn test_check_result_skipped() {
    let result = repolens::utils::prerequisites::CheckResult::skipped("Test", CheckLevel::Optional);
    assert_eq!(result.status, CheckStatus::Skipped);
    assert!(!result.is_failed());
}

#[test]
fn test_prerequisites_report_new() {
    let report = PrerequisitesReport::new();
    assert!(report.checks.is_empty());
    assert!(report.all_required_passed());
}

#[test]
fn test_prerequisites_report_add() {
    let mut report = PrerequisitesReport::new();
    report.add(repolens::utils::prerequisites::CheckResult::ok(
        "Test",
        CheckLevel::Required,
    ));
    assert_eq!(report.checks.len(), 1);
}

#[test]
fn test_prerequisites_report_all_required_passed() {
    let mut report = PrerequisitesReport::new();
    report.add(repolens::utils::prerequisites::CheckResult::ok(
        "Test1",
        CheckLevel::Required,
    ));
    report.add(repolens::utils::prerequisites::CheckResult::ok(
        "Test2",
        CheckLevel::Required,
    ));
    assert!(report.all_required_passed());

    report.add(repolens::utils::prerequisites::CheckResult::failed(
        "Test3",
        CheckLevel::Required,
        "Error",
        None,
    ));
    assert!(!report.all_required_passed());
}

#[test]
fn test_prerequisites_report_required_failures() {
    let mut report = PrerequisitesReport::new();
    report.add(repolens::utils::prerequisites::CheckResult::ok(
        "Test1",
        CheckLevel::Required,
    ));
    report.add(repolens::utils::prerequisites::CheckResult::failed(
        "Test2",
        CheckLevel::Required,
        "Error",
        None,
    ));
    report.add(repolens::utils::prerequisites::CheckResult::failed(
        "Test3",
        CheckLevel::Optional,
        "Warning",
        None,
    ));

    let failures = report.required_failures();
    assert_eq!(failures.len(), 1);
    assert_eq!(failures[0].name, "Test2");
}

#[test]
fn test_prerequisites_report_optional_failures() {
    let mut report = PrerequisitesReport::new();
    report.add(repolens::utils::prerequisites::CheckResult::failed(
        "Test1",
        CheckLevel::Optional,
        "Warning",
        None,
    ));
    report.add(repolens::utils::prerequisites::CheckResult::failed(
        "Test2",
        CheckLevel::Required,
        "Error",
        None,
    ));

    let warnings = report.optional_failures();
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].name, "Test1");
    assert!(report.has_warnings());
}

#[test]
fn test_check_git_installed() {
    // This test assumes git is installed (should be in CI/dev environments)
    let result = check_git_installed();
    // We can't assert the exact result, but we can check the structure
    assert_eq!(result.name, "Git installed");
    assert_eq!(result.level, CheckLevel::Required);
}

#[test]
fn test_check_is_git_repo() {
    // Test with a non-git directory
    let temp_dir = TempDir::new().unwrap();
    let result = check_is_git_repo(temp_dir.path());
    assert_eq!(result.name, "Git repository");
    assert_eq!(result.status, CheckStatus::Failed);

    // Test with a git directory
    let git_dir = TempDir::new().unwrap();
    fs::create_dir(git_dir.path().join(".git")).unwrap();
    let result = check_is_git_repo(git_dir.path());
    assert_eq!(result.status, CheckStatus::Ok);
}

#[test]
fn test_check_gh_installed() {
    // This test assumes gh may or may not be installed
    // gh CLI is now optional (GITHUB_TOKEN is the primary auth method)
    let result = check_gh_installed();
    assert_eq!(result.name, "GitHub CLI installed");
    assert_eq!(result.level, CheckLevel::Optional);
}

#[test]
fn test_check_gh_authenticated() {
    // This test may pass or fail depending on environment
    // gh CLI auth is now optional (GITHUB_TOKEN is the primary auth method)
    let result = check_gh_authenticated();
    assert_eq!(result.name, "GitHub CLI authenticated");
    assert_eq!(result.level, CheckLevel::Optional);
}

#[test]
fn test_check_remote_origin() {
    let temp_dir = TempDir::new().unwrap();
    let result = check_remote_origin(temp_dir.path());
    assert_eq!(result.name, "Remote origin configured");
    assert_eq!(result.level, CheckLevel::Optional);
    // Status depends on whether git is initialized, but structure should be correct
}

#[test]
fn test_check_remote_is_github() {
    let temp_dir = TempDir::new().unwrap();
    let result = check_remote_is_github(temp_dir.path());
    assert_eq!(result.name, "Remote is GitHub");
    assert_eq!(result.level, CheckLevel::Optional);
    // Status depends on remote configuration, but should be Skipped if no remote
}

#[test]
fn test_run_all_checks() {
    let temp_dir = TempDir::new().unwrap();
    let options = CheckOptions::default();
    let report = run_all_checks(temp_dir.path(), &options);

    // Should have multiple checks
    assert!(!report.checks.is_empty());

    // Check that required checks are present
    let check_names: Vec<&str> = report.checks.iter().map(|c| c.name.as_str()).collect();
    assert!(check_names.contains(&"Git installed"));
    assert!(check_names.contains(&"GitHub CLI installed"));
}

#[test]
fn test_is_gh_available() {
    // This is a utility function, test that it doesn't panic
    let _ = repolens::utils::prerequisites::is_gh_available();
}

#[test]
fn test_get_repo_info() {
    // This may fail if not in a git repo or gh not authenticated
    // Just test that it doesn't panic and returns a Result
    let result = repolens::utils::prerequisites::get_repo_info();
    // Result may be Ok or Err depending on environment
    let _ = result;
}
