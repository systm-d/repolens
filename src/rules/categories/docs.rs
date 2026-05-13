//! Documentation rules
//!
//! This module provides rules for checking repository documentation, including:
//! - README files and their quality
//! - LICENSE files
//! - CONTRIBUTING guidelines
//! - CODE_OF_CONDUCT files
//! - SECURITY policy files
//! - CHANGELOG presence and format

use crate::error::RepoLensError;

use crate::config::Config;
use crate::rules::engine::RuleCategory;
use crate::rules::results::{Finding, Severity};
use crate::scanner::Scanner;

/// Rules for checking repository documentation
pub struct DocsRules;

#[async_trait::async_trait]
impl RuleCategory for DocsRules {
    /// Get the category name
    fn name(&self) -> &'static str {
        "docs"
    }

    /// Run all documentation-related rules
    ///
    /// # Arguments
    ///
    /// * `scanner` - The scanner to access repository files
    /// * `config` - The configuration with enabled rules
    ///
    /// # Returns
    ///
    /// A vector of findings for documentation issues
    async fn run(&self, scanner: &Scanner, config: &Config) -> Result<Vec<Finding>, RepoLensError> {
        let mut findings = Vec::new();

        // Check README
        if config.is_rule_enabled("docs/readme") {
            findings.extend(check_readme(scanner).await?);
        }

        // Check LICENSE
        if config.is_rule_enabled("docs/license") {
            findings.extend(check_license(scanner, config).await?);
        }

        // Check CONTRIBUTING
        if config.is_rule_enabled("docs/contributing") {
            findings.extend(check_contributing(scanner).await?);
        }

        // Check CODE_OF_CONDUCT
        if config.is_rule_enabled("docs/code-of-conduct") {
            findings.extend(check_code_of_conduct(scanner).await?);
        }

        // Check SECURITY
        if config.is_rule_enabled("docs/security") {
            findings.extend(check_security(scanner).await?);
        }

        // Check CHANGELOG
        if config.is_rule_enabled("docs/changelog") {
            findings.extend(check_changelog(scanner).await?);
        }

        // Check CHANGELOG format
        if config.is_rule_enabled("docs/changelog-format") {
            findings.extend(check_changelog_format(scanner).await?);
        }

        // Check CHANGELOG unreleased section
        if config.is_rule_enabled("docs/changelog-unreleased") {
            findings.extend(check_changelog_unreleased(scanner).await?);
        }

        Ok(findings)
    }
}

/// Check for README file and assess its quality
///
/// Verifies README existence and checks for recommended sections like
/// installation, usage, and license information.
///
/// # Arguments
///
/// * `scanner` - The scanner to access repository files
///
/// # Returns
///
/// A vector of findings for README issues
async fn check_readme(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    let readme_files = ["README.md", "README", "README.txt", "README.rst"];
    let has_readme = readme_files.iter().any(|f| scanner.file_exists(f));

    if !has_readme {
        findings.push(
            Finding::new(
                "DOC001",
                "docs",
                Severity::Warning,
                "README file is missing",
            )
            .with_description(
                "A README file is essential for explaining what the project does and how to use it.",
            )
            .with_remediation(
                "Create a README.md file with project description, installation instructions, and usage examples.",
            ),
        );
        return Ok(findings);
    }

    // Check README quality
    if let Ok(content) = scanner.read_file("README.md") {
        let line_count = content.lines().count();

        if line_count < 10 {
            findings.push(
                Finding::new(
                    "DOC002",
                    "docs",
                    Severity::Warning,
                    format!("README is too short ({} lines)", line_count),
                )
                .with_description(
                    "A comprehensive README should include sections for description, installation, usage, and contribution guidelines.",
                ),
            );
        }

        // Check for recommended sections
        let sections = [
            ("installation", "Installation instructions"),
            ("usage", "Usage examples"),
            ("license", "License information"),
        ];

        for (keyword, description) in sections {
            if !content.to_lowercase().contains(keyword) {
                findings.push(Finding::new(
                    "DOC003",
                    "docs",
                    Severity::Info,
                    format!("README missing section: {}", description),
                ));
            }
        }
    }

    Ok(findings)
}

/// Check for LICENSE file
///
/// Verifies that a LICENSE file exists. For enterprise preset, LICENSE is optional.
///
/// # Arguments
///
/// * `scanner` - The scanner to access repository files
/// * `config` - The configuration (used to check preset)
///
/// # Returns
///
/// A vector of findings for LICENSE issues
async fn check_license(scanner: &Scanner, config: &Config) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    let license_files = [
        "LICENSE",
        "LICENSE.md",
        "LICENSE.txt",
        "COPYING",
        "LICENSE-MIT",
        "LICENSE-APACHE",
        "LICENCE",
    ];
    let has_license = license_files.iter().any(|f| scanner.file_exists(f));

    // For enterprise preset, LICENSE is optional
    if config.preset == "enterprise" && !has_license {
        return Ok(findings);
    }

    if !has_license {
        findings.push(
            Finding::new(
                "DOC004",
                "docs",
                Severity::Critical,
                "LICENSE file is missing",
            )
            .with_description(
                "A LICENSE file is required for open source projects to define how others can use your code.",
            )
            .with_remediation(
                "Add a LICENSE file with an appropriate open source license (MIT, Apache-2.0, GPL-3.0, etc.).",
            ),
        );
    }

    Ok(findings)
}

/// Check for CONTRIBUTING file
///
/// Verifies that a CONTRIBUTING file exists to guide contributors.
///
/// # Arguments
///
/// * `scanner` - The scanner to access repository files
///
/// # Returns
///
/// A vector of findings for CONTRIBUTING issues
async fn check_contributing(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    let contributing_files = ["CONTRIBUTING.md", "CONTRIBUTING", ".github/CONTRIBUTING.md"];
    let has_contributing = contributing_files.iter().any(|f| scanner.file_exists(f));

    if !has_contributing {
        findings.push(
            Finding::new(
                "DOC005",
                "docs",
                Severity::Warning,
                "CONTRIBUTING file is missing",
            )
            .with_description(
                "A CONTRIBUTING file helps potential contributors understand how to participate in your project.",
            )
            .with_remediation(
                "Create a CONTRIBUTING.md file with contribution guidelines, code style, and pull request process.",
            ),
        );
    }

    Ok(findings)
}

/// Check for CODE_OF_CONDUCT file
///
/// Verifies that a CODE_OF_CONDUCT file exists to establish community standards.
///
/// # Arguments
///
/// * `scanner` - The scanner to access repository files
///
/// # Returns
///
/// A vector of findings for CODE_OF_CONDUCT issues
async fn check_code_of_conduct(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    let coc_files = [
        "CODE_OF_CONDUCT.md",
        "CODE_OF_CONDUCT",
        ".github/CODE_OF_CONDUCT.md",
    ];
    let has_coc = coc_files.iter().any(|f| scanner.file_exists(f));

    if !has_coc {
        findings.push(
            Finding::new(
                "DOC006",
                "docs",
                Severity::Warning,
                "CODE_OF_CONDUCT file is missing",
            )
            .with_description(
                "A Code of Conduct establishes expectations for behavior and helps create a welcoming community.",
            )
            .with_remediation(
                "Add a CODE_OF_CONDUCT.md file. Consider using the Contributor Covenant as a starting point.",
            ),
        );
    }

    Ok(findings)
}

/// Check for SECURITY policy file
///
/// Verifies that a SECURITY.md file exists for reporting vulnerabilities.
///
/// # Arguments
///
/// * `scanner` - The scanner to access repository files
///
/// # Returns
///
/// A vector of findings for SECURITY policy issues
async fn check_security(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    let security_files = ["SECURITY.md", ".github/SECURITY.md"];
    let has_security = security_files.iter().any(|f| scanner.file_exists(f));

    if !has_security {
        findings.push(
            Finding::new(
                "DOC007",
                "docs",
                Severity::Warning,
                "SECURITY policy file is missing",
            )
            .with_description(
                "A SECURITY.md file tells users how to report security vulnerabilities responsibly.",
            )
            .with_remediation(
                "Create a SECURITY.md file with instructions for reporting security issues.",
            ),
        );
    }

    Ok(findings)
}

/// Find the changelog file path if it exists
///
/// Checks for common changelog file names.
///
/// # Arguments
///
/// * `scanner` - The scanner to access repository files
///
/// # Returns
///
/// The path to the changelog file, or None if not found
fn find_changelog(scanner: &Scanner) -> Option<&'static str> {
    let changelog_files = [
        "CHANGELOG.md",
        "CHANGELOG",
        "CHANGELOG.txt",
        "HISTORY.md",
        "CHANGES.md",
    ];
    changelog_files.into_iter().find(|f| scanner.file_exists(f))
}

/// Check for CHANGELOG file
///
/// Verifies that a CHANGELOG file exists.
///
/// # Arguments
///
/// * `scanner` - The scanner to access repository files
///
/// # Returns
///
/// A vector of findings for missing CHANGELOG
async fn check_changelog(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    if find_changelog(scanner).is_none() {
        findings.push(
            Finding::new(
                "DOC008",
                "docs",
                Severity::Warning,
                "CHANGELOG file is missing",
            )
            .with_description(
                "A CHANGELOG file helps users and contributors track notable changes between releases.",
            )
            .with_remediation(
                "Create a CHANGELOG.md file. Consider following the Keep a Changelog format (https://keepachangelog.com).",
            ),
        );
    }

    Ok(findings)
}

/// Check if CHANGELOG follows Keep a Changelog format
///
/// Looks for semver version patterns like `## [x.y.z]` in the changelog.
///
/// # Arguments
///
/// * `scanner` - The scanner to access repository files
///
/// # Returns
///
/// A vector of findings for non-standard changelog format
async fn check_changelog_format(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    if let Some(changelog_path) = find_changelog(scanner) {
        if let Ok(content) = scanner.read_file(changelog_path) {
            // Look for ## [x.y.z] pattern (Keep a Changelog format)
            let has_semver_headers = content.lines().any(|line| {
                let trimmed = line.trim();
                trimmed.starts_with("## [")
                    && trimmed.contains('.')
                    && (trimmed.contains(']') || trimmed.ends_with(']'))
            });

            if !has_semver_headers {
                findings.push(
                    Finding::new(
                        "DOC009",
                        "docs",
                        Severity::Info,
                        "CHANGELOG does not follow Keep a Changelog format",
                    )
                    .with_location(changelog_path)
                    .with_description(
                        "The Keep a Changelog format uses '## [x.y.z]' headers for each version, making it easier to parse and read.",
                    )
                    .with_remediation(
                        "Consider reformatting your CHANGELOG to follow Keep a Changelog (https://keepachangelog.com).",
                    ),
                );
            }
        }
    }

    Ok(findings)
}

/// Check if CHANGELOG has content in the Unreleased section
///
/// If an `## [Unreleased]` section exists, checks whether it has content.
///
/// # Arguments
///
/// * `scanner` - The scanner to access repository files
///
/// # Returns
///
/// A vector of findings for empty Unreleased section
async fn check_changelog_unreleased(scanner: &Scanner) -> Result<Vec<Finding>, RepoLensError> {
    let mut findings = Vec::new();

    if let Some(changelog_path) = find_changelog(scanner) {
        if let Ok(content) = scanner.read_file(changelog_path) {
            let lines: Vec<&str> = content.lines().collect();

            // Find ## [Unreleased] section
            let unreleased_idx = lines.iter().position(|line| {
                let trimmed = line.trim().to_lowercase();
                trimmed.starts_with("## [unreleased]") || trimmed == "## [unreleased]"
            });

            if let Some(idx) = unreleased_idx {
                // Find the next ## [ section
                let next_section_idx = lines
                    .iter()
                    .skip(idx + 1)
                    .position(|line| line.trim().starts_with("## ["))
                    .map(|pos| pos + idx + 1);

                let end = next_section_idx.unwrap_or(lines.len());

                // Check if there's any non-empty content between Unreleased header and next section
                let has_content = lines[idx + 1..end]
                    .iter()
                    .any(|line| !line.trim().is_empty());

                if !has_content {
                    findings.push(
                        Finding::new(
                            "DOC010",
                            "docs",
                            Severity::Info,
                            "CHANGELOG has empty Unreleased section",
                        )
                        .with_location(changelog_path)
                        .with_description(
                            "The [Unreleased] section in the CHANGELOG is empty. Consider adding pending changes or removing the section until there are changes to document.",
                        )
                        .with_remediation(
                            "Add pending changes to the [Unreleased] section or remove it until there are changes to document.",
                        ),
                    );
                }
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
    async fn test_check_readme_missing() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_readme(&scanner).await.unwrap();

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id == "DOC001"));
    }

    #[tokio::test]
    async fn test_check_readme_too_short() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let readme = root.join("README.md");

        fs::write(&readme, "# Test\n\nShort.").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_readme(&scanner).await.unwrap();

        assert!(findings.iter().any(|f| f.rule_id == "DOC002"));
    }

    #[tokio::test]
    async fn test_check_readme_missing_sections() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        let readme = root.join("README.md");

        fs::write(&readme, "# Project\n\nDescription here.\n\nMore content.\n\nEven more.\n\nAnd more.\n\nAnd more.\n\nAnd more.\n\nAnd more.\n\nAnd more.\n\nAnd more.\n\nAnd more.").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_readme(&scanner).await.unwrap();

        assert!(findings.iter().any(|f| f.rule_id == "DOC003"));
    }

    #[tokio::test]
    async fn test_check_license_missing() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let scanner = Scanner::new(root.to_path_buf());
        let config = Config::default();
        let findings = check_license(&scanner, &config).await.unwrap();

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id == "DOC004"));
    }

    #[tokio::test]
    async fn test_check_license_enterprise_optional() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let scanner = Scanner::new(root.to_path_buf());
        let config = Config {
            preset: "enterprise".to_string(),
            ..Default::default()
        };
        let findings = check_license(&scanner, &config).await.unwrap();

        assert!(findings.is_empty());
    }

    #[tokio::test]
    async fn test_check_contributing_missing() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_contributing(&scanner).await.unwrap();

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id == "DOC005"));
    }

    #[tokio::test]
    async fn test_check_code_of_conduct_missing() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_code_of_conduct(&scanner).await.unwrap();

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id == "DOC006"));
    }

    #[tokio::test]
    async fn test_check_security_missing() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_security(&scanner).await.unwrap();

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id == "DOC007"));
    }

    // ===== DOC008: CHANGELOG Tests =====

    #[tokio::test]
    async fn test_check_changelog_missing() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_changelog(&scanner).await.unwrap();

        assert!(!findings.is_empty());
        assert!(findings.iter().any(|f| f.rule_id == "DOC008"));
    }

    #[tokio::test]
    async fn test_check_changelog_present() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::write(
            root.join("CHANGELOG.md"),
            "# Changelog\n\n## [1.0.0]\n\n- Initial release",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_changelog(&scanner).await.unwrap();

        assert!(findings.iter().all(|f| f.rule_id != "DOC008"));
    }

    #[tokio::test]
    async fn test_check_changelog_present_as_history() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::write(
            root.join("HISTORY.md"),
            "# History\n\n## 1.0.0\n\n- First release",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_changelog(&scanner).await.unwrap();

        assert!(findings.iter().all(|f| f.rule_id != "DOC008"));
    }

    #[tokio::test]
    async fn test_check_changelog_present_as_changes() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::write(root.join("CHANGES.md"), "# Changes").unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_changelog(&scanner).await.unwrap();

        assert!(findings.iter().all(|f| f.rule_id != "DOC008"));
    }

    // ===== DOC009: CHANGELOG Format Tests =====

    #[tokio::test]
    async fn test_check_changelog_format_valid() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::write(
            root.join("CHANGELOG.md"),
            "# Changelog\n\n## [Unreleased]\n\n## [1.0.0] - 2024-01-01\n\n### Added\n- Initial release\n",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_changelog_format(&scanner).await.unwrap();

        assert!(findings.iter().all(|f| f.rule_id != "DOC009"));
    }

    #[tokio::test]
    async fn test_check_changelog_format_invalid() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::write(
            root.join("CHANGELOG.md"),
            "# Changelog\n\nJust some text about what changed.\nNo structured headers here.\n",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_changelog_format(&scanner).await.unwrap();

        assert!(findings.iter().any(|f| f.rule_id == "DOC009"));
    }

    #[tokio::test]
    async fn test_check_changelog_format_no_changelog() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_changelog_format(&scanner).await.unwrap();

        // No changelog file means no DOC009 finding
        assert!(findings.iter().all(|f| f.rule_id != "DOC009"));
    }

    // ===== DOC010: CHANGELOG Unreleased Tests =====

    #[tokio::test]
    async fn test_check_changelog_unreleased_with_content() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::write(
            root.join("CHANGELOG.md"),
            "# Changelog\n\n## [Unreleased]\n\n### Added\n- New feature\n\n## [1.0.0] - 2024-01-01\n\n- Initial release\n",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_changelog_unreleased(&scanner).await.unwrap();

        assert!(findings.iter().all(|f| f.rule_id != "DOC010"));
    }

    #[tokio::test]
    async fn test_check_changelog_unreleased_empty() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::write(
            root.join("CHANGELOG.md"),
            "# Changelog\n\n## [Unreleased]\n\n## [1.0.0] - 2024-01-01\n\n- Initial release\n",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_changelog_unreleased(&scanner).await.unwrap();

        assert!(findings.iter().any(|f| f.rule_id == "DOC010"));
    }

    #[tokio::test]
    async fn test_check_changelog_unreleased_no_unreleased_section() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::write(
            root.join("CHANGELOG.md"),
            "# Changelog\n\n## [1.0.0] - 2024-01-01\n\n- Initial release\n",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_changelog_unreleased(&scanner).await.unwrap();

        // No Unreleased section, so no DOC010 finding
        assert!(findings.iter().all(|f| f.rule_id != "DOC010"));
    }

    #[tokio::test]
    async fn test_check_changelog_unreleased_no_changelog() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_changelog_unreleased(&scanner).await.unwrap();

        // No changelog file means no DOC010 finding
        assert!(findings.iter().all(|f| f.rule_id != "DOC010"));
    }

    #[tokio::test]
    async fn test_check_changelog_unreleased_at_end_of_file_empty() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();
        fs::write(
            root.join("CHANGELOG.md"),
            "# Changelog\n\n## [1.0.0] - 2024-01-01\n\n- Initial release\n\n## [Unreleased]\n",
        )
        .unwrap();

        let scanner = Scanner::new(root.to_path_buf());
        let findings = check_changelog_unreleased(&scanner).await.unwrap();

        assert!(findings.iter().any(|f| f.rule_id == "DOC010"));
    }
}
