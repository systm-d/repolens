//! Git hygiene rules
//!
//! This module provides rules for checking Git repository best practices, including:
//! - Large binary files detection (GIT001)
//! - .gitattributes presence (GIT002)
//! - Sensitive files tracked in repository (GIT003)

use crate::config::Config;
use crate::error::RepoLensError;
use crate::rules::engine::RuleCategory;
use crate::rules::results::{Finding, Severity};
use crate::scanner::Scanner;

/// Binary file extensions to check for large files
const BINARY_EXTENSIONS: &[&str] = &[
    "exe", "dll", "so", "dylib", "zip", "tar", "gz", "png", "jpg", "jpeg", "mp4", "pdf", "jar",
    "whl", "a", "lib", "bin", "o", "obj",
];

/// Sensitive file patterns to check
const SENSITIVE_PATTERNS: &[&str] = &[
    ".env",
    "*.key",
    "*.pem",
    "credentials*",
    "secrets*",
    "*_rsa",
    "*.p12",
];

/// Size threshold for large binary files (1 MB)
const LARGE_FILE_THRESHOLD: u64 = 1024 * 1024;

/// Rules for checking Git repository best practices
pub struct GitRules;

#[async_trait::async_trait]
impl RuleCategory for GitRules {
    fn name(&self) -> &'static str {
        "git"
    }

    async fn run(&self, scanner: &Scanner, config: &Config) -> Result<Vec<Finding>, RepoLensError> {
        let mut findings = Vec::new();

        if config.is_rule_enabled("git/large-binaries") {
            findings.extend(check_large_binaries(scanner).await?);
        }

        if config.is_rule_enabled("git/gitattributes") {
            findings.extend(check_gitattributes(scanner).await?);
        }

        if config.is_rule_enabled("git/sensitive-files") {
            findings.extend(check_sensitive_files(scanner).await?);
        }

        Ok(findings)
    }
}

/// GIT001: Check for large binary files (> 1MB)
async fn check_large_binaries(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    // Get files larger than threshold
    let large_files = scanner.files_larger_than(LARGE_FILE_THRESHOLD);

    for file_info in large_files {
        // Check if file has a binary extension
        let path_lower = file_info.path.to_lowercase();
        let is_binary = BINARY_EXTENSIONS
            .iter()
            .any(|ext| path_lower.ends_with(&format!(".{}", ext)));

        if is_binary {
            let size_mb = file_info.size as f64 / (1024.0 * 1024.0);
            findings.push(
                Finding::new(
                    "GIT001",
                    "git",
                    Severity::Warning,
                    format!(
                        "Large binary file '{}' ({:.2} MB) detected",
                        file_info.path, size_mb
                    ),
                )
                .with_location(&file_info.path)
                .with_description(
                    "Large binary files in Git repositories increase clone time, \
                     consume disk space, and cannot be efficiently diffed. \
                     Git is designed for text files and handles binaries poorly.",
                )
                .with_remediation(
                    "Consider using Git LFS (Large File Storage) for binary files, \
                     or store them in an external artifact repository. \
                     Add the file to .gitignore if it shouldn't be tracked.",
                ),
            );
        }
    }

    Ok(findings)
}

/// GIT002: Check for .gitattributes file presence
async fn check_gitattributes(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    if !scanner.file_exists(".gitattributes") {
        findings.push(
            Finding::new(
                "GIT002",
                "git",
                Severity::Info,
                ".gitattributes file is missing",
            )
            .with_description(
                "A .gitattributes file helps ensure consistent line endings, \
                 enables Git LFS tracking, and defines merge strategies for specific files. \
                 Without it, contributors may experience inconsistent behavior across platforms.",
            )
            .with_remediation(
                "Create a .gitattributes file to define text/binary handling, line endings, \
                 and Git LFS patterns. Example:\n\
                 * text=auto\n\
                 *.png binary\n\
                 *.jpg binary\n\
                 *.sh text eol=lf",
            ),
        );
    }

    Ok(findings)
}

/// GIT003: Check for sensitive files that might be tracked
async fn check_sensitive_files(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    // Read .gitignore content if it exists
    let gitignore_content = scanner.read_file(".gitignore").unwrap_or_default();

    // Check each sensitive pattern
    for pattern in SENSITIVE_PATTERNS {
        let matching_files = find_sensitive_files(scanner, pattern);

        for file_path in matching_files {
            // Check if the file is ignored in .gitignore
            if !is_pattern_in_gitignore(&gitignore_content, &file_path, pattern) {
                findings.push(
                    Finding::new(
                        "GIT003",
                        "git",
                        Severity::Warning,
                        format!(
                            "Sensitive file '{}' may be tracked in repository",
                            file_path
                        ),
                    )
                    .with_location(&file_path)
                    .with_description(
                        "This file matches a sensitive pattern and may contain secrets, \
                         credentials, or private keys. Tracking such files in Git exposes \
                         them in the repository history, even if later deleted.",
                    )
                    .with_remediation(format!(
                        "Add '{}' or '{}' to .gitignore. If the file was already committed, \
                         you may need to remove it from Git history using 'git filter-branch' \
                         or 'git filter-repo', and rotate any exposed credentials.",
                        file_path, pattern
                    )),
                );
            }
        }
    }

    Ok(findings)
}

/// Find files matching a sensitive pattern
fn find_sensitive_files(scanner: &Scanner, pattern: &str) -> Vec<String> {
    let mut matching_files = Vec::new();

    // Handle different pattern types
    if let Some(ext) = pattern.strip_prefix("*.") {
        // Extension pattern like "*.key"
        for file_info in scanner.files_with_extensions(&[ext]) {
            matching_files.push(file_info.path.clone());
        }
    } else if let Some(prefix) = pattern.strip_suffix('*') {
        // Prefix pattern like "credentials*"
        for file_info in scanner.files_matching_pattern(pattern) {
            // Verify it matches the prefix
            let file_name = file_info.path.rsplit('/').next().unwrap_or(&file_info.path);
            if file_name.starts_with(prefix) {
                matching_files.push(file_info.path.clone());
            }
        }
    } else if let Some(suffix) = pattern.strip_prefix('*') {
        // Suffix pattern like "*_rsa"
        for file_info in scanner.files_matching_pattern(pattern) {
            if file_info.path.ends_with(suffix) {
                matching_files.push(file_info.path.clone());
            }
        }
    } else {
        // Exact match like ".env"
        if scanner.file_exists(pattern) {
            matching_files.push(pattern.to_string());
        }
    }

    matching_files
}

/// Check if a file path or pattern is covered by .gitignore
fn is_pattern_in_gitignore(gitignore_content: &str, file_path: &str, pattern: &str) -> bool {
    let file_name = file_path.rsplit('/').next().unwrap_or(file_path);

    for line in gitignore_content.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Check if the line ignores this file
        if line == file_path || line == file_name {
            return true;
        }

        // Check if the pattern itself is in gitignore
        if line == pattern {
            return true;
        }

        // Handle glob patterns in gitignore
        if line.contains('*')
            && (gitignore_pattern_matches(line, file_name)
                || gitignore_pattern_matches(line, file_path))
        {
            return true;
        }

        // Handle directory patterns (trailing slash)
        if let Some(dir_pattern) = line.strip_suffix('/') {
            if file_path.starts_with(&format!("{}/", dir_pattern)) {
                return true;
            }
        }
    }

    false
}

/// Check if a gitignore glob pattern matches a path
fn gitignore_pattern_matches(pattern: &str, path: &str) -> bool {
    // Simple glob matching for common patterns
    if let Some(ext) = pattern.strip_prefix('*') {
        // Extension or suffix pattern (*.key or *_rsa)
        return path.ends_with(ext);
    }

    if let Some(prefix) = pattern.strip_suffix('*') {
        // Prefix pattern
        return path.starts_with(prefix);
    }

    // Exact match
    pattern == path
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::scanner::Scanner;
    use std::fs;
    use tempfile::TempDir;

    // --- GIT001: Large binary files ---

    #[tokio::test]
    async fn test_git001_large_binary_detected() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create a large binary file (> 1MB)
        let large_content = vec![0u8; 2 * 1024 * 1024]; // 2MB
        fs::write(root.join("large_file.zip"), large_content).unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_large_binaries(&scanner).await.unwrap();

        assert!(findings.iter().any(|f| f.rule_id == "GIT001"));
        assert!(
            findings
                .iter()
                .any(|f| f.message.contains("large_file.zip"))
        );
    }

    #[tokio::test]
    async fn test_git001_small_binary_no_finding() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create a small binary file (< 1MB)
        let small_content = vec![0u8; 100 * 1024]; // 100KB
        fs::write(root.join("small_file.zip"), small_content).unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_large_binaries(&scanner).await.unwrap();

        assert!(findings.is_empty());
    }

    #[tokio::test]
    async fn test_git001_large_text_file_no_finding() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create a large text file (not binary extension)
        let large_content = "x".repeat(2 * 1024 * 1024); // 2MB text
        fs::write(root.join("large_file.txt"), large_content).unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_large_binaries(&scanner).await.unwrap();

        assert!(findings.is_empty());
    }

    #[tokio::test]
    async fn test_git001_multiple_binary_extensions() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let large_content = vec![0u8; 2 * 1024 * 1024];

        // Create large files with various binary extensions
        fs::write(root.join("file.exe"), &large_content).unwrap();
        fs::write(root.join("file.dll"), &large_content).unwrap();
        fs::write(root.join("file.jar"), &large_content).unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_large_binaries(&scanner).await.unwrap();

        assert_eq!(findings.len(), 3);
        assert!(findings.iter().all(|f| f.rule_id == "GIT001"));
    }

    // --- GIT002: .gitattributes missing ---

    #[tokio::test]
    async fn test_git002_gitattributes_missing() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(root.join("README.md"), "# Project").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_gitattributes(&scanner).await.unwrap();

        assert!(findings.iter().any(|f| f.rule_id == "GIT002"));
        assert!(findings.iter().any(|f| f.severity == Severity::Info));
    }

    #[tokio::test]
    async fn test_git002_gitattributes_present() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(root.join(".gitattributes"), "* text=auto").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_gitattributes(&scanner).await.unwrap();

        assert!(findings.is_empty());
    }

    // --- GIT003: Sensitive files tracked ---

    #[tokio::test]
    async fn test_git003_env_file_not_ignored() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(root.join(".env"), "SECRET_KEY=abc123").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_sensitive_files(&scanner).await.unwrap();

        assert!(findings.iter().any(|f| f.rule_id == "GIT003"));
        assert!(findings.iter().any(|f| f.message.contains(".env")));
    }

    #[tokio::test]
    async fn test_git003_env_file_ignored() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(root.join(".env"), "SECRET_KEY=abc123").unwrap();
        fs::write(root.join(".gitignore"), ".env\n").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_sensitive_files(&scanner).await.unwrap();

        // Should not report .env since it's in .gitignore
        assert!(!findings.iter().any(|f| f.message.contains(".env")));
    }

    #[tokio::test]
    async fn test_git003_key_file_detected() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(root.join("private.key"), "-----BEGIN PRIVATE KEY-----").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_sensitive_files(&scanner).await.unwrap();

        assert!(findings.iter().any(|f| f.rule_id == "GIT003"));
        assert!(findings.iter().any(|f| f.message.contains("private.key")));
    }

    #[tokio::test]
    async fn test_git003_pem_file_ignored_by_pattern() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(root.join("cert.pem"), "-----BEGIN CERTIFICATE-----").unwrap();
        fs::write(root.join(".gitignore"), "*.pem\n").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_sensitive_files(&scanner).await.unwrap();

        // Should not report cert.pem since *.pem is in .gitignore
        assert!(!findings.iter().any(|f| f.message.contains("cert.pem")));
    }

    #[tokio::test]
    async fn test_git003_credentials_prefix_pattern() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(root.join("credentials.json"), "{}").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_sensitive_files(&scanner).await.unwrap();

        assert!(findings.iter().any(|f| f.message.contains("credentials")));
    }

    #[tokio::test]
    async fn test_git003_rsa_suffix_pattern() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(root.join("id_rsa"), "-----BEGIN RSA PRIVATE KEY-----").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_sensitive_files(&scanner).await.unwrap();

        assert!(findings.iter().any(|f| f.message.contains("id_rsa")));
    }

    // --- Integration tests ---

    #[tokio::test]
    async fn test_git_rules_category_name() {
        let rules = GitRules;
        assert_eq!(rules.name(), "git");
    }

    #[tokio::test]
    async fn test_git_rules_run_integration() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create files that trigger all rules
        let large_content = vec![0u8; 2 * 1024 * 1024];
        fs::write(root.join("archive.zip"), large_content).unwrap();
        fs::write(root.join(".env"), "SECRET=value").unwrap();
        // No .gitattributes

        let scanner = Scanner::new(root.to_path_buf());
        let config = Config::default();
        let rules = GitRules;

        let findings = rules.run(&scanner, &config).await.unwrap();

        let rule_ids: Vec<&str> = findings.iter().map(|f| f.rule_id.as_str()).collect();
        assert!(
            rule_ids.contains(&"GIT001"),
            "Should find GIT001 (large binary)"
        );
        assert!(
            rule_ids.contains(&"GIT002"),
            "Should find GIT002 (no .gitattributes)"
        );
        assert!(
            rule_ids.contains(&"GIT003"),
            "Should find GIT003 (sensitive file)"
        );
    }

    #[tokio::test]
    async fn test_git_rules_clean_repo() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create a clean repo with proper configuration
        fs::write(root.join(".gitattributes"), "* text=auto").unwrap();
        fs::write(root.join(".gitignore"), ".env\n*.key\n*.pem\n").unwrap();
        fs::write(root.join("README.md"), "# Clean Project").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let config = Config::default();
        let rules = GitRules;

        let findings = rules.run(&scanner, &config).await.unwrap();

        assert!(findings.is_empty(), "Clean repo should have no findings");
    }

    // --- Helper function tests ---

    #[test]
    fn test_gitignore_pattern_matches_extension() {
        assert!(gitignore_pattern_matches("*.key", "private.key"));
        assert!(gitignore_pattern_matches("*.pem", "cert.pem"));
        assert!(!gitignore_pattern_matches("*.key", "private.pem"));
    }

    #[test]
    fn test_gitignore_pattern_matches_prefix() {
        assert!(gitignore_pattern_matches(
            "credentials*",
            "credentials.json"
        ));
        assert!(gitignore_pattern_matches("secrets*", "secrets.yaml"));
        assert!(!gitignore_pattern_matches("credentials*", "my_credentials"));
    }

    #[test]
    fn test_gitignore_pattern_matches_suffix() {
        assert!(gitignore_pattern_matches("*_rsa", "id_rsa"));
        assert!(gitignore_pattern_matches("*_rsa", "deploy_rsa"));
        assert!(!gitignore_pattern_matches("*_rsa", "id_rsa.pub"));
    }

    #[test]
    fn test_is_pattern_in_gitignore() {
        let gitignore = ".env\n*.key\ncredentials*\n# comment\n";

        assert!(is_pattern_in_gitignore(gitignore, ".env", ".env"));
        assert!(is_pattern_in_gitignore(gitignore, "private.key", "*.key"));
        assert!(is_pattern_in_gitignore(
            gitignore,
            "credentials.json",
            "credentials*"
        ));
        assert!(!is_pattern_in_gitignore(
            gitignore,
            "config.yml",
            "config.yml"
        ));
    }
}
