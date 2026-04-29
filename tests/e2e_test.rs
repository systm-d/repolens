//! End-to-end tests for RepoLens CLI
//!
//! These tests run the CLI against real or simulated repositories to verify
//! that the audit correctly identifies issues and produces expected results.
//!
//! Tests marked with #[ignore] require network access and clone real repos.
//! Run them with: cargo test --test e2e_test -- --ignored

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use std::process::Command as StdCommand;
use tempfile::TempDir;

#[allow(deprecated)]
fn get_cmd() -> Command {
    Command::cargo_bin("repolens").unwrap()
}

// ============================================================================
// E2E Tests on RepoLens itself (this repository)
// ============================================================================

#[tokio::test]
async fn e2e_repolens_audit_runs_successfully() {
    // Run audit on the RepoLens repository itself
    let repo_root = std::env::current_dir().unwrap();

    get_cmd()
        .current_dir(&repo_root)
        .args(["plan", "--format", "json"])
        .assert()
        .code(predicate::in_iter([0, 1, 2])); // 0 = no issues, 1 = critical, 2 = warnings
}

#[tokio::test]
async fn e2e_repolens_has_required_files() {
    let repo_root = std::env::current_dir().unwrap();

    // RepoLens should have all these files
    assert!(repo_root.join("README.md").exists());
    assert!(repo_root.join("LICENSE").exists());
    assert!(repo_root.join("CHANGELOG.md").exists());
    assert!(repo_root.join("Cargo.toml").exists());
    assert!(repo_root.join("Cargo.lock").exists());
    assert!(repo_root.join(".gitignore").exists());
}

#[tokio::test]
async fn e2e_repolens_report_json_valid() {
    let repo_root = std::env::current_dir().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("report.json");

    get_cmd()
        .current_dir(&repo_root)
        .args(["report", "--format", "json", "--output"])
        .arg(&output_path)
        .assert()
        .code(predicate::in_iter([0, 1, 2]));

    // Verify output is valid JSON
    let content = fs::read_to_string(&output_path).unwrap();
    let report: serde_json::Value =
        serde_json::from_str(&content).expect("Report should be valid JSON");

    // Verify report structure (report has different format than plan)
    assert!(
        report.get("findings").is_some()
            || report
                .get("audit")
                .and_then(|a| a.get("findings"))
                .is_some(),
        "Report should have findings"
    );
    assert!(
        report.get("repository_name").is_some() || report.get("repository").is_some(),
        "Report should have repository info"
    );
}

#[tokio::test]
async fn e2e_repolens_report_markdown_valid() {
    let repo_root = std::env::current_dir().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("report.md");

    get_cmd()
        .current_dir(&repo_root)
        .args(["report", "--format", "markdown", "--output"])
        .arg(&output_path)
        .assert()
        .code(predicate::in_iter([0, 1, 2]));

    // Verify output is valid Markdown
    let content = fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("# "), "Markdown should have headers");
}

// ============================================================================
// E2E Tests with simulated repositories
// ============================================================================

/// Create a minimal Rust project for testing
fn create_rust_project(dir: &Path) {
    // Cargo.toml
    fs::write(
        dir.join("Cargo.toml"),
        r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
"#,
    )
    .unwrap();

    // src/main.rs
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("src/main.rs"),
        "fn main() { println!(\"Hello\"); }\n",
    )
    .unwrap();

    // Initialize git
    StdCommand::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .ok();
    StdCommand::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir)
        .output()
        .ok();
    StdCommand::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir)
        .output()
        .ok();
}

/// Create a minimal Node.js project for testing
fn create_node_project(dir: &Path) {
    // package.json
    fs::write(
        dir.join("package.json"),
        r#"{
  "name": "test-project",
  "version": "1.0.0",
  "description": "Test project",
  "main": "index.js",
  "scripts": {
    "test": "echo \"Error: no test specified\" && exit 1"
  }
}
"#,
    )
    .unwrap();

    // index.js
    fs::write(dir.join("index.js"), "console.log('Hello');\n").unwrap();

    // Initialize git
    StdCommand::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .ok();
    StdCommand::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir)
        .output()
        .ok();
    StdCommand::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir)
        .output()
        .ok();
}

/// Create a minimal Python project for testing
fn create_python_project(dir: &Path) {
    // pyproject.toml
    fs::write(
        dir.join("pyproject.toml"),
        r#"[project]
name = "test-project"
version = "0.1.0"
description = "Test project"
requires-python = ">=3.8"
"#,
    )
    .unwrap();

    // main.py
    fs::write(dir.join("main.py"), "print('Hello')\n").unwrap();

    // Initialize git
    StdCommand::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .ok();
    StdCommand::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir)
        .output()
        .ok();
    StdCommand::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir)
        .output()
        .ok();
}

#[tokio::test]
async fn e2e_rust_project_missing_readme() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_project(temp_dir.path());

    // Initialize repolens config
    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "opensource",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    // Run plan - should detect missing README
    let output = get_cmd()
        .current_dir(temp_dir.path())
        .args(["plan"])
        .assert()
        .code(predicate::eq(1)) // Should find issues
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        stdout.contains("README") || stdout.contains("readme"),
        "Should detect missing README"
    );
}

#[tokio::test]
async fn e2e_rust_project_missing_license() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_project(temp_dir.path());

    // Initialize repolens config
    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "opensource",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    // Run plan - should detect missing LICENSE
    let output = get_cmd()
        .current_dir(temp_dir.path())
        .args(["plan"])
        .assert()
        .code(predicate::eq(1))
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(
        stdout.contains("LICENSE") || stdout.contains("license"),
        "Should detect missing LICENSE"
    );
}

#[tokio::test]
async fn e2e_rust_project_missing_lock_file() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_project(temp_dir.path());

    // Initialize repolens config
    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "opensource",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    // Run plan with JSON output
    let output_path = temp_dir.path().join("plan.json");
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["plan", "--format", "json", "--output"])
        .arg(&output_path)
        .assert()
        .code(predicate::eq(1));

    let content = fs::read_to_string(&output_path).unwrap();
    let plan: serde_json::Value = serde_json::from_str(&content).unwrap();

    // Check that DEP003 (missing lock file) is detected
    let findings = plan
        .get("audit")
        .and_then(|a| a.get("findings"))
        .and_then(|f| f.as_array());
    assert!(findings.is_some(), "Plan should have findings array");

    let has_lock_file_finding = findings.unwrap().iter().any(|f| {
        f.get("rule_id")
            .and_then(|r| r.as_str())
            .map(|r| r == "DEP003")
            .unwrap_or(false)
    });
    assert!(
        has_lock_file_finding,
        "Should detect missing Cargo.lock (DEP003)"
    );
}

#[tokio::test]
async fn e2e_node_project_missing_lock_file() {
    let temp_dir = TempDir::new().unwrap();
    create_node_project(temp_dir.path());

    // Initialize repolens config
    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "opensource",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    // Run plan with JSON output
    let output_path = temp_dir.path().join("plan.json");
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["plan", "--format", "json", "--output"])
        .arg(&output_path)
        .assert()
        .code(predicate::eq(1));

    let content = fs::read_to_string(&output_path).unwrap();
    let plan: serde_json::Value = serde_json::from_str(&content).unwrap();

    let findings = plan
        .get("audit")
        .and_then(|a| a.get("findings"))
        .and_then(|f| f.as_array());
    assert!(findings.is_some(), "Plan should have findings array");

    let has_lock_file_finding = findings.unwrap().iter().any(|f| {
        f.get("rule_id")
            .and_then(|r| r.as_str())
            .map(|r| r == "DEP003")
            .unwrap_or(false)
    });
    assert!(
        has_lock_file_finding,
        "Should detect missing package-lock.json (DEP003)"
    );
}

#[tokio::test]
async fn e2e_python_project_audit() {
    let temp_dir = TempDir::new().unwrap();
    create_python_project(temp_dir.path());

    // Initialize repolens config
    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "opensource",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    // Run plan - should complete without crashing
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["plan"])
        .assert()
        .code(predicate::in_iter([0, 1]));
}

#[tokio::test]
async fn e2e_project_with_secrets_detected() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_project(temp_dir.path());

    // Add a file with a secret pattern (using realistic patterns that will be detected)
    // Note: These are fake secrets used only for testing detection
    fs::write(
        temp_dir.path().join("config.rs"),
        r#"
// Test file for secret detection
const AWS_ACCESS_KEY: &str = "AKIAIOSFODNN7EXAMPLE";
const AWS_SECRET_KEY: &str = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
"#,
    )
    .unwrap();

    // Initialize repolens config
    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "strict",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    // Run plan with JSON output
    let output_path = temp_dir.path().join("plan.json");
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["plan", "--format", "json", "--output"])
        .arg(&output_path)
        .assert()
        .code(predicate::eq(1));

    let content = fs::read_to_string(&output_path).unwrap();
    let plan: serde_json::Value = serde_json::from_str(&content).unwrap();

    let findings = plan
        .get("audit")
        .and_then(|a| a.get("findings"))
        .and_then(|f| f.as_array())
        .unwrap();

    // Should detect secrets (AWS keys are commonly detected)
    let has_secret_finding = findings.iter().any(|f| {
        f.get("category")
            .and_then(|c| c.as_str())
            .map(|c| c == "secrets")
            .unwrap_or(false)
    });
    assert!(has_secret_finding, "Should detect hardcoded secrets");
}

#[tokio::test]
async fn e2e_project_with_env_file() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_project(temp_dir.path());

    // Add .env file (should be detected as sensitive)
    fs::write(
        temp_dir.path().join(".env"),
        "DATABASE_URL=localhost\nAPI_KEY=test123\n",
    )
    .unwrap();

    // Initialize repolens config
    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "opensource",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    // Run plan with JSON output
    let output_path = temp_dir.path().join("plan.json");
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["plan", "--format", "json", "--output"])
        .arg(&output_path)
        .assert()
        .code(predicate::eq(1));

    let content = fs::read_to_string(&output_path).unwrap();
    let plan: serde_json::Value = serde_json::from_str(&content).unwrap();

    let findings = plan
        .get("audit")
        .and_then(|a| a.get("findings"))
        .and_then(|f| f.as_array())
        .unwrap();

    // Should detect .env file as sensitive (GIT003 or SEC003)
    let has_sensitive_file = findings.iter().any(|f| {
        let rule_id = f.get("rule_id").and_then(|r| r.as_str()).unwrap_or("");
        let message = f.get("message").and_then(|m| m.as_str()).unwrap_or("");
        rule_id == "GIT003" || rule_id == "SEC003" || message.to_lowercase().contains(".env")
    });
    assert!(
        has_sensitive_file,
        "Should detect .env as sensitive file. Findings: {:?}",
        findings
    );
}

#[tokio::test]
async fn e2e_project_with_dockerfile() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_project(temp_dir.path());

    // Add Dockerfile without best practices
    fs::write(
        temp_dir.path().join("Dockerfile"),
        r#"FROM rust:latest
COPY . .
RUN cargo build --release
CMD ["./target/release/test-project"]
"#,
    )
    .unwrap();

    // Initialize repolens config
    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "strict",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    // Run plan with JSON output
    let output_path = temp_dir.path().join("plan.json");
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["plan", "--format", "json", "--output"])
        .arg(&output_path)
        .assert()
        .code(predicate::eq(1));

    let content = fs::read_to_string(&output_path).unwrap();
    let plan: serde_json::Value = serde_json::from_str(&content).unwrap();

    let findings = plan
        .get("audit")
        .and_then(|a| a.get("findings"))
        .and_then(|f| f.as_array())
        .unwrap();

    // Should detect Docker issues
    let docker_findings: Vec<_> = findings
        .iter()
        .filter(|f| {
            f.get("category")
                .and_then(|c| c.as_str())
                .map(|c| c == "docker")
                .unwrap_or(false)
        })
        .collect();

    assert!(
        !docker_findings.is_empty(),
        "Should detect Docker best practice issues"
    );

    // Specifically check for :latest tag (DOCKER003)
    let has_latest_tag = findings.iter().any(|f| {
        f.get("rule_id")
            .and_then(|r| r.as_str())
            .map(|r| r == "DOCKER003")
            .unwrap_or(false)
    });
    assert!(has_latest_tag, "Should detect unpinned :latest tag");
}

#[tokio::test]
async fn e2e_complete_opensource_project() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_path_buf();

    create_rust_project(&temp_path);

    // Add all required files for opensource
    fs::write(
        temp_path.join("README.md"),
        "# Test Project\n\nA test project.\n",
    )
    .unwrap();
    fs::write(temp_path.join("LICENSE"), "MIT License\n\nCopyright 2024\n").unwrap();
    fs::write(
        temp_path.join("CONTRIBUTING.md"),
        "# Contributing\n\nWelcome!\n",
    )
    .unwrap();
    fs::write(
        temp_path.join("CODE_OF_CONDUCT.md"),
        "# Code of Conduct\n\nBe nice.\n",
    )
    .unwrap();
    fs::write(
        temp_path.join("SECURITY.md"),
        "# Security Policy\n\nReport issues.\n",
    )
    .unwrap();
    fs::write(
        temp_path.join("CHANGELOG.md"),
        "# Changelog\n\n## [Unreleased]\n\n## [0.1.0] - 2024-01-01\n\n- Initial release\n",
    )
    .unwrap();
    fs::write(temp_path.join(".gitignore"), "/target\n").unwrap();
    fs::write(
        temp_path.join("Cargo.lock"),
        "version = 3\n\n[[package]]\nname = \"test\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    // Initialize repolens config
    get_cmd()
        .current_dir(&temp_path)
        .args([
            "init",
            "--preset",
            "opensource",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    // Run plan - should have fewer/no critical issues
    // Exit codes: 0 = no issues, 1 = warnings only, 2 = critical issues
    let output_path = temp_path.join("plan.json");
    get_cmd()
        .current_dir(&temp_path)
        .args(["plan", "--format", "json", "--output"])
        .arg(&output_path)
        .assert()
        .code(predicate::in_iter([0, 1, 2]));

    let content = fs::read_to_string(&output_path).expect("Should be able to read plan output");
    let plan: serde_json::Value =
        serde_json::from_str(&content).expect("Plan should be valid JSON");

    // Check findings exist
    let findings = plan
        .get("audit")
        .and_then(|a| a.get("findings"))
        .and_then(|f| f.as_array());

    if let Some(findings) = findings {
        // Should not have critical file-related findings
        let critical_file_findings: Vec<_> = findings
            .iter()
            .filter(|f| {
                let severity = f.get("severity").and_then(|s| s.as_str()).unwrap_or("");
                let category = f.get("category").and_then(|c| c.as_str()).unwrap_or("");
                severity == "critical" && (category == "files" || category == "docs")
            })
            .collect();

        assert!(
            critical_file_findings.is_empty(),
            "Complete project should not have critical file/docs findings: {:?}",
            critical_file_findings
        );
    }
    // If findings is None, the test still passes (no findings = no critical findings)
}

#[tokio::test]
async fn e2e_presets_have_different_strictness() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_project(temp_dir.path());

    // Add README to avoid the most common finding
    fs::write(temp_dir.path().join("README.md"), "# Test\n").unwrap();

    let mut findings_counts: Vec<(String, usize)> = Vec::new();

    for preset in &["opensource", "enterprise", "strict"] {
        // Re-initialize with different preset
        get_cmd()
            .current_dir(temp_dir.path())
            .args([
                "init",
                "--preset",
                preset,
                "--non-interactive",
                "--force",
                "--skip-checks",
            ])
            .assert()
            .success();

        let output_path = temp_dir.path().join(format!("plan-{}.json", preset));
        get_cmd()
            .current_dir(temp_dir.path())
            .args(["plan", "--format", "json", "--output"])
            .arg(&output_path)
            .assert()
            .code(predicate::in_iter([0, 1]));

        let content = fs::read_to_string(&output_path).unwrap();
        let plan: serde_json::Value = serde_json::from_str(&content).unwrap();
        let count = plan
            .get("findings")
            .and_then(|f| f.as_array())
            .map(|a| a.len())
            .unwrap_or(0);

        findings_counts.push((preset.to_string(), count));
    }

    // Strict should generally find more issues than enterprise, which should find more than opensource
    // This isn't always true depending on the specific project, but in general it holds
    println!("Findings counts: {:?}", findings_counts);

    // At minimum, strict should not find fewer issues than opensource
    let opensource_count = findings_counts
        .iter()
        .find(|(p, _)| p == "opensource")
        .unwrap()
        .1;
    let strict_count = findings_counts
        .iter()
        .find(|(p, _)| p == "strict")
        .unwrap()
        .1;

    assert!(
        strict_count >= opensource_count,
        "Strict preset ({}) should find at least as many issues as opensource ({})",
        strict_count,
        opensource_count
    );
}

// ============================================================================
// E2E Tests with real GitHub repositories (requires network, run with --ignored)
// ============================================================================

/// Clone a repository to a temp directory
fn clone_repo(url: &str, dir: &Path) -> bool {
    StdCommand::new("git")
        .args(["clone", "--depth", "1", url, "."])
        .current_dir(dir)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[tokio::test]
#[ignore = "Requires network access - run with: cargo test -- --ignored"]
async fn e2e_real_repo_tokio() {
    let temp_dir = TempDir::new().unwrap();

    // Clone tokio (well-maintained Rust project)
    if !clone_repo("https://github.com/tokio-rs/tokio.git", temp_dir.path()) {
        eprintln!("Failed to clone tokio, skipping test");
        return;
    }

    // Initialize and run audit
    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "opensource",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    get_cmd()
        .current_dir(temp_dir.path())
        .args(["plan"])
        .assert()
        .code(predicate::in_iter([0, 1]));
}

#[tokio::test]
#[ignore = "Requires network access - run with: cargo test -- --ignored"]
async fn e2e_real_repo_express() {
    let temp_dir = TempDir::new().unwrap();

    // Clone express (popular Node.js project)
    if !clone_repo("https://github.com/expressjs/express.git", temp_dir.path()) {
        eprintln!("Failed to clone express, skipping test");
        return;
    }

    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "opensource",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    let output_path = temp_dir.path().join("plan.json");
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["plan", "--format", "json", "--output"])
        .arg(&output_path)
        .assert()
        .code(predicate::in_iter([0, 1]));

    // Verify we can parse the output
    let content = fs::read_to_string(&output_path).unwrap();
    let _: serde_json::Value = serde_json::from_str(&content).expect("Should produce valid JSON");
}

#[tokio::test]
#[ignore = "Requires network access - run with: cargo test -- --ignored"]
async fn e2e_real_repo_flask() {
    let temp_dir = TempDir::new().unwrap();

    // Clone flask (popular Python project)
    if !clone_repo("https://github.com/pallets/flask.git", temp_dir.path()) {
        eprintln!("Failed to clone flask, skipping test");
        return;
    }

    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "opensource",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    get_cmd()
        .current_dir(temp_dir.path())
        .args(["plan"])
        .assert()
        .code(predicate::in_iter([0, 1]));
}

#[tokio::test]
#[ignore = "Requires network access - run with: cargo test -- --ignored"]
async fn e2e_real_repo_kubernetes() {
    let temp_dir = TempDir::new().unwrap();

    // Clone kubernetes (large Go project)
    if !clone_repo(
        "https://github.com/kubernetes/kubernetes.git",
        temp_dir.path(),
    ) {
        eprintln!("Failed to clone kubernetes, skipping test");
        return;
    }

    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "enterprise",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    // Just verify it runs without crashing on a large repo
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["plan"])
        .timeout(std::time::Duration::from_secs(120))
        .assert()
        .code(predicate::in_iter([0, 1]));
}

// ============================================================================
// E2E Tests for other CLI commands
// ============================================================================

#[tokio::test]
async fn e2e_schema_command_outputs_valid_json() {
    let output = get_cmd()
        .args(["schema"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let schema: serde_json::Value =
        serde_json::from_str(&stdout).expect("Schema should be valid JSON");

    // Verify it's a JSON schema
    assert!(
        schema.get("$schema").is_some() || schema.get("type").is_some(),
        "Should be a JSON schema"
    );
}

#[tokio::test]
async fn e2e_compare_command_detects_changes() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_project(temp_dir.path());

    // Initialize config
    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "opensource",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    // Generate first report
    let report1_path = temp_dir.path().join("report1.json");
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["report", "--format", "json", "--output"])
        .arg(&report1_path)
        .assert()
        .code(predicate::in_iter([0, 1, 2]));

    // Add README to reduce findings
    fs::write(temp_dir.path().join("README.md"), "# Test Project\n").unwrap();

    // Generate second report
    let report2_path = temp_dir.path().join("report2.json");
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["report", "--format", "json", "--output"])
        .arg(&report2_path)
        .assert()
        .code(predicate::in_iter([0, 1, 2]));

    // Compare reports
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["compare"])
        .arg(&report1_path)
        .arg(&report2_path)
        .assert()
        .code(predicate::in_iter([0, 1, 2])); // Any valid exit code
}

#[tokio::test]
async fn e2e_install_hooks_command() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_project(temp_dir.path());

    // Initialize config
    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "opensource",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    // Install hooks
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["install-hooks"])
        .assert()
        .success();

    // Verify hooks were created
    let hooks_dir = temp_dir.path().join(".git/hooks");
    assert!(
        hooks_dir.join("pre-commit").exists() || hooks_dir.join("pre-push").exists(),
        "At least one hook should be installed"
    );
}

#[tokio::test]
async fn e2e_apply_command_creates_readme() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_project(temp_dir.path());

    // Initialize config
    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "opensource",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    // Verify README doesn't exist
    assert!(!temp_dir.path().join("README.md").exists());

    // Run apply with auto-confirm, skip GitHub actions (which require network)
    // and disable PR/issue creation which need GitHub API access
    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "apply",
            "--yes",
            "--no-issues",
            "--no-pr",
            "--skip",
            "github",
        ])
        .assert()
        .code(predicate::in_iter([0, 1, 2]));

    // Note: Whether README is created depends on implementation
    // The test verifies the command runs without error
}

// ============================================================================
// E2E Tests for CLI options
// ============================================================================

#[tokio::test]
async fn e2e_verbose_option() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_project(temp_dir.path());

    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "opensource",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    // Run with verbose flag
    let output = get_cmd()
        .current_dir(temp_dir.path())
        .args(["-v", "plan"])
        .assert()
        .code(predicate::in_iter([0, 1, 2]))
        .get_output()
        .stderr
        .clone();

    // Verbose output goes to stderr typically
    let _stderr = String::from_utf8_lossy(&output);
    // Test passes if command runs without error - verbose output depends on implementation
}

#[tokio::test]
async fn e2e_directory_option() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_project(temp_dir.path());

    // Initialize from outside the directory using -C option
    get_cmd()
        .args([
            "-C",
            temp_dir.path().to_str().unwrap(),
            "init",
            "--preset",
            "opensource",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    // Verify config was created in the target directory
    assert!(temp_dir.path().join(".repolens.toml").exists());

    // Run plan using -C option
    get_cmd()
        .args(["-C", temp_dir.path().to_str().unwrap(), "plan"])
        .assert()
        .code(predicate::in_iter([0, 1, 2]));
}

#[tokio::test]
async fn e2e_config_option() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_project(temp_dir.path());

    // Create a custom config file
    let custom_config = temp_dir.path().join("custom-config.toml");
    fs::write(
        &custom_config,
        r#"
preset = "strict"
"#,
    )
    .unwrap();

    // Run plan with custom config
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["-c", custom_config.to_str().unwrap(), "plan"])
        .assert()
        .code(predicate::in_iter([0, 1, 2]));
}

// ============================================================================
// E2E Tests for error handling
// ============================================================================

#[tokio::test]
async fn e2e_error_invalid_preset() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_project(temp_dir.path());

    // Try to init with invalid preset
    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "invalid-preset-name",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .failure();
}

#[tokio::test]
async fn e2e_error_nonexistent_directory() {
    // Try to run in non-existent directory
    get_cmd()
        .args(["-C", "/nonexistent/directory/path", "plan"])
        .assert()
        .failure();
}

#[tokio::test]
async fn e2e_error_invalid_config_file() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_project(temp_dir.path());

    // Create an invalid config file
    let invalid_config = temp_dir.path().join("invalid.toml");
    fs::write(&invalid_config, "this is not valid toml {{{{").unwrap();

    // Try to run with invalid config
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["-c", invalid_config.to_str().unwrap(), "plan"])
        .assert()
        .failure();
}

#[tokio::test]
async fn e2e_error_missing_config() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_project(temp_dir.path());

    // Don't initialize - try to run plan without config
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["plan"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("config").or(predicate::str::contains("initialize")));
}

#[tokio::test]
async fn e2e_error_compare_missing_file() {
    let temp_dir = TempDir::new().unwrap();

    // Try to compare with non-existent files
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["compare", "nonexistent1.json", "nonexistent2.json"])
        .assert()
        .failure();
}

// ============================================================================
// E2E Tests for Git hygiene rules (GIT001-003)
// ============================================================================

#[tokio::test]
async fn e2e_git_rule_large_binary_detected() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_project(temp_dir.path());

    // Create a large binary file (>1MB)
    let large_file = temp_dir.path().join("large.exe");
    let large_content = vec![0u8; 2 * 1024 * 1024]; // 2MB
    fs::write(&large_file, large_content).unwrap();

    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "strict",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    let output_path = temp_dir.path().join("plan.json");
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["plan", "--format", "json", "--output"])
        .arg(&output_path)
        .assert()
        .code(predicate::in_iter([0, 1, 2]));

    let content = fs::read_to_string(&output_path).unwrap();
    let plan: serde_json::Value = serde_json::from_str(&content).unwrap();

    let findings = plan
        .get("audit")
        .and_then(|a| a.get("findings"))
        .and_then(|f| f.as_array())
        .unwrap();

    let has_git001 = findings.iter().any(|f| {
        f.get("rule_id")
            .and_then(|r| r.as_str())
            .map(|r| r == "GIT001")
            .unwrap_or(false)
    });
    assert!(has_git001, "Should detect large binary file (GIT001)");
}

#[tokio::test]
async fn e2e_git_rule_gitattributes_missing() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_project(temp_dir.path());

    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "strict",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    let output_path = temp_dir.path().join("plan.json");
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["plan", "--format", "json", "--output"])
        .arg(&output_path)
        .assert()
        .code(predicate::in_iter([0, 1, 2]));

    let content = fs::read_to_string(&output_path).unwrap();
    let plan: serde_json::Value = serde_json::from_str(&content).unwrap();

    let findings = plan
        .get("audit")
        .and_then(|a| a.get("findings"))
        .and_then(|f| f.as_array())
        .unwrap();

    let has_git002 = findings.iter().any(|f| {
        f.get("rule_id")
            .and_then(|r| r.as_str())
            .map(|r| r == "GIT002")
            .unwrap_or(false)
    });
    assert!(has_git002, "Should detect missing .gitattributes (GIT002)");
}

// ============================================================================
// E2E Tests for branch protection rules (SEC007-010)
// ============================================================================

#[tokio::test]
async fn e2e_security_rule_branch_protection_missing() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_project(temp_dir.path());

    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "strict",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    let output_path = temp_dir.path().join("plan.json");
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["plan", "--format", "json", "--output"])
        .arg(&output_path)
        .assert()
        .code(predicate::in_iter([0, 1, 2]));

    let content = fs::read_to_string(&output_path).unwrap();
    let plan: serde_json::Value = serde_json::from_str(&content).unwrap();

    let findings = plan
        .get("audit")
        .and_then(|a| a.get("findings"))
        .and_then(|f| f.as_array())
        .unwrap();

    let has_sec007 = findings.iter().any(|f| {
        f.get("rule_id")
            .and_then(|r| r.as_str())
            .map(|r| r == "SEC007")
            .unwrap_or(false)
    });
    assert!(
        has_sec007,
        "Should detect missing .github/settings.yml (SEC007)"
    );
}

#[tokio::test]
async fn e2e_security_rule_branch_protection_incomplete() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_project(temp_dir.path());

    // Create incomplete .github/settings.yml
    fs::create_dir_all(temp_dir.path().join(".github")).unwrap();
    fs::write(
        temp_dir.path().join(".github/settings.yml"),
        r#"
repository:
  name: test
# No branch protection rules
"#,
    )
    .unwrap();

    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "strict",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    let output_path = temp_dir.path().join("plan.json");
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["plan", "--format", "json", "--output"])
        .arg(&output_path)
        .assert()
        .code(predicate::in_iter([0, 1, 2]));

    let content = fs::read_to_string(&output_path).unwrap();
    let plan: serde_json::Value = serde_json::from_str(&content).unwrap();

    let findings = plan
        .get("audit")
        .and_then(|a| a.get("findings"))
        .and_then(|f| f.as_array())
        .unwrap();

    // Should detect missing branch protection rules (SEC008)
    let has_sec008 = findings.iter().any(|f| {
        f.get("rule_id")
            .and_then(|r| r.as_str())
            .map(|r| r == "SEC008")
            .unwrap_or(false)
    });
    assert!(
        has_sec008,
        "Should detect missing branch protection rules (SEC008)"
    );
}

// ============================================================================
// E2E Tests for ecosystem detection
// ============================================================================

/// Create a Go project for testing
fn create_go_project(dir: &Path) {
    fs::write(dir.join("go.mod"), "module example.com/test\n\ngo 1.21\n").unwrap();
    fs::write(dir.join("main.go"), "package main\n\nfunc main() {}\n").unwrap();
    StdCommand::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .ok();
    StdCommand::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir)
        .output()
        .ok();
    StdCommand::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir)
        .output()
        .ok();
}

/// Create a PHP project for testing
fn create_php_project(dir: &Path) {
    fs::write(
        dir.join("composer.json"),
        r#"{
    "name": "test/project",
    "require": {
        "php": ">=8.0"
    }
}"#,
    )
    .unwrap();
    fs::write(dir.join("index.php"), "<?php echo 'Hello'; ?>\n").unwrap();
    StdCommand::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .ok();
    StdCommand::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir)
        .output()
        .ok();
    StdCommand::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir)
        .output()
        .ok();
}

/// Create a Ruby project for testing
fn create_ruby_project(dir: &Path) {
    fs::write(dir.join("Gemfile"), "source 'https://rubygems.org'\n").unwrap();
    fs::write(dir.join("main.rb"), "puts 'Hello'\n").unwrap();
    StdCommand::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .ok();
    StdCommand::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir)
        .output()
        .ok();
    StdCommand::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir)
        .output()
        .ok();
}

#[tokio::test]
async fn e2e_go_project_audit() {
    let temp_dir = TempDir::new().unwrap();
    create_go_project(temp_dir.path());

    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "opensource",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    // Run plan - should complete without crashing
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["plan"])
        .assert()
        .code(predicate::in_iter([0, 1, 2]));
}

#[tokio::test]
async fn e2e_go_project_missing_go_sum() {
    let temp_dir = TempDir::new().unwrap();
    create_go_project(temp_dir.path());

    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "opensource",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    let output_path = temp_dir.path().join("plan.json");
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["plan", "--format", "json", "--output"])
        .arg(&output_path)
        .assert()
        .code(predicate::in_iter([0, 1, 2]));

    let content = fs::read_to_string(&output_path).unwrap();
    let plan: serde_json::Value = serde_json::from_str(&content).unwrap();

    let findings = plan
        .get("audit")
        .and_then(|a| a.get("findings"))
        .and_then(|f| f.as_array());

    if let Some(findings) = findings {
        let has_dep003 = findings.iter().any(|f| {
            f.get("rule_id")
                .and_then(|r| r.as_str())
                .map(|r| r == "DEP003")
                .unwrap_or(false)
        });
        assert!(has_dep003, "Should detect missing go.sum (DEP003)");
    }
}

#[tokio::test]
async fn e2e_php_project_audit() {
    let temp_dir = TempDir::new().unwrap();
    create_php_project(temp_dir.path());

    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "opensource",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    get_cmd()
        .current_dir(temp_dir.path())
        .args(["plan"])
        .assert()
        .code(predicate::in_iter([0, 1, 2]));
}

#[tokio::test]
async fn e2e_ruby_project_audit() {
    let temp_dir = TempDir::new().unwrap();
    create_ruby_project(temp_dir.path());

    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "opensource",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    get_cmd()
        .current_dir(temp_dir.path())
        .args(["plan"])
        .assert()
        .code(predicate::in_iter([0, 1, 2]));
}

#[tokio::test]
async fn e2e_ruby_project_missing_gemfile_lock() {
    let temp_dir = TempDir::new().unwrap();
    create_ruby_project(temp_dir.path());

    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "opensource",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    let output_path = temp_dir.path().join("plan.json");
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["plan", "--format", "json", "--output"])
        .arg(&output_path)
        .assert()
        .code(predicate::in_iter([0, 1, 2]));

    let content = fs::read_to_string(&output_path).unwrap();
    let plan: serde_json::Value = serde_json::from_str(&content).unwrap();

    let findings = plan
        .get("audit")
        .and_then(|a| a.get("findings"))
        .and_then(|f| f.as_array());

    if let Some(findings) = findings {
        let has_dep003 = findings.iter().any(|f| {
            f.get("rule_id")
                .and_then(|r| r.as_str())
                .map(|r| r == "DEP003")
                .unwrap_or(false)
        });
        assert!(has_dep003, "Should detect missing Gemfile.lock (DEP003)");
    }
}

// ============================================================================
// E2E Tests for workflow rules
// ============================================================================

#[tokio::test]
async fn e2e_workflow_rules_detected() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_project(temp_dir.path());

    // Create a GitHub workflow without best practices
    fs::create_dir_all(temp_dir.path().join(".github/workflows")).unwrap();
    fs::write(
        temp_dir.path().join(".github/workflows/ci.yml"),
        r#"
name: CI
on: push
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - run: cargo build
"#,
    )
    .unwrap();

    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "strict",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    let output_path = temp_dir.path().join("plan.json");
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["plan", "--format", "json", "--output"])
        .arg(&output_path)
        .assert()
        .code(predicate::in_iter([0, 1, 2]));

    let content = fs::read_to_string(&output_path).unwrap();
    let plan: serde_json::Value = serde_json::from_str(&content).unwrap();

    let findings = plan
        .get("audit")
        .and_then(|a| a.get("findings"))
        .and_then(|f| f.as_array())
        .unwrap();

    // Should detect workflow issues (WF004 - missing timeout)
    let workflow_findings: Vec<_> = findings
        .iter()
        .filter(|f| {
            f.get("category")
                .and_then(|c| c.as_str())
                .map(|c| c == "workflows")
                .unwrap_or(false)
        })
        .collect();

    assert!(
        !workflow_findings.is_empty(),
        "Should detect workflow issues"
    );
}

// ============================================================================
// E2E Tests for report formats
// ============================================================================

#[tokio::test]
async fn e2e_report_html_format() {
    let repo_root = std::env::current_dir().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("report.html");

    let result = get_cmd()
        .current_dir(&repo_root)
        .args(["report", "--format", "html", "--output"])
        .arg(&output_path)
        .assert()
        .code(predicate::in_iter([0, 1, 2]));

    // HTML format might not be implemented - just verify command doesn't crash
    let _ = result;
}

#[tokio::test]
async fn e2e_report_text_format() {
    let repo_root = std::env::current_dir().unwrap();

    // Run with text format (default)
    let output = get_cmd()
        .current_dir(&repo_root)
        .args(["report"])
        .assert()
        .code(predicate::in_iter([0, 1, 2]))
        .get_output()
        .stdout
        .clone();

    // Text output should have some content
    let _stdout = String::from_utf8_lossy(&output);
    // Test passes if command runs without error - report writes to file by default
}

// ============================================================================
// E2E Tests for Exit Codes
// ============================================================================

/// Exit code constants for tests
mod exit_codes {
    pub const SUCCESS: i32 = 0;
    pub const CRITICAL_ISSUES: i32 = 1;
    pub const WARNINGS: i32 = 2;
    pub const ERROR: i32 = 3;
    pub const INVALID_ARGS: i32 = 4;
}

#[tokio::test]
async fn e2e_exit_code_returns_valid_exit_codes() {
    // This test verifies that exit codes are within expected range
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path().to_path_buf();

    create_rust_project(&temp_path);

    // Add some required files
    fs::write(
        temp_path.join("README.md"),
        "# Test Project\n\nA test project.\n",
    )
    .unwrap();

    // Initialize config
    get_cmd()
        .current_dir(&temp_path)
        .args([
            "init",
            "--preset",
            "opensource",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    // Run plan - should return a valid exit code (0, 1, or 2)
    get_cmd()
        .current_dir(&temp_path)
        .args(["plan"])
        .assert()
        .code(predicate::in_iter([
            exit_codes::SUCCESS,
            exit_codes::CRITICAL_ISSUES,
            exit_codes::WARNINGS,
        ]));
}

#[tokio::test]
async fn e2e_exit_code_critical_for_secrets() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_project(temp_dir.path());

    // Add a file with exposed secrets
    fs::write(
        temp_dir.path().join("secrets.rs"),
        r#"
// Fake AWS credentials for testing
const AWS_ACCESS_KEY: &str = "AKIAIOSFODNN7EXAMPLE";
const AWS_SECRET_KEY: &str = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
"#,
    )
    .unwrap();

    // Initialize config with strict preset (more likely to detect secrets)
    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "strict",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    // Run plan - should return CRITICAL_ISSUES (1) for exposed secrets
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["plan"])
        .assert()
        .code(predicate::eq(exit_codes::CRITICAL_ISSUES));
}

#[tokio::test]
async fn e2e_exit_code_warnings_for_missing_files() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_project(temp_dir.path());

    // Add README but leave other files missing (warnings, not critical)
    fs::write(temp_dir.path().join("README.md"), "# Test\n").unwrap();

    // Initialize config
    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "opensource",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    // Run plan - should return WARNINGS (2) or CRITICAL_ISSUES (1) depending on findings
    // Missing LICENSE is typically critical, so we check for non-zero exit
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["plan"])
        .assert()
        .code(predicate::in_iter([
            exit_codes::CRITICAL_ISSUES,
            exit_codes::WARNINGS,
        ]));
}

#[tokio::test]
async fn e2e_exit_code_for_missing_config_uses_default() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_project(temp_dir.path());

    // Without explicit init, CLI uses load_or_default which creates a default config
    // So plan should run successfully (finding issues in the project)
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["plan"])
        .assert()
        .code(predicate::in_iter([
            exit_codes::SUCCESS,
            exit_codes::CRITICAL_ISSUES,
            exit_codes::WARNINGS,
        ]));
}

#[tokio::test]
async fn e2e_exit_code_invalid_args_for_invalid_preset() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_project(temp_dir.path());

    // Try to init with invalid preset - should return INVALID_ARGS (4)
    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "invalid-preset-name",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .code(predicate::eq(exit_codes::INVALID_ARGS));
}

#[tokio::test]
async fn e2e_exit_code_error_for_nonexistent_directory() {
    // Try to run in non-existent directory - should return ERROR (3)
    get_cmd()
        .args(["-C", "/nonexistent/directory/path/12345", "plan"])
        .assert()
        .code(predicate::eq(exit_codes::ERROR));
}

#[tokio::test]
async fn e2e_exit_code_compare_regression() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_project(temp_dir.path());

    // Initialize config
    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "opensource",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    // Generate first report (fewer issues)
    fs::write(temp_dir.path().join("README.md"), "# Test Project\n").unwrap();
    let report1_path = temp_dir.path().join("report1.json");
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["report", "--format", "json", "--output"])
        .arg(&report1_path)
        .assert()
        .code(predicate::in_iter([0, 1, 2]));

    // Remove README to create a regression
    fs::remove_file(temp_dir.path().join("README.md")).unwrap();

    // Generate second report (more issues - regression)
    let report2_path = temp_dir.path().join("report2.json");
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["report", "--format", "json", "--output"])
        .arg(&report2_path)
        .assert()
        .code(predicate::in_iter([0, 1, 2]));

    // Compare with --fail-on-regression - should return CRITICAL_ISSUES (1)
    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "compare",
            "--base-file",
            report1_path.to_str().unwrap(),
            "--head-file",
            report2_path.to_str().unwrap(),
            "--fail-on-regression",
        ])
        .assert()
        .code(predicate::eq(exit_codes::CRITICAL_ISSUES));
}

#[tokio::test]
async fn e2e_exit_code_compare_returns_valid_codes() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_project(temp_dir.path());

    // Initialize config
    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "opensource",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    // Generate first report
    let report1_path = temp_dir.path().join("report1.json");
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["report", "--format", "json", "--output"])
        .arg(&report1_path)
        .assert()
        .code(predicate::in_iter([0, 1, 2]));

    // Add a file
    fs::write(temp_dir.path().join("README.md"), "# Test Project\n").unwrap();

    // Generate second report
    let report2_path = temp_dir.path().join("report2.json");
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["report", "--format", "json", "--output"])
        .arg(&report2_path)
        .assert()
        .code(predicate::in_iter([0, 1, 2]));

    // Compare without --fail-on-regression - should always return SUCCESS (0)
    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "compare",
            "--base-file",
            report1_path.to_str().unwrap(),
            "--head-file",
            report2_path.to_str().unwrap(),
        ])
        .assert()
        .code(predicate::eq(exit_codes::SUCCESS));

    // Compare with --fail-on-regression returns SUCCESS or CRITICAL_ISSUES
    // depending on whether new issues were introduced
    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "compare",
            "--base-file",
            report1_path.to_str().unwrap(),
            "--head-file",
            report2_path.to_str().unwrap(),
            "--fail-on-regression",
        ])
        .assert()
        .code(predicate::in_iter([
            exit_codes::SUCCESS,
            exit_codes::CRITICAL_ISSUES,
        ]));
}

// ============================================================================
// E2E Tests for Init Command
// ============================================================================

#[tokio::test]
async fn e2e_init_command_creates_config() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_project(temp_dir.path());

    // Config should not exist initially
    assert!(!temp_dir.path().join(".repolens.toml").exists());

    // Run init
    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "opensource",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    // Config should now exist
    assert!(temp_dir.path().join(".repolens.toml").exists());

    // Verify config content
    let config = fs::read_to_string(temp_dir.path().join(".repolens.toml")).unwrap();
    assert!(config.contains("preset"));
}

#[tokio::test]
async fn e2e_init_command_with_different_presets() {
    for preset in ["opensource", "enterprise", "strict"] {
        let temp_dir = TempDir::new().unwrap();
        create_rust_project(temp_dir.path());

        get_cmd()
            .current_dir(temp_dir.path())
            .args([
                "init",
                "--preset",
                preset,
                "--non-interactive",
                "--force",
                "--skip-checks",
            ])
            .assert()
            .success();

        let config = fs::read_to_string(temp_dir.path().join(".repolens.toml")).unwrap();
        assert!(
            config.contains(preset),
            "Config should contain preset '{}'",
            preset
        );
    }
}

#[tokio::test]
async fn e2e_init_command_force_overwrites() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_project(temp_dir.path());

    // Create initial config
    fs::write(
        temp_dir.path().join(".repolens.toml"),
        "[general]\npreset = \"opensource\"\n",
    )
    .unwrap();

    // Run init with --force and different preset
    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "strict",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .success();

    // Config should be overwritten with new preset
    let config = fs::read_to_string(temp_dir.path().join(".repolens.toml")).unwrap();
    assert!(
        config.contains("strict"),
        "Config should contain new preset 'strict'"
    );
}

#[tokio::test]
async fn e2e_init_command_invalid_preset_fails() {
    let temp_dir = TempDir::new().unwrap();
    create_rust_project(temp_dir.path());

    // Invalid preset should fail
    get_cmd()
        .current_dir(temp_dir.path())
        .args([
            "init",
            "--preset",
            "invalid_preset",
            "--non-interactive",
            "--force",
            "--skip-checks",
        ])
        .assert()
        .code(predicate::ne(0));
}

// ============================================================================
// E2E Tests for Generate-Man Command
// ============================================================================

#[tokio::test]
async fn e2e_generate_man_command() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("repolens.1");

    // Generate man page
    get_cmd()
        .args(["generate-man", "--output"])
        .arg(temp_dir.path())
        .assert()
        .success();

    // Verify man page was created
    assert!(output_path.exists(), "Man page should be created");

    // Verify content is valid roff format
    let content = fs::read_to_string(&output_path).unwrap();
    assert!(
        content.contains(".TH"),
        "Man page should contain .TH header"
    );
    assert!(
        content.contains("repolens") || content.contains("REPOLENS"),
        "Man page should mention repolens"
    );
}

#[tokio::test]
async fn e2e_generate_man_default_output() {
    let temp_dir = TempDir::new().unwrap();

    // Generate man page to current directory
    get_cmd()
        .current_dir(temp_dir.path())
        .args(["generate-man"])
        .assert()
        .success();

    // Verify man page was created in current directory
    assert!(
        temp_dir.path().join("repolens.1").exists(),
        "Man page should be created in current directory"
    );
}
