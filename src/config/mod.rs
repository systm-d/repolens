//! # Configuration Module
//!
//! This module handles all configuration-related functionality for RepoLens,
//! including loading configuration files, managing presets, and providing
//! rule-specific settings.
//!
//! ## Configuration Priority
//!
//! Configuration is loaded with the following priority (highest to lowest):
//!
//! 1. CLI flags (handled by clap)
//! 2. Environment variables (`REPOLENS_*`)
//! 3. Configuration file (`.repolens.toml`)
//! 4. Default values
//!
//! ## Configuration File
//!
//! The default configuration file is `.repolens.toml` in the project root.
//!
//! ```toml
//! preset = "opensource"
//!
//! [actions]
//! gitignore = true
//! contributing = true
//!
//! [actions.license]
//! enabled = true
//! license_type = "MIT"
//! author = "Your Name"
//!
//! [actions.branch_protection]
//! enabled = true
//! required_approvals = 1
//!
//! ["rules.secrets"]
//! ignore_patterns = ["test_*"]
//! ignore_files = ["*.test.ts"]
//!
//! ["rules.custom"."no-todo"]
//! pattern = "TODO"
//! severity = "warning"
//! files = ["**/*.rs"]
//! message = "TODO comment found"
//! ```
//!
//! ## Environment Variables
//!
//! | Variable | Description |
//! |----------|-------------|
//! | `REPOLENS_PRESET` | Override preset (opensource, enterprise, strict) |
//! | `REPOLENS_CONFIG` | Path to configuration file |
//! | `REPOLENS_VERBOSE` | Verbosity level (0-3) |
//! | `REPOLENS_NO_CACHE` | Disable caching (true/false) |
//! | `REPOLENS_GITHUB_TOKEN` | GitHub API token |
//!
//! ## Examples
//!
//! ### Loading Configuration
//!
//! ```rust,no_run
//! use repolens::config::Config;
//!
//! // Load from default location or environment
//! let config = Config::load_or_default().expect("Failed to load config");
//!
//! // Check preset
//! println!("Using preset: {}", config.preset);
//! ```
//!
//! ### Creating from Preset
//!
//! ```rust
//! use repolens::config::{Config, Preset};
//!
//! let config = Config::from_preset(Preset::Enterprise);
//! assert_eq!(config.preset, "enterprise");
//! ```
//!
//! ### Checking Rule Configuration
//!
//! ```rust
//! use repolens::config::Config;
//!
//! let config = Config::default();
//!
//! // Check if a rule is enabled
//! if config.is_rule_enabled("SEC001") {
//!     println!("Secret detection is enabled");
//! }
//! ```

pub mod loader;
pub mod presets;

pub use loader::get_env_verbosity;
pub use loader::Config;
pub use presets::Preset;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Re-export CacheConfig from cache module for convenience
pub use crate::cache::CacheConfig;

// Re-export HooksConfig from hooks module for convenience
pub use crate::hooks::HooksConfig;

/// Configuration for individual audit rules.
///
/// Allows enabling/disabling rules and overriding their default severity.
///
/// # Examples
///
/// ```toml
/// [rules.SEC001]
/// enabled = false
/// severity = "warning"
/// ```
///
/// ```rust
/// use repolens::config::RuleConfig;
///
/// let rule = RuleConfig {
///     enabled: true,
///     severity: Some("critical".to_string()),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuleConfig {
    /// Whether the rule is enabled. Defaults to `true`.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Severity override (critical, warning, info).
    /// If `None`, the rule's default severity is used.
    pub severity: Option<String>,
}

fn default_true() -> bool {
    true
}

/// Configuration for secrets detection.
///
/// Controls which patterns and files are scanned for secrets,
/// and allows defining custom secret patterns.
///
/// # Examples
///
/// ```toml
/// ["rules.secrets"]
/// ignore_patterns = ["test_*", "*_mock"]
/// ignore_files = ["*.test.ts", "fixtures/**"]
/// custom_patterns = ["MY_SECRET_\\w+"]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecretsConfig {
    /// Patterns to ignore when scanning for secrets.
    /// Supports glob patterns like `test_*` or `*_mock`.
    #[serde(default)]
    pub ignore_patterns: Vec<String>,

    /// Files to ignore when scanning for secrets.
    /// Supports glob patterns like `*.test.ts` or `vendor/**`.
    #[serde(default)]
    pub ignore_files: Vec<String>,

    /// Custom regex patterns to detect as secrets.
    /// Added to the default secret detection patterns.
    #[serde(default)]
    pub custom_patterns: Vec<String>,
}

/// Configuration for URL validation.
///
/// Used primarily in enterprise mode to allow internal URLs
/// that would otherwise be flagged as potential issues.
///
/// # Examples
///
/// ```toml
/// ["rules.urls"]
/// allowed_internal = ["https://internal.company.com/*", "http://localhost:*"]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UrlConfig {
    /// Allowed internal URLs (for enterprise mode).
    /// Supports glob patterns for URL matching.
    #[serde(default)]
    pub allowed_internal: Vec<String>,
}

/// Configuration for remediation actions.
///
/// Controls which automated fixes and file generations are enabled
/// when running `repolens apply`.
///
/// # Examples
///
/// ```toml
/// [actions]
/// gitignore = true
/// contributing = true
/// code_of_conduct = true
/// security_policy = true
///
/// [actions.license]
/// enabled = true
/// license_type = "MIT"
///
/// [actions.branch_protection]
/// enabled = true
/// required_approvals = 2
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionsConfig {
    /// Whether to update `.gitignore` with recommended entries.
    #[serde(default = "default_true")]
    pub gitignore: bool,

    /// License file generation configuration.
    #[serde(default)]
    pub license: LicenseConfig,

    /// Whether to create `CONTRIBUTING.md` if missing.
    #[serde(default = "default_true")]
    pub contributing: bool,

    /// Whether to create `CODE_OF_CONDUCT.md` if missing.
    #[serde(default = "default_true")]
    pub code_of_conduct: bool,

    /// Whether to create `SECURITY.md` if missing.
    #[serde(default = "default_true")]
    pub security_policy: bool,

    /// GitHub branch protection rule configuration.
    #[serde(default)]
    pub branch_protection: BranchProtectionConfig,

    /// GitHub repository settings configuration.
    #[serde(default)]
    pub github_settings: GitHubSettingsConfig,
}

impl Default for ActionsConfig {
    fn default() -> Self {
        Self {
            gitignore: true,
            license: LicenseConfig::default(),
            contributing: true,
            code_of_conduct: true,
            security_policy: true,
            branch_protection: BranchProtectionConfig::default(),
            github_settings: GitHubSettingsConfig::default(),
        }
    }
}

/// Configuration for LICENSE file generation.
///
/// Controls the license type and metadata used when generating
/// a LICENSE file.
///
/// # Supported License Types
///
/// - `MIT` - MIT License (default)
/// - `Apache-2.0` - Apache License 2.0
/// - `GPL-3.0` - GNU General Public License v3.0
/// - `BSD-3-Clause` - BSD 3-Clause License
/// - `UNLICENSED` - Proprietary/No License
///
/// # Examples
///
/// ```toml
/// [actions.license]
/// enabled = true
/// license_type = "Apache-2.0"
/// author = "Your Name"
/// year = "2024"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseConfig {
    /// Whether to create LICENSE file if missing.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// License type (MIT, Apache-2.0, GPL-3.0, etc.).
    /// Defaults to "MIT".
    #[serde(default = "default_license_type")]
    pub license_type: String,

    /// Author name for license. If not set, attempts to
    /// detect from git configuration.
    #[serde(default)]
    pub author: Option<String>,

    /// Year for license. Defaults to current year if not specified.
    #[serde(default)]
    pub year: Option<String>,
}

impl Default for LicenseConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            license_type: "MIT".to_string(),
            author: None,
            year: None,
        }
    }
}

fn default_license_type() -> String {
    "MIT".to_string()
}

/// Configuration for GitHub branch protection rules.
///
/// These settings are applied via the GitHub API when running
/// `repolens apply` with appropriate permissions.
///
/// # Examples
///
/// ```toml
/// [actions.branch_protection]
/// enabled = true
/// branch = "main"
/// required_approvals = 2
/// require_status_checks = true
/// block_force_push = true
/// require_signed_commits = true
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchProtectionConfig {
    /// Whether to enable branch protection rules.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Branch to protect. Defaults to "main".
    #[serde(default = "default_branch")]
    pub branch: String,

    /// Number of required pull request approvals.
    /// Defaults to 1, enterprise preset uses 2.
    #[serde(default = "default_approvals")]
    pub required_approvals: u32,

    /// Whether to require status checks to pass before merging.
    #[serde(default = "default_true")]
    pub require_status_checks: bool,

    /// Whether to block force pushes to the protected branch.
    #[serde(default = "default_true")]
    pub block_force_push: bool,

    /// Whether to require signed commits.
    /// Defaults to `false`, enterprise/strict presets enable this.
    #[serde(default)]
    pub require_signed_commits: bool,
}

impl Default for BranchProtectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            branch: "main".to_string(),
            required_approvals: 1,
            require_status_checks: true,
            block_force_push: true,
            require_signed_commits: false,
        }
    }
}

fn default_branch() -> String {
    "main".to_string()
}

fn default_approvals() -> u32 {
    1
}

/// Configuration for GitHub repository settings.
///
/// These settings are applied via the GitHub API when running
/// `repolens apply` with appropriate permissions.
///
/// # Examples
///
/// ```toml
/// [actions.github_settings]
/// discussions = true
/// issues = true
/// wiki = false
/// vulnerability_alerts = true
/// automated_security_fixes = true
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubSettingsConfig {
    /// Whether to enable GitHub Discussions for the repository.
    #[serde(default = "default_true")]
    pub discussions: bool,

    /// Whether to enable GitHub Issues for the repository.
    #[serde(default = "default_true")]
    pub issues: bool,

    /// Whether to enable GitHub Wiki for the repository.
    /// Defaults to `false` as wikis are often unused.
    #[serde(default)]
    pub wiki: bool,

    /// Whether to enable Dependabot vulnerability alerts.
    #[serde(default = "default_true")]
    pub vulnerability_alerts: bool,

    /// Whether to enable Dependabot automatic security fixes.
    #[serde(default = "default_true")]
    pub automated_security_fixes: bool,
}

impl Default for GitHubSettingsConfig {
    fn default() -> Self {
        Self {
            discussions: true,
            issues: true,
            wiki: false,
            vulnerability_alerts: true,
            automated_security_fixes: true,
        }
    }
}

/// Configuration for file template generation.
///
/// These values are used when generating files like LICENSE,
/// CONTRIBUTING.md, and other template-based files.
///
/// # Examples
///
/// ```toml
/// [templates]
/// license_author = "Your Company"
/// license_year = "2024"
/// project_name = "My Project"
/// project_description = "A description of the project"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TemplatesConfig {
    /// Author name for license and other templates.
    /// Overrides auto-detected git user name.
    pub license_author: Option<String>,

    /// Year for license templates.
    /// Defaults to current year if not specified.
    pub license_year: Option<String>,

    /// Project name override.
    /// Defaults to repository/directory name if not specified.
    pub project_name: Option<String>,

    /// Project description for generated files.
    pub project_description: Option<String>,
}

/// Configuration for a custom audit rule.
///
/// Custom rules allow defining project-specific checks using either
/// regex patterns or shell commands.
///
/// # Pattern-based Rules
///
/// ```toml
/// ["rules.custom"."no-todo"]
/// pattern = "TODO|FIXME"
/// severity = "warning"
/// files = ["**/*.rs", "**/*.py"]
/// message = "Found TODO/FIXME comment"
/// remediation = "Complete the task or remove the comment"
/// ```
///
/// # Command-based Rules
///
/// ```toml
/// ["rules.custom"."has-makefile"]
/// command = "test -f Makefile"
/// severity = "info"
/// message = "Makefile not found"
/// invert = true
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomRule {
    /// Regex pattern to match in file contents.
    /// Required if `command` is not set.
    #[serde(default)]
    pub pattern: Option<String>,

    /// Shell command to execute for the check.
    /// The rule triggers if the command returns exit code 0
    /// (or non-zero if `invert` is true).
    /// Required if `pattern` is not set.
    #[serde(default)]
    pub command: Option<String>,

    /// Severity level: "critical", "warning", or "info".
    /// Defaults to "warning".
    #[serde(default = "default_custom_severity")]
    pub severity: String,

    /// File glob patterns to scan (only used with `pattern`).
    /// If empty, all files are scanned.
    #[serde(default)]
    pub files: Vec<String>,

    /// Custom message shown when the rule triggers.
    pub message: Option<String>,

    /// Detailed description of the issue.
    pub description: Option<String>,

    /// Suggested steps to fix the issue.
    pub remediation: Option<String>,

    /// If true, inverts the matching logic:
    /// - For patterns: triggers when pattern is NOT found
    /// - For commands: triggers when command returns non-zero
    #[serde(default)]
    pub invert: bool,
}

fn default_custom_severity() -> String {
    "warning".to_string()
}

/// Container for custom rule definitions.
///
/// Custom rules are defined under the `["rules.custom"]` section
/// in the configuration file.
///
/// # Examples
///
/// ```toml
/// ["rules.custom"."rule-id"]
/// pattern = "some_pattern"
/// severity = "warning"
/// message = "Issue found"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CustomRulesConfig {
    /// Map of rule ID to rule configuration.
    /// Rule IDs should be kebab-case (e.g., "no-todo", "require-tests").
    #[serde(flatten)]
    pub rules: HashMap<String, CustomRule>,
}

/// Configuration for dependency license compliance checking.
///
/// Allows specifying which licenses are allowed or denied for
/// project dependencies.
///
/// # Examples
///
/// ```toml
/// ["rules.licenses"]
/// enabled = true
/// allowed_licenses = ["MIT", "Apache-2.0", "BSD-3-Clause"]
/// denied_licenses = ["GPL-3.0", "AGPL-3.0"]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseComplianceConfig {
    /// Whether license compliance checking is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// List of allowed SPDX license identifiers.
    /// If empty, all known licenses are allowed (unless in `denied_licenses`).
    /// Example: `["MIT", "Apache-2.0", "BSD-3-Clause"]`
    #[serde(default)]
    pub allowed_licenses: Vec<String>,

    /// List of denied SPDX license identifiers.
    /// Dependencies with these licenses will be flagged.
    /// Example: `["GPL-3.0", "AGPL-3.0"]`
    #[serde(default)]
    pub denied_licenses: Vec<String>,
}

impl Default for LicenseComplianceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            allowed_licenses: Vec::new(),
            denied_licenses: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rule_config_default() {
        let config = RuleConfig::default();
        assert!(!config.enabled); // Default for bool is false
        assert!(config.severity.is_none());
    }

    #[test]
    fn test_rule_config_deserialize() {
        let toml_str = r#"
            enabled = true
            severity = "critical"
        "#;
        let config: RuleConfig = toml::from_str(toml_str).unwrap();
        assert!(config.enabled);
        assert_eq!(config.severity, Some("critical".to_string()));
    }

    #[test]
    fn test_secrets_config_default() {
        let config = SecretsConfig::default();
        assert!(config.ignore_patterns.is_empty());
        assert!(config.ignore_files.is_empty());
        assert!(config.custom_patterns.is_empty());
    }

    #[test]
    fn test_url_config_default() {
        let config = UrlConfig::default();
        assert!(config.allowed_internal.is_empty());
    }

    #[test]
    fn test_actions_config_default() {
        let config = ActionsConfig::default();
        assert!(config.gitignore);
        assert!(config.contributing);
        assert!(config.code_of_conduct);
        assert!(config.security_policy);
    }

    #[test]
    fn test_license_config_default() {
        let config = LicenseConfig::default();
        assert!(config.enabled);
        assert_eq!(config.license_type, "MIT");
        assert!(config.author.is_none());
        assert!(config.year.is_none());
    }

    #[test]
    fn test_branch_protection_config_default() {
        let config = BranchProtectionConfig::default();
        assert!(config.enabled);
        assert_eq!(config.branch, "main");
        assert_eq!(config.required_approvals, 1);
        assert!(config.require_status_checks);
        assert!(config.block_force_push);
        assert!(!config.require_signed_commits);
    }

    #[test]
    fn test_github_settings_config_default() {
        let config = GitHubSettingsConfig::default();
        assert!(config.discussions);
        assert!(config.issues);
        assert!(!config.wiki);
        assert!(config.vulnerability_alerts);
        assert!(config.automated_security_fixes);
    }

    #[test]
    fn test_templates_config_default() {
        let config = TemplatesConfig::default();
        assert!(config.license_author.is_none());
        assert!(config.license_year.is_none());
        assert!(config.project_name.is_none());
        assert!(config.project_description.is_none());
    }

    #[test]
    fn test_custom_rule_deserialize() {
        let toml_str = r#"
            pattern = "TODO|FIXME"
            severity = "warning"
            files = ["*.rs", "*.py"]
            message = "Found TODO comment"
            description = "TODO comments should be addressed"
            remediation = "Complete the task or remove the comment"
            invert = false
        "#;
        let rule: CustomRule = toml::from_str(toml_str).unwrap();
        assert_eq!(rule.pattern, Some("TODO|FIXME".to_string()));
        assert_eq!(rule.severity, "warning");
        assert_eq!(rule.files.len(), 2);
        assert!(!rule.invert);
    }

    #[test]
    fn test_custom_rule_with_command() {
        let toml_str = r#"
            command = "test -f Makefile"
            severity = "info"
            message = "Makefile not found"
            invert = true
        "#;
        let rule: CustomRule = toml::from_str(toml_str).unwrap();
        assert!(rule.pattern.is_none());
        assert_eq!(rule.command, Some("test -f Makefile".to_string()));
        assert!(rule.invert);
    }

    #[test]
    fn test_custom_rules_config_default() {
        let config = CustomRulesConfig::default();
        assert!(config.rules.is_empty());
    }

    #[test]
    fn test_default_true_function() {
        assert!(default_true());
    }

    #[test]
    fn test_default_license_type_function() {
        assert_eq!(default_license_type(), "MIT");
    }

    #[test]
    fn test_default_branch_function() {
        assert_eq!(default_branch(), "main");
    }

    #[test]
    fn test_default_approvals_function() {
        assert_eq!(default_approvals(), 1);
    }

    #[test]
    fn test_default_custom_severity_function() {
        assert_eq!(default_custom_severity(), "warning");
    }

    #[test]
    fn test_license_compliance_config_default() {
        let config = LicenseComplianceConfig::default();
        assert!(config.enabled);
        assert!(config.allowed_licenses.is_empty());
        assert!(config.denied_licenses.is_empty());
    }

    #[test]
    fn test_license_compliance_config_deserialize() {
        let toml_str = r#"
            enabled = true
            allowed_licenses = ["MIT", "Apache-2.0"]
            denied_licenses = ["GPL-3.0"]
        "#;
        let config: LicenseComplianceConfig = toml::from_str(toml_str).unwrap();
        assert!(config.enabled);
        assert_eq!(config.allowed_licenses.len(), 2);
        assert_eq!(config.denied_licenses.len(), 1);
        assert_eq!(config.allowed_licenses[0], "MIT");
        assert_eq!(config.denied_licenses[0], "GPL-3.0");
    }

    #[test]
    fn test_license_compliance_config_deserialize_defaults() {
        let toml_str = r#""#;
        let config: LicenseComplianceConfig = toml::from_str(toml_str).unwrap();
        assert!(config.enabled);
        assert!(config.allowed_licenses.is_empty());
        assert!(config.denied_licenses.is_empty());
    }
}
