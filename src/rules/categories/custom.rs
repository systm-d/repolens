//! Custom rules category
//!
//! Allows users to define custom audit rules via regex patterns or shell commands in configuration.

use crate::config::Config;
use crate::error::RepoLensError;
use crate::rules::engine::RuleCategory;
use crate::rules::{Finding, Severity};
use crate::scanner::Scanner;
use regex::Regex;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use tracing::debug;

/// Flag to ensure we only print the shell command warning once per run
static SHELL_WARNING_PRINTED: AtomicBool = AtomicBool::new(false);

/// Print a security warning about shell command execution (only once per run)
fn warn_about_shell_commands() {
    if !SHELL_WARNING_PRINTED.swap(true, Ordering::SeqCst) {
        eprintln!(
            "\n\u{26a0}\u{fe0f}  Warning: Custom rules with shell commands detected. Only use commands from trusted sources.\n"
        );
    }
}

/// Custom rules implementation
pub struct CustomRules;

/// Simple glob matching (supports * and **)
fn glob_match(pattern: &str, text: &str) -> bool {
    if pattern.contains("**") {
        return glob_match_double_star(pattern, text);
    }

    if pattern.contains('*') {
        return glob_match_single_star(pattern, text);
    }

    text == pattern
}

/// Match pattern with double star (**)
fn glob_match_double_star(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split("**").collect();

    if parts.len() == 3 && parts[0].is_empty() && parts[2].is_empty() {
        let middle = parts[1].trim_matches('/');
        return text.contains(&format!("/{}", middle)) || text.starts_with(middle);
    }

    if parts.len() != 2 {
        return false;
    }

    let prefix = parts[0].trim_end_matches('/');
    let suffix_raw = parts[1];
    let suffix = suffix_raw.trim_start_matches('/');

    if !prefix.is_empty() && !text.starts_with(prefix) {
        return false;
    }

    if suffix.is_empty() {
        return true;
    }

    if suffix.starts_with('*') {
        let suffix_pattern = suffix.trim_start_matches('*');
        return text.ends_with(suffix_pattern);
    }

    if prefix.is_empty() {
        if suffix_raw.starts_with('/') {
            let pattern_to_find = format!("/{}", suffix);
            if text.contains(&pattern_to_find) {
                return true;
            }
            if text.starts_with(suffix) {
                return true;
            }
            return false;
        }
        return text.contains(suffix);
    }

    if let Some(after_prefix) = text.strip_prefix(prefix) {
        return after_prefix.contains(suffix) || after_prefix.ends_with(suffix);
    }

    text.ends_with(suffix) || text.contains(suffix)
}

/// Match pattern with single star (*)
fn glob_match_single_star(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();
    let mut pos = 0;

    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }

        if let Some(found_pos) = text[pos..].find(part) {
            if i == 0 && found_pos != 0 {
                return false;
            }
            pos += found_pos + part.len();
        } else {
            return false;
        }
    }

    if let Some(last_part) = parts.last() {
        if !last_part.is_empty() {
            return text.ends_with(last_part);
        }
    }

    true
}

/// Execute a shell command and return its exit code
fn execute_shell_command(command: &str) -> Result<i32, RepoLensError> {
    // Use sh -c to execute the command (works on Unix-like systems)
    #[cfg(unix)]
    {
        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .output()
            .map_err(|e| {
                RepoLensError::Scan(crate::error::ScanError::FileRead {
                    path: format!("shell command: {}", command),
                    source: std::io::Error::other(format!("Failed to execute command: {}", e)),
                })
            })?;

        Ok(output.status.code().unwrap_or(1))
    }

    // On Windows, use cmd /C
    #[cfg(windows)]
    {
        let output = Command::new("cmd")
            .arg("/C")
            .arg(command)
            .output()
            .map_err(|e| {
                RepoLensError::Scan(crate::error::ScanError::FileRead {
                    path: format!("shell command: {}", command),
                    source: std::io::Error::other(format!("Failed to execute command: {}", e)),
                })
            })?;

        Ok(output.status.code().unwrap_or(1))
    }
}

#[async_trait::async_trait]
impl RuleCategory for CustomRules {
    fn name(&self) -> &'static str {
        "custom"
    }

    async fn run(&self, scanner: &Scanner, config: &Config) -> Result<Vec<Finding>, RepoLensError> {
        let mut findings = Vec::new();

        // Skip if no custom rules defined
        if config.custom_rules.rules.is_empty() {
            debug!("No custom rules defined");
            return Ok(findings);
        }

        // Get all files from the scanner
        let all_files = scanner.all_files();

        for (rule_id, rule) in &config.custom_rules.rules {
            // Determine severity
            let severity = match rule.severity.to_lowercase().as_str() {
                "critical" => Severity::Critical,
                "warning" => Severity::Warning,
                "info" => Severity::Info,
                _ => Severity::Warning,
            };

            // Handle shell command rules
            if let Some(ref command) = rule.command {
                warn_about_shell_commands();
                debug!(rule_id = %rule_id, command = %command, "Processing custom shell command rule");

                match execute_shell_command(command) {
                    Ok(exit_code) => {
                        let command_succeeded = exit_code == 0;
                        let should_report = if rule.invert {
                            !command_succeeded // Report if command failed
                        } else {
                            command_succeeded // Report if command succeeded
                        };

                        if should_report {
                            let message = rule.message.clone().unwrap_or_else(|| {
                                if rule.invert {
                                    format!(
                                        "Command '{}' failed (exit code: {})",
                                        command, exit_code
                                    )
                                } else {
                                    format!("Command '{}' succeeded", command)
                                }
                            });

                            let description = rule.description.clone().unwrap_or_else(|| {
                                format!(
                                    "Custom rule '{}' triggered by command '{}'",
                                    rule_id, command
                                )
                            });

                            findings.push(Finding {
                                rule_id: format!("custom/{}", rule_id),
                                category: "custom".to_string(),
                                severity,
                                message,
                                location: None,
                                description: Some(description),
                                remediation: rule.remediation.clone(),
                            });
                        }
                    }
                    Err(e) => {
                        debug!(
                            rule_id = %rule_id,
                            command = %command,
                            error = %e,
                            "Failed to execute shell command"
                        );
                        // Report error if invert=false (command should succeed but failed)
                        if !rule.invert {
                            findings.push(Finding {
                                rule_id: format!("custom/{}", rule_id),
                                category: "custom".to_string(),
                                severity,
                                message: rule.message.clone().unwrap_or_else(|| {
                                    format!("Failed to execute command '{}': {}", command, e)
                                }),
                                location: None,
                                description: rule.description.clone(),
                                remediation: rule.remediation.clone(),
                            });
                        }
                    }
                }
                continue;
            }

            // Handle regex pattern rules
            let pattern = match &rule.pattern {
                Some(p) => p,
                None => {
                    debug!(
                        rule_id = %rule_id,
                        "Custom rule must have either 'pattern' or 'command'"
                    );
                    continue;
                }
            };

            debug!(rule_id = %rule_id, pattern = %pattern, "Processing custom regex pattern rule");

            // Compile the regex pattern
            let regex = match Regex::new(pattern) {
                Ok(r) => r,
                Err(e) => {
                    debug!(
                        rule_id = %rule_id,
                        error = %e,
                        "Invalid regex pattern in custom rule"
                    );
                    continue;
                }
            };

            // Filter files based on glob patterns
            let files_to_check: Vec<_> = if rule.files.is_empty() {
                all_files.iter().collect()
            } else {
                all_files
                    .iter()
                    .filter(|file| {
                        rule.files
                            .iter()
                            .any(|pattern| glob_match(pattern, &file.path))
                    })
                    .collect()
            };

            // Check each file
            for file_info in files_to_check {
                let file_path = &file_info.path;

                // Read file content
                let content = match scanner.read_file(file_path) {
                    Ok(c) => c,
                    Err(_) => continue, // Skip files that can't be read
                };

                let pattern_found = regex.is_match(&content);

                // Handle inverted matching
                let should_report = if rule.invert {
                    !pattern_found // Report if pattern NOT found
                } else {
                    pattern_found // Report if pattern found
                };

                if should_report {
                    // Find line numbers for matches (only for non-inverted)
                    let locations: Vec<(usize, String)> = if !rule.invert {
                        content
                            .lines()
                            .enumerate()
                            .filter(|(_, line)| regex.is_match(line))
                            .map(|(i, line)| (i + 1, line.to_string()))
                            .take(5) // Limit to first 5 matches
                            .collect()
                    } else {
                        vec![]
                    };

                    let message = rule.message.clone().unwrap_or_else(|| {
                        if rule.invert {
                            format!("Required pattern '{}' not found", pattern)
                        } else {
                            format!("Pattern '{}' matched", pattern)
                        }
                    });

                    let description = rule.description.clone().unwrap_or_else(|| {
                        if locations.is_empty() {
                            format!("Custom rule '{}' triggered in {}", rule_id, file_path)
                        } else {
                            let lines: Vec<String> = locations
                                .iter()
                                .map(|(line_num, _)| format!("line {}", line_num))
                                .collect();
                            format!(
                                "Custom rule '{}' triggered in {} at {}",
                                rule_id,
                                file_path,
                                lines.join(", ")
                            )
                        }
                    });

                    let location = if let Some((line_num, _)) = locations.first() {
                        Some(format!("{}:{}", file_path, line_num))
                    } else {
                        Some(file_path.to_string())
                    };

                    findings.push(Finding {
                        rule_id: format!("custom/{}", rule_id),
                        category: "custom".to_string(),
                        severity,
                        message,
                        location,
                        description: Some(description),
                        remediation: rule.remediation.clone(),
                    });
                }
            }
        }

        Ok(findings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CustomRule, CustomRulesConfig};
    use std::collections::HashMap;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_config_with_rule(rule_id: &str, rule: CustomRule) -> Config {
        let mut rules = HashMap::new();
        rules.insert(rule_id.to_string(), rule);
        Config {
            custom_rules: CustomRulesConfig { rules },
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn test_custom_rule_pattern_match() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.rs");
        fs::write(&test_file, "// TODO: fix this later\nfn main() {}").unwrap();

        let rule = CustomRule {
            pattern: Some("TODO".to_string()),
            command: None,
            severity: "warning".to_string(),
            files: vec!["**/*.rs".to_string()],
            message: Some("TODO comment found".to_string()),
            description: None,
            remediation: Some("Address or remove the TODO".to_string()),
            invert: false,
        };

        let config = create_test_config_with_rule("no-todo", rule);
        let scanner = Scanner::new(temp_dir.path().to_path_buf());
        let custom_rules = CustomRules;

        let findings = custom_rules.run(&scanner, &config).await.unwrap();

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "custom/no-todo");
        assert_eq!(findings[0].severity, Severity::Warning);
        assert!(findings[0].message.contains("TODO"));
    }

    #[tokio::test]
    async fn test_custom_rule_no_match() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("test.rs");
        fs::write(&test_file, "fn main() { println!(\"Hello\"); }").unwrap();

        let rule = CustomRule {
            pattern: Some("TODO".to_string()),
            command: None,
            severity: "warning".to_string(),
            files: vec!["**/*.rs".to_string()],
            message: None,
            description: None,
            remediation: None,
            invert: false,
        };

        let config = create_test_config_with_rule("no-todo", rule);
        let scanner = Scanner::new(temp_dir.path().to_path_buf());
        let custom_rules = CustomRules;

        let findings = custom_rules.run(&scanner, &config).await.unwrap();

        assert!(findings.is_empty());
    }

    #[tokio::test]
    async fn test_custom_rule_inverted_match() {
        let temp_dir = TempDir::new().unwrap();
        let test_file = temp_dir.path().join("lib.rs");
        fs::write(&test_file, "fn helper() {}").unwrap();

        let rule = CustomRule {
            pattern: Some(r"^//!".to_string()), // Module doc comment
            command: None,
            severity: "info".to_string(),
            files: vec!["**/lib.rs".to_string()],
            message: Some("Missing module documentation".to_string()),
            description: None,
            remediation: Some("Add module-level documentation".to_string()),
            invert: true, // Fail when NOT found
        };

        let config = create_test_config_with_rule("require-doc", rule);
        let scanner = Scanner::new(temp_dir.path().to_path_buf());
        let custom_rules = CustomRules;

        let findings = custom_rules.run(&scanner, &config).await.unwrap();

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "custom/require-doc");
        assert!(findings[0].message.contains("Missing module documentation"));
    }

    #[tokio::test]
    async fn test_custom_rule_file_filter() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("test.rs"), "TODO: fix").unwrap();
        fs::write(temp_dir.path().join("test.js"), "// TODO: fix").unwrap();

        let rule = CustomRule {
            pattern: Some("TODO".to_string()),
            command: None,
            severity: "warning".to_string(),
            files: vec!["**/*.rs".to_string()], // Only Rust files
            message: None,
            description: None,
            remediation: None,
            invert: false,
        };

        let config = create_test_config_with_rule("no-todo", rule);
        let scanner = Scanner::new(temp_dir.path().to_path_buf());
        let custom_rules = CustomRules;

        let findings = custom_rules.run(&scanner, &config).await.unwrap();

        // Should only find in .rs file, not .js
        assert_eq!(findings.len(), 1);
        assert!(
            findings[0].location.as_ref().unwrap().ends_with(".rs")
                || findings[0].location.as_ref().unwrap().contains(".rs:")
        );
    }

    #[tokio::test]
    async fn test_custom_rule_severity_levels() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("test.rs"), "FIXME: urgent").unwrap();

        let rule = CustomRule {
            pattern: Some("FIXME".to_string()),
            command: None,
            severity: "critical".to_string(),
            files: vec![],
            message: None,
            description: None,
            remediation: None,
            invert: false,
        };

        let config = create_test_config_with_rule("no-fixme", rule);
        let scanner = Scanner::new(temp_dir.path().to_path_buf());
        let custom_rules = CustomRules;

        let findings = custom_rules.run(&scanner, &config).await.unwrap();

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Critical);
    }

    #[tokio::test]
    async fn test_no_custom_rules_returns_empty() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("test.rs"), "fn main() {}").unwrap();

        let config = Config::default(); // No custom rules
        let scanner = Scanner::new(temp_dir.path().to_path_buf());
        let custom_rules = CustomRules;

        let findings = custom_rules.run(&scanner, &config).await.unwrap();

        assert!(findings.is_empty());
    }

    #[tokio::test]
    async fn test_invalid_regex_is_skipped() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("test.rs"), "test content").unwrap();

        let rule = CustomRule {
            pattern: Some("[invalid regex".to_string()), // Invalid regex
            command: None,
            severity: "warning".to_string(),
            files: vec![],
            message: None,
            description: None,
            remediation: None,
            invert: false,
        };

        let config = create_test_config_with_rule("bad-rule", rule);
        let scanner = Scanner::new(temp_dir.path().to_path_buf());
        let custom_rules = CustomRules;

        // Should not panic, just skip the invalid rule
        let findings = custom_rules.run(&scanner, &config).await.unwrap();
        assert!(findings.is_empty());
    }

    #[tokio::test]
    async fn test_custom_rule_shell_command_success() {
        let temp_dir = TempDir::new().unwrap();
        let scanner = Scanner::new(temp_dir.path().to_path_buf());

        // Command that succeeds (exit code 0)
        let rule = CustomRule {
            pattern: None,
            command: Some("echo test".to_string()),
            severity: "warning".to_string(),
            files: vec![],
            message: Some("Command succeeded".to_string()),
            description: None,
            remediation: None,
            invert: false, // Report on success
        };

        let config = create_test_config_with_rule("test-command", rule);
        let custom_rules = CustomRules;

        let findings = custom_rules.run(&scanner, &config).await.unwrap();

        // Should report because command succeeded and invert=false
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "custom/test-command");
    }

    #[tokio::test]
    async fn test_custom_rule_shell_command_failure() {
        let temp_dir = TempDir::new().unwrap();
        let scanner = Scanner::new(temp_dir.path().to_path_buf());

        // Command that fails (exit code != 0)
        let rule = CustomRule {
            pattern: None,
            command: Some("false".to_string()), // Always returns non-zero
            severity: "warning".to_string(),
            files: vec![],
            message: Some("Command failed".to_string()),
            description: None,
            remediation: None,
            invert: true, // Report on failure
        };

        let config = create_test_config_with_rule("test-command-fail", rule);
        let custom_rules = CustomRules;

        let findings = custom_rules.run(&scanner, &config).await.unwrap();

        // Should report because command failed and invert=true
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Command failed"));
    }

    #[tokio::test]
    async fn test_custom_rule_shell_command_invalid() {
        let temp_dir = TempDir::new().unwrap();
        let scanner = Scanner::new(temp_dir.path().to_path_buf());

        // Invalid command that will fail to execute or return non-zero
        let rule = CustomRule {
            pattern: None,
            command: Some("nonexistent-command-xyz123".to_string()),
            severity: "warning".to_string(),
            files: vec![],
            message: None,
            description: None,
            remediation: None,
            invert: true, // Report on failure (command will fail)
        };

        let config = create_test_config_with_rule("test-invalid-command", rule);
        let custom_rules = CustomRules;

        let findings = custom_rules.run(&scanner, &config).await.unwrap();

        // Should report because command failed (exit code != 0) and invert=true
        // Note: Some shells may return 127 for command not found, others may return 1
        assert!(!findings.is_empty());
    }

    #[tokio::test]
    async fn test_custom_rule_no_pattern_or_command() {
        let temp_dir = TempDir::new().unwrap();
        let scanner = Scanner::new(temp_dir.path().to_path_buf());

        // Rule with neither pattern nor command
        let rule = CustomRule {
            pattern: None,
            command: None,
            severity: "warning".to_string(),
            files: vec![],
            message: None,
            description: None,
            remediation: None,
            invert: false,
        };

        let config = create_test_config_with_rule("test-empty", rule);
        let custom_rules = CustomRules;

        let findings = custom_rules.run(&scanner, &config).await.unwrap();

        // Should skip rule with no pattern or command
        assert!(findings.is_empty());
    }

    // ===== Glob Matching Tests =====

    #[test]
    fn test_glob_match_exact() {
        assert!(glob_match("test.rs", "test.rs"));
        assert!(!glob_match("test.rs", "other.rs"));
    }

    #[test]
    fn test_glob_match_single_star() {
        assert!(glob_match("*.rs", "test.rs"));
        assert!(glob_match("*.rs", "main.rs"));
        assert!(!glob_match("*.rs", "test.js"));
        assert!(glob_match("test.*", "test.rs"));
        assert!(glob_match("test.*", "test.js"));
        assert!(!glob_match("test.*", "other.rs"));
    }

    #[test]
    fn test_glob_match_single_star_middle() {
        assert!(glob_match("test_*.rs", "test_unit.rs"));
        assert!(glob_match("test_*.rs", "test_integration.rs"));
        assert!(!glob_match("test_*.rs", "unit_test.rs"));
    }

    #[test]
    fn test_glob_match_double_star() {
        assert!(glob_match("**/*.rs", "src/main.rs"));
        assert!(glob_match("**/*.rs", "src/lib/mod.rs"));
        assert!(glob_match("**/*.rs", "test.rs"));
        assert!(!glob_match("**/*.rs", "test.js"));
    }

    #[test]
    fn test_glob_match_double_star_prefix() {
        assert!(glob_match("src/**/*.rs", "src/main.rs"));
        assert!(glob_match("src/**/*.rs", "src/lib/mod.rs"));
        assert!(!glob_match("src/**/*.rs", "tests/test.rs"));
    }

    #[test]
    fn test_glob_match_double_star_suffix() {
        assert!(glob_match("**/test/**", "src/test/unit.rs"));
        assert!(glob_match("**/test/**", "test/integration.rs"));
    }

    #[test]
    fn test_glob_match_double_star_middle_directory() {
        assert!(glob_match("**/lib.rs", "src/lib.rs"));
        assert!(glob_match("**/lib.rs", "crate/src/lib.rs"));
        assert!(glob_match("**/lib.rs", "lib.rs"));
    }

    #[test]
    fn test_glob_match_complex_patterns() {
        // Note: The glob_match function has limited support for complex patterns
        // Patterns with both ** and * in different parts may not work as expected
        // These tests reflect the actual behavior
        assert!(glob_match("src/**/*.rs", "src/tests/test_unit.rs"));
        assert!(glob_match("**/*.go", "pkg/handlers/api_test.go"));
    }

    #[test]
    fn test_glob_match_double_star_only() {
        assert!(glob_match("**", "any/path/file.rs"));
        assert!(glob_match("**", "file.rs"));
    }

    // ===== CustomRules Edge Cases =====

    #[tokio::test]
    async fn test_custom_rule_info_severity() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("test.rs"), "NOTE: something").unwrap();

        let rule = CustomRule {
            pattern: Some("NOTE".to_string()),
            command: None,
            severity: "info".to_string(),
            files: vec![],
            message: None,
            description: None,
            remediation: None,
            invert: false,
        };

        let config = create_test_config_with_rule("notes", rule);
        let scanner = Scanner::new(temp_dir.path().to_path_buf());
        let custom_rules = CustomRules;

        let findings = custom_rules.run(&scanner, &config).await.unwrap();

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Info);
    }

    #[tokio::test]
    async fn test_custom_rule_unknown_severity_defaults_to_warning() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(temp_dir.path().join("test.rs"), "FIXME").unwrap();

        let rule = CustomRule {
            pattern: Some("FIXME".to_string()),
            command: None,
            severity: "unknown".to_string(), // Unknown severity
            files: vec![],
            message: None,
            description: None,
            remediation: None,
            invert: false,
        };

        let config = create_test_config_with_rule("fixme", rule);
        let scanner = Scanner::new(temp_dir.path().to_path_buf());
        let custom_rules = CustomRules;

        let findings = custom_rules.run(&scanner, &config).await.unwrap();

        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Warning); // Defaults to warning
    }

    #[tokio::test]
    async fn test_custom_rule_multiple_matches_in_file() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join("test.rs"),
            "TODO: first\nfn main() {}\n// TODO: second\n// TODO: third",
        )
        .unwrap();

        let rule = CustomRule {
            pattern: Some("TODO".to_string()),
            command: None,
            severity: "warning".to_string(),
            files: vec![],
            message: None,
            description: None,
            remediation: None,
            invert: false,
        };

        let config = create_test_config_with_rule("no-todo", rule);
        let scanner = Scanner::new(temp_dir.path().to_path_buf());
        let custom_rules = CustomRules;

        let findings = custom_rules.run(&scanner, &config).await.unwrap();

        // Should find one finding per file, not per match
        assert_eq!(findings.len(), 1);
        // Description should mention line numbers
        assert!(findings[0].description.as_ref().unwrap().contains("line"));
    }

    #[test]
    fn test_custom_rules_name() {
        let custom_rules = CustomRules;
        assert_eq!(custom_rules.name(), "custom");
    }
}
