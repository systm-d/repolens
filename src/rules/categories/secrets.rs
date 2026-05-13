//! Secrets detection rules
//!
//! This module provides rules for detecting exposed secrets and credentials
//! in repository files. It checks for:
//! - Hardcoded secrets in source files (API keys, tokens, passwords)
//! - Sensitive files (private keys, certificates, credentials)
//! - Environment files (.env) that should not be committed

use rayon::prelude::*;

use crate::config::Config;
use crate::error::RepoLensError;
use crate::rules::engine::RuleCategory;
use crate::rules::patterns::SECRET_PATTERNS;
use crate::rules::results::{Finding, Severity};
use crate::scanner::Scanner;

/// Rules for detecting secrets and credentials
pub struct SecretsRules;

#[async_trait::async_trait]
impl RuleCategory for SecretsRules {
    /// Get the category name
    fn name(&self) -> &'static str {
        "secrets"
    }

    /// Run all secrets detection rules
    ///
    /// # Arguments
    ///
    /// * `scanner` - The scanner to access repository files
    /// * `config` - The configuration with enabled rules
    ///
    /// # Returns
    ///
    /// A vector of findings for detected secrets
    ///
    /// # Errors
    ///
    /// Returns an error if the scan fails
    async fn run(&self, scanner: &Scanner, config: &Config) -> Result<Vec<Finding>, RepoLensError> {
        let mut findings = Vec::new();

        // Check for hardcoded secrets in source files
        if config.is_rule_enabled("secrets/hardcoded") {
            findings.extend(check_hardcoded_secrets(scanner, config).await?);
        }

        // Check for sensitive files
        if config.is_rule_enabled("secrets/files") {
            findings.extend(check_sensitive_files(scanner, config).await?);
        }

        // Check for .env files
        if config.is_rule_enabled("secrets/env") {
            findings.extend(check_env_files(scanner, config).await?);
        }

        Ok(findings)
    }
}

/// Check for hardcoded secrets in source files
///
/// Scans files with common source code extensions for patterns that indicate
/// hardcoded secrets like API keys, tokens, and passwords.
///
/// # Arguments
///
/// * `scanner` - The scanner to access repository files
/// * `config` - The configuration with ignore patterns
///
/// # Returns
///
/// A vector of findings for detected secrets
async fn check_hardcoded_secrets(
    scanner: &Scanner,
    config: &Config,
) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    // File extensions to scan
    let extensions = [
        "js", "ts", "jsx", "tsx", "py", "rb", "php", "java", "go", "rs", "cpp", "c", "yml", "yaml",
        "json", "toml", "env", "config", "conf", "sql", "sh", "bash",
    ];

    let files: Vec<_> = scanner
        .files_with_extensions(&extensions)
        .into_iter()
        .filter(|file| !config.should_ignore_file(&file.path))
        .map(|file| file.path.clone())
        .collect();

    // Process files in parallel
    let file_findings: Vec<Vec<Finding>> = files
        .par_iter()
        .filter_map(|file_path| {
            let content = match scanner.read_file(file_path) {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!("Failed to read file {}: {}", file_path, e);
                    return None;
                }
            };

            match check_file_for_secrets(file_path, &content, config) {
                Ok(f) => Some(f),
                Err(e) => {
                    tracing::warn!("Error checking file {}: {}", file_path, e);
                    None
                }
            }
        })
        .collect();

    // Flatten results
    for file_finding in file_findings {
        findings.extend(file_finding);
    }

    Ok(findings)
}

/// Check a single file for secrets
///
/// # Arguments
///
/// * `file_path` - Path to the file
/// * `content` - File content
/// * `config` - Configuration with ignore patterns
///
/// # Returns
///
/// A vector of findings for secrets found in this file
fn check_file_for_secrets(
    file_path: &str,
    content: &str,
    config: &Config,
) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    for pattern in SECRET_PATTERNS.iter() {
        if let Some(captures) = pattern.regex.captures(content) {
            if config.should_ignore_pattern(file_path) {
                continue;
            }

            let line_num = find_line_number(content, &captures)?;

            findings.push(
                Finding::new(
                    "SEC001",
                    "secrets",
                    Severity::Critical,
                    format!("{} detected", pattern.name),
                )
                .with_location(format!("{}:{}", file_path, line_num))
                .with_description(pattern.description.to_string())
                .with_remediation(
                    "Remove the secret and use environment variables or a secrets manager instead.",
                ),
            );
        }
    }

    Ok(findings)
}

/// Find the line number where a regex match occurs
///
/// # Arguments
///
/// * `content` - File content
/// * `captures` - Regex captures from the match
///
/// # Returns
///
/// The line number (1-indexed)
fn find_line_number(content: &str, captures: &regex::Captures) -> Result<usize, RepoLensError> {
    let match_start = captures
        .get(0)
        .ok_or_else(|| {
            RepoLensError::Rule(crate::error::RuleError::ExecutionFailed {
                message: "No match found in pattern capture".to_string(),
            })
        })?
        .start();

    Ok(content[..match_start].matches('\n').count() + 1)
}

/// Check for sensitive files that should not be in version control
///
/// Detects files like private keys, certificates, and credential files
/// that pose a security risk if committed to the repository.
///
/// # Arguments
///
/// * `scanner` - The scanner to access repository files
/// * `_config` - The configuration (currently unused)
///
/// # Returns
///
/// A vector of findings for detected sensitive files
async fn check_sensitive_files(
    scanner: &Scanner,
    _config: &Config,
) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    // List of sensitive file patterns
    let sensitive_patterns = [
        ("*.pem", "Private key file"),
        ("*.key", "Private key file"),
        ("*.p12", "PKCS#12 certificate bundle"),
        ("*.pfx", "PKCS#12 certificate bundle"),
        ("*.jks", "Java keystore"),
        ("id_rsa", "SSH private key"),
        ("id_dsa", "SSH private key"),
        ("id_ecdsa", "SSH private key"),
        ("id_ed25519", "SSH private key"),
        (".htpasswd", "Apache password file"),
        ("credentials.json", "Credentials file"),
        ("service-account.json", "Service account credentials"),
        ("secrets.yml", "Secrets configuration"),
        ("secrets.yaml", "Secrets configuration"),
        ("secrets.json", "Secrets configuration"),
    ];

    for (pattern, description) in sensitive_patterns {
        for file in scanner.files_matching_pattern(pattern) {
            findings.push(
                Finding::new(
                    "SEC002",
                    "secrets",
                    Severity::Critical,
                    format!("{} found in repository", description),
                )
                .with_location(&file.path)
                .with_description(format!(
                    "The file '{}' appears to contain sensitive data and should not be committed to version control.",
                    file.path
                ))
                .with_remediation(
                    "Remove the file from the repository and add it to .gitignore. If the file was previously committed, consider rotating any contained credentials."
                )
            );
        }
    }

    Ok(findings)
}

/// Check for .env files that should not be committed
///
/// Detects environment files that may contain secrets. Example files
/// (.env.example, .env.template) are allowed.
///
/// # Arguments
///
/// * `scanner` - The scanner to access repository files
/// * `_config` - The configuration (currently unused)
///
/// # Returns
///
/// A vector of findings for detected .env files
async fn check_env_files(
    scanner: &Scanner,
    _config: &Config,
) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    // Check for .env files (but allow .env.example)
    let env_patterns = [
        ".env",
        ".env.local",
        ".env.production",
        ".env.development",
        ".env.test",
    ];

    for pattern in env_patterns {
        for file in scanner.files_matching_pattern(pattern) {
            // Allow example/template files
            if file.path.contains(".example")
                || file.path.contains(".template")
                || file.path.contains(".sample")
            {
                continue;
            }

            findings.push(
                Finding::new(
                    "SEC003",
                    "secrets",
                    Severity::Critical,
                    "Environment file found in repository",
                )
                .with_location(&file.path)
                .with_description(
                    "Environment files often contain sensitive configuration and secrets that should not be committed."
                )
                .with_remediation(
                    "Add the file to .gitignore and create a .env.example file as a template."
                )
            );
        }
    }

    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::Scanner;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_check_hardcoded_secrets_detects_api_key() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let config_file = root.join("config.js");

        fs::write(&config_file, "const apiKey = 'sk_test_1234567890abcdef';").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let config = Config::default();

        let findings = check_hardcoded_secrets(&scanner, &config).await.unwrap();

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id == "SEC001"));
        assert!(findings.iter().any(|f| f.message.contains("detected")));
    }

    #[tokio::test]
    async fn test_check_hardcoded_secrets_ignores_ignored_files() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let config_file = root.join("config.js");

        fs::write(&config_file, "const apiKey = 'sk_test_1234567890abcdef';").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let mut config = Config::default();
        config.secrets.ignore_files.push("config.js".to_string());

        let findings = check_hardcoded_secrets(&scanner, &config).await.unwrap();

        assert!(findings.is_empty());
    }

    #[tokio::test]
    async fn test_check_sensitive_files_detects_pem() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let key_file = root.join("private.pem");

        fs::write(&key_file, "-----BEGIN PRIVATE KEY-----").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let config = Config::default();

        let findings = check_sensitive_files(&scanner, &config).await.unwrap();

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id == "SEC002"));
        assert!(
            findings
                .iter()
                .any(|f| f.message.contains("Private key file"))
        );
    }

    #[tokio::test]
    async fn test_check_env_files_detects_env() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let env_file = root.join(".env");

        fs::write(&env_file, "API_KEY=secret123").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let config = Config::default();

        let findings = check_env_files(&scanner, &config).await.unwrap();

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id == "SEC003"));
    }

    #[tokio::test]
    async fn test_check_env_files_allows_example() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let env_example = root.join(".env.example");

        fs::write(&env_example, "API_KEY=your_key_here").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let config = Config::default();

        let findings = check_env_files(&scanner, &config).await.unwrap();

        assert!(findings.is_empty());
    }

    #[tokio::test]
    async fn test_check_env_files_allows_template() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(root.join(".env.template"), "API_KEY=your_key_here").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let config = Config::default();

        let findings = check_env_files(&scanner, &config).await.unwrap();

        assert!(findings.is_empty());
    }

    #[tokio::test]
    async fn test_check_env_files_allows_sample() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(root.join(".env.sample"), "API_KEY=your_key_here").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let config = Config::default();

        let findings = check_env_files(&scanner, &config).await.unwrap();

        assert!(findings.is_empty());
    }

    #[tokio::test]
    async fn test_check_sensitive_files_detects_ssh_key() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(root.join("id_rsa"), "-----BEGIN RSA PRIVATE KEY-----").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let config = Config::default();

        let findings = check_sensitive_files(&scanner, &config).await.unwrap();

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id == "SEC002"));
    }

    #[tokio::test]
    async fn test_check_sensitive_files_detects_credentials_json() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(root.join("credentials.json"), "{}").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let config = Config::default();

        let findings = check_sensitive_files(&scanner, &config).await.unwrap();

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.message.contains("Credentials")));
    }

    #[tokio::test]
    async fn test_secrets_rules_name() {
        let rules = SecretsRules;
        assert_eq!(rules.name(), "secrets");
    }

    #[tokio::test]
    async fn test_check_hardcoded_secrets_ignores_pattern() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(
            root.join("test_config.js"),
            "const apiKey = 'sk_test_1234567890abcdef';",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let mut config = Config::default();
        config
            .secrets
            .ignore_patterns
            .push("test_config.js".to_string());

        let findings = check_hardcoded_secrets(&scanner, &config).await.unwrap();

        // File matches ignore pattern, so findings should be suppressed
        assert!(findings.is_empty());
    }

    #[tokio::test]
    async fn test_check_hardcoded_secrets_no_secrets() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        fs::write(root.join("clean.js"), "const x = 42;").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let config = Config::default();

        let findings = check_hardcoded_secrets(&scanner, &config).await.unwrap();

        // No secrets in clean file
        assert!(findings.is_empty());
    }

    #[test]
    fn test_find_line_number() {
        let content = "line1\nline2\nline3\n";
        let pattern = regex::Regex::new("line2").unwrap();
        let captures = pattern.captures(content).unwrap();
        let line_num = find_line_number(content, &captures).unwrap();
        assert_eq!(line_num, 2);
    }

    #[test]
    fn test_find_line_number_first_line() {
        let content = "line1\nline2\nline3\n";
        let pattern = regex::Regex::new("line1").unwrap();
        let captures = pattern.captures(content).unwrap();
        let line_num = find_line_number(content, &captures).unwrap();
        assert_eq!(line_num, 1);
    }
}
