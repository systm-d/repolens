//! Configuration loader
//!
//! Configuration priority (highest to lowest):
//! 1. CLI flags (handled by clap)
//! 2. Environment variables (REPOLENS_*)
//! 3. Configuration file (.repolens.toml)
//! 4. Default values

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{ConfigError, RepoLensError};

use super::presets::Preset;
use super::{
    ActionsConfig, CacheConfig, CustomRulesConfig, HooksConfig, LicenseComplianceConfig,
    RuleConfig, SecretsConfig, TemplatesConfig, UrlConfig,
};

const CONFIG_FILENAME: &str = ".repolens.toml";

/// Environment variable names for configuration
pub mod env_vars {
    /// Preset name (opensource, enterprise, strict)
    pub const REPOLENS_PRESET: &str = "REPOLENS_PRESET";
    /// Path to config file (alternative to -c flag)
    pub const REPOLENS_CONFIG: &str = "REPOLENS_CONFIG";
    /// Verbosity level (0-3)
    pub const REPOLENS_VERBOSE: &str = "REPOLENS_VERBOSE";
    /// Disable cache (true/false)
    pub const REPOLENS_NO_CACHE: &str = "REPOLENS_NO_CACHE";
    /// GitHub token for API calls
    pub const REPOLENS_GITHUB_TOKEN: &str = "REPOLENS_GITHUB_TOKEN";
}

/// Get verbosity level from environment variable
pub fn get_env_verbosity() -> Option<u8> {
    std::env::var(env_vars::REPOLENS_VERBOSE)
        .ok()
        .and_then(|v| v.parse::<u8>().ok())
        .map(|v| v.min(3)) // Clamp to max 3
}

/// Get config path from environment variable
pub fn get_env_config_path() -> Option<PathBuf> {
    std::env::var(env_vars::REPOLENS_CONFIG)
        .ok()
        .map(PathBuf::from)
}

/// Parse boolean environment variable
fn parse_bool_env(name: &str) -> Option<bool> {
    std::env::var(name)
        .ok()
        .map(|v| matches!(v.to_lowercase().as_str(), "true" | "1" | "yes" | "on"))
}

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Preset name (opensource, enterprise, strict)
    #[serde(default = "default_preset")]
    pub preset: String,

    /// Rule overrides
    #[serde(default)]
    pub rules: HashMap<String, RuleConfig>,

    /// Secrets detection configuration
    #[serde(default)]
    #[serde(rename = "rules.secrets")]
    pub secrets: SecretsConfig,

    /// URL detection configuration
    #[serde(default)]
    #[serde(rename = "rules.urls")]
    pub urls: UrlConfig,

    /// Actions configuration
    #[serde(default)]
    pub actions: ActionsConfig,

    /// Template configuration
    #[serde(default)]
    pub templates: TemplatesConfig,

    /// Custom rules configuration
    #[serde(default)]
    #[serde(rename = "rules.custom")]
    pub custom_rules: CustomRulesConfig,

    /// License compliance configuration
    #[serde(default)]
    #[serde(rename = "rules.licenses")]
    pub license_compliance: LicenseComplianceConfig,

    /// Cache configuration
    #[serde(default)]
    pub cache: CacheConfig,

    /// Git hooks configuration
    #[serde(default)]
    pub hooks: HooksConfig,
}

fn default_preset() -> String {
    "opensource".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            preset: "opensource".to_string(),
            rules: HashMap::new(),
            secrets: SecretsConfig::default(),
            urls: UrlConfig::default(),
            actions: ActionsConfig::default(),
            templates: TemplatesConfig::default(),
            custom_rules: CustomRulesConfig::default(),
            license_compliance: LicenseComplianceConfig::default(),
            cache: CacheConfig::default(),
            hooks: HooksConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from file or return default
    ///
    /// Priority: CLI flags > env vars > config file > defaults
    pub fn load_or_default() -> Result<Self, RepoLensError> {
        // Check REPOLENS_CONFIG env var first
        let config_path = get_env_config_path()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from(CONFIG_FILENAME));

        let mut config = if config_path.exists() {
            Self::load_from_file(&config_path)?
        } else if get_env_config_path().is_some() {
            // If env var specified a path that doesn't exist, error
            return Err(RepoLensError::Config(ConfigError::FileRead {
                path: config_path.display().to_string(),
                source: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Configuration file not found",
                ),
            }));
        } else {
            Self::default()
        };

        // Apply environment variable overrides
        config.apply_env_overrides();

        Ok(config)
    }

    /// Apply environment variable overrides to configuration
    fn apply_env_overrides(&mut self) {
        // REPOLENS_PRESET
        if let Ok(preset_str) = std::env::var(env_vars::REPOLENS_PRESET) {
            if let Ok(preset) = preset_str.parse::<Preset>() {
                let preset_config = Config::from_preset(preset);
                self.preset = preset_config.preset;
                self.actions = preset_config.actions;
            }
        }

        // REPOLENS_NO_CACHE
        if let Some(true) = parse_bool_env(env_vars::REPOLENS_NO_CACHE) {
            self.cache.enabled = false;
        }

        // REPOLENS_GITHUB_TOKEN - forward to GH_TOKEN for gh CLI
        if let Ok(token) = std::env::var(env_vars::REPOLENS_GITHUB_TOKEN) {
            // TODO: Audit that the environment access only happens in single-threaded code.
            unsafe { std::env::set_var("GH_TOKEN", token) };
        }
    }

    /// Load configuration from a specific file
    pub fn load_from_file(path: &Path) -> Result<Self, RepoLensError> {
        let content = fs::read_to_string(path).map_err(|e| {
            RepoLensError::Config(ConfigError::FileRead {
                path: path.display().to_string(),
                source: e,
            })
        })?;

        toml::from_str(&content).map_err(Into::into)
    }

    /// Create a new configuration from a preset
    pub fn from_preset(preset: Preset) -> Self {
        let mut config = Self {
            preset: preset.name().to_string(),
            ..Default::default()
        };

        match preset {
            Preset::OpenSource => {
                config.actions.license.enabled = true;
                config.actions.contributing = true;
                config.actions.code_of_conduct = true;
                config.actions.security_policy = true;
                config.actions.github_settings.discussions = true;
            }
            Preset::Enterprise => {
                config.actions.license.enabled = false;
                config.actions.contributing = false;
                config.actions.code_of_conduct = false;
                config.actions.security_policy = true;
                config.actions.branch_protection.required_approvals = 2;
                config.actions.branch_protection.require_signed_commits = true;
                config.actions.github_settings.discussions = false;
            }
            Preset::Strict => {
                config.actions.license.enabled = true;
                config.actions.contributing = true;
                config.actions.code_of_conduct = true;
                config.actions.security_policy = true;
                config.actions.branch_protection.required_approvals = 2;
                config.actions.branch_protection.require_signed_commits = true;
                config.actions.github_settings.discussions = true;
            }
        }

        config
    }

    /// Serialize configuration to TOML
    pub fn to_toml(&self) -> Result<String, RepoLensError> {
        toml::to_string_pretty(self).map_err(Into::into)
    }

    /// Check if a rule is enabled
    pub fn is_rule_enabled(&self, rule_id: &str) -> bool {
        self.rules.get(rule_id).map(|r| r.enabled).unwrap_or(true)
    }

    /// Get severity override for a rule.
    ///
    /// Part of the public API for configuration rule customization.
    /// Allows external code to query custom severity levels for specific rules.
    #[allow(dead_code)]
    pub fn get_rule_severity(&self, rule_id: &str) -> Option<&str> {
        self.rules.get(rule_id).and_then(|r| r.severity.as_deref())
    }

    /// Check if a file should be ignored for secrets scanning
    pub fn should_ignore_file(&self, file_path: &str) -> bool {
        self.secrets
            .ignore_files
            .iter()
            .any(|pattern| glob_match(pattern, file_path))
    }

    /// Check if a pattern should be ignored for secrets scanning
    pub fn should_ignore_pattern(&self, path: &str) -> bool {
        self.secrets
            .ignore_patterns
            .iter()
            .any(|pattern| glob_match(pattern, path))
    }

    /// Check if a URL is allowed (for enterprise mode).
    ///
    /// Part of the public API for enterprise URL filtering.
    /// Allows external code to check if internal URLs are permitted.
    #[allow(dead_code)]
    pub fn is_url_allowed(&self, url: &str) -> bool {
        if self.urls.allowed_internal.is_empty() {
            return false;
        }

        self.urls
            .allowed_internal
            .iter()
            .any(|pattern| glob_match(pattern, url))
    }
}

fn glob_match(pattern: &str, text: &str) -> bool {
    if pattern.contains("**") {
        return glob_match_double_star(pattern, text);
    }

    if pattern.contains('*') {
        return glob_match_single_star(pattern, text);
    }

    text == pattern
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_glob_match() {
        assert!(glob_match("*.ts", "file.ts"));
        assert!(glob_match("*.ts", "path/to/file.ts"));
        assert!(!glob_match("*.ts", "file.js"));
        assert!(
            glob_match("**/test/**", "src/test/file.ts"),
            "Pattern **/test/** should match src/test/file.ts"
        );
        assert!(glob_match("**/test/**", "test/file.ts"));
        assert!(glob_match("**/*.test.ts", "src/file.test.ts"));
    }

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.preset, "opensource");
        assert!(config.actions.gitignore);
        // Verify hooks default
        assert!(config.hooks.pre_commit);
        assert!(config.hooks.pre_push);
        assert!(!config.hooks.fail_on_warnings);
    }

    #[test]
    fn test_from_preset() {
        let config = Config::from_preset(Preset::Enterprise);
        assert_eq!(config.preset, "enterprise");
        assert!(!config.actions.license.enabled);
        assert_eq!(config.actions.branch_protection.required_approvals, 2);
    }

    #[test]
    fn test_custom_rules_config_parsing() {
        let toml_content = r#"
preset = "opensource"

["rules.custom"."no-todo"]
pattern = "TODO"
severity = "warning"
files = ["**/*.rs"]
message = "TODO comment found"
"#;
        let config: Config = toml::from_str(toml_content).unwrap();
        assert!(config.custom_rules.rules.contains_key("no-todo"));
        let rule = config.custom_rules.rules.get("no-todo").unwrap();
        assert_eq!(rule.pattern, Some("TODO".to_string()));
        assert_eq!(rule.severity, "warning");
    }

    #[test]
    fn test_default_preset_function() {
        assert_eq!(default_preset(), "opensource");
    }

    #[test]
    fn test_from_preset_opensource() {
        let config = Config::from_preset(Preset::OpenSource);
        assert_eq!(config.preset, "opensource");
        assert!(config.actions.license.enabled);
        assert!(config.actions.contributing);
        assert!(config.actions.code_of_conduct);
        assert!(config.actions.security_policy);
        assert!(config.actions.github_settings.discussions);
    }

    #[test]
    fn test_from_preset_strict() {
        let config = Config::from_preset(Preset::Strict);
        assert_eq!(config.preset, "strict");
        assert!(config.actions.license.enabled);
        assert!(config.actions.contributing);
        assert_eq!(config.actions.branch_protection.required_approvals, 2);
        assert!(config.actions.branch_protection.require_signed_commits);
    }

    #[test]
    fn test_load_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        fs::write(&config_path, r#"preset = "enterprise""#).unwrap();

        let config = Config::load_from_file(&config_path).unwrap();
        assert_eq!(config.preset, "enterprise");
    }

    #[test]
    fn test_load_from_file_not_found() {
        let result = Config::load_from_file(Path::new("/nonexistent/config.toml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_to_toml() {
        let config = Config::default();
        let toml_str = config.to_toml().unwrap();
        assert!(toml_str.contains("preset = \"opensource\""));
    }

    #[test]
    fn test_is_rule_enabled_default() {
        let config = Config::default();
        assert!(config.is_rule_enabled("nonexistent_rule"));
    }

    #[test]
    fn test_is_rule_enabled_disabled() {
        let mut config = Config::default();
        config.rules.insert(
            "test_rule".to_string(),
            RuleConfig {
                enabled: false,
                severity: None,
            },
        );
        assert!(!config.is_rule_enabled("test_rule"));
    }

    #[test]
    fn test_get_rule_severity() {
        let mut config = Config::default();
        config.rules.insert(
            "test_rule".to_string(),
            RuleConfig {
                enabled: true,
                severity: Some("critical".to_string()),
            },
        );
        assert_eq!(config.get_rule_severity("test_rule"), Some("critical"));
        assert_eq!(config.get_rule_severity("nonexistent"), None);
    }

    #[test]
    fn test_should_ignore_file() {
        let mut config = Config::default();
        config.secrets.ignore_files = vec!["*.min.js".to_string(), "vendor/**".to_string()];

        assert!(config.should_ignore_file("bundle.min.js"));
        assert!(config.should_ignore_file("vendor/lib.js"));
        assert!(!config.should_ignore_file("main.js"));
    }

    #[test]
    fn test_should_ignore_pattern() {
        let mut config = Config::default();
        config.secrets.ignore_patterns = vec!["test_*".to_string(), "*_mock".to_string()];

        assert!(config.should_ignore_pattern("test_secret"));
        assert!(config.should_ignore_pattern("api_mock"));
        assert!(!config.should_ignore_pattern("real_secret"));
    }

    #[test]
    fn test_is_url_allowed() {
        let mut config = Config::default();
        config.urls.allowed_internal = vec![
            "https://internal.company.com/*".to_string(),
            "http://localhost:*".to_string(),
        ];

        assert!(config.is_url_allowed("https://internal.company.com/api"));
        assert!(config.is_url_allowed("http://localhost:3000"));
        assert!(!config.is_url_allowed("https://external.com/api"));
    }

    #[test]
    fn test_is_url_allowed_empty() {
        let config = Config::default();
        assert!(!config.is_url_allowed("any_url"));
    }

    #[test]
    fn test_glob_match_exact() {
        assert!(glob_match("file.txt", "file.txt"));
        assert!(!glob_match("file.txt", "other.txt"));
    }

    #[test]
    fn test_glob_match_single_star() {
        assert!(glob_match("*.txt", "file.txt"));
        assert!(glob_match("file.*", "file.txt"));
        assert!(glob_match("*.txt", "path/to/file.txt"));
        assert!(glob_match("test_*", "test_file"));
        assert!(glob_match("*_test", "my_test"));
    }

    #[test]
    fn test_glob_match_double_star_prefix() {
        assert!(glob_match("src/**", "src/file.txt"));
        assert!(glob_match("src/**", "src/sub/file.txt"));
        assert!(!glob_match("src/**", "other/file.txt"));
    }

    #[test]
    fn test_glob_match_double_star_suffix() {
        assert!(glob_match("**/*.rs", "src/main.rs"));
        assert!(glob_match("**/*.rs", "main.rs"));
    }

    #[test]
    fn test_glob_match_double_star_middle() {
        assert!(glob_match("**/test/**", "src/test/file.txt"));
        assert!(glob_match("**/test/**", "test/file.txt"));
    }

    #[test]
    fn test_glob_match_double_star_with_prefix_and_suffix() {
        // Pattern with prefix and suffix: "src/**/test"
        assert!(glob_match("src/**/test", "src/sub/test"));
        assert!(glob_match("src/**/test", "src/a/b/test"));
        assert!(!glob_match("src/**/test", "other/sub/test"));
    }

    #[test]
    fn test_glob_match_double_star_prefix_only_with_suffix_wildcard() {
        // Pattern like "**/*.test.ts"
        assert!(glob_match("**/*.test.ts", "src/file.test.ts"));
        assert!(glob_match("**/*.test.ts", "file.test.ts"));
    }

    #[test]
    fn test_glob_match_double_star_slash_prefix() {
        // Pattern like "**/src/file.txt"
        assert!(glob_match("**/src/file.txt", "a/b/src/file.txt"));
        assert!(glob_match("**/src/file.txt", "src/file.txt"));
    }

    #[test]
    fn test_glob_match_many_double_stars() {
        // Pattern with more than 2 double-star segments should return false
        assert!(!glob_match("a/**/b/**/c", "a/x/b/y/c"));
    }

    #[test]
    fn test_glob_match_single_star_in_middle() {
        assert!(glob_match("test_*.rs", "test_file.rs"));
        assert!(glob_match("*.min.*", "file.min.js"));
    }

    #[test]
    fn test_glob_match_single_star_no_match() {
        assert!(!glob_match("foo*bar", "foobaz"));
    }

    #[test]
    fn test_load_from_file_invalid_toml() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        fs::write(&config_path, "invalid [[[toml content").unwrap();

        let result = Config::load_from_file(&config_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_glob_match_double_star_no_suffix_no_prefix() {
        // Pattern: "**" should match anything
        assert!(glob_match("**", "any/path"));
        assert!(glob_match("**", "file.txt"));
    }

    #[test]
    fn test_glob_match_double_star_empty_parts() {
        // Pattern where double star results in empty prefix
        assert!(glob_match("**/file.txt", "some/path/file.txt"));
        assert!(glob_match("**/file.txt", "file.txt"));
    }

    #[test]
    fn test_glob_match_single_star_complex() {
        // Complex patterns with multiple single stars
        assert!(glob_match("*.test.*", "file.test.js"));
        assert!(glob_match("src/*.rs", "src/main.rs"));
    }

    #[test]
    fn test_glob_match_double_star_with_wildcard_suffix() {
        // Pattern like "src/**/*.rs" - prefix with double star and wildcard suffix
        // This tests the suffix.starts_with('*') branch
        assert!(glob_match("src/**/*.rs", "src/main.rs"));
        assert!(glob_match("src/**/*.rs", "src/nested/lib.rs"));
    }

    #[test]
    fn test_glob_match_double_star_three_parts() {
        // Three parts pattern like "**/test/**" - handled by the 3-part branch
        assert!(glob_match("**/test/**", "src/test/file.rs"));
        assert!(glob_match("**/test/**", "test/file.rs"));
    }

    #[test]
    fn test_config_full_deserialization() {
        let toml_content = r#"
preset = "strict"

[rules]
[rules.SEC001]
enabled = false
severity = "warning"

["rules.secrets"]
ignore_patterns = ["test_*"]
ignore_files = ["*.test.ts"]
custom_patterns = ["MY_SECRET_\\w+"]

["rules.urls"]
allowed_internal = ["https://internal.example.com/*"]

[actions]
gitignore = true
contributing = true
code_of_conduct = true
security_policy = true

[actions.license]
enabled = true
license_type = "Apache-2.0"
author = "Test Author"
year = "2024"

[actions.branch_protection]
enabled = true
branch = "main"
required_approvals = 2
require_status_checks = true
block_force_push = true
require_signed_commits = true

[actions.github_settings]
discussions = false
issues = true
wiki = false
vulnerability_alerts = true
automated_security_fixes = true

[templates]
license_author = "Template Author"
license_year = "2024"
project_name = "My Project"
project_description = "A test project"

[cache]
enabled = true
ttl_seconds = 3600
"#;
        let config: Config = toml::from_str(toml_content).unwrap();
        assert_eq!(config.preset, "strict");
        assert!(!config.is_rule_enabled("SEC001"));
        assert_eq!(config.get_rule_severity("SEC001"), Some("warning"));
        assert!(config.should_ignore_pattern("test_secret"));
        assert!(config.should_ignore_file("file.test.ts"));
        assert!(config.is_url_allowed("https://internal.example.com/api"));
        assert_eq!(config.actions.license.license_type, "Apache-2.0");
        assert_eq!(config.actions.branch_protection.required_approvals, 2);
        assert_eq!(
            config.templates.project_name,
            Some("My Project".to_string())
        );
    }

    #[test]
    fn test_config_with_license_compliance() {
        let toml_content = r#"
preset = "opensource"

["rules.licenses"]
enabled = true
allowed_licenses = ["MIT", "Apache-2.0", "BSD-3-Clause"]
denied_licenses = ["GPL-3.0", "AGPL-3.0"]
"#;
        let config: Config = toml::from_str(toml_content).unwrap();
        assert!(config.license_compliance.enabled);
        assert_eq!(config.license_compliance.allowed_licenses.len(), 3);
        assert_eq!(config.license_compliance.denied_licenses.len(), 2);
        assert_eq!(config.license_compliance.allowed_licenses[0], "MIT");
        assert_eq!(config.license_compliance.denied_licenses[0], "GPL-3.0");
    }

    #[test]
    fn test_config_default_license_compliance() {
        let config = Config::default();
        assert!(config.license_compliance.enabled);
        assert!(config.license_compliance.allowed_licenses.is_empty());
        assert!(config.license_compliance.denied_licenses.is_empty());
    }

    #[test]
    fn test_config_with_hooks_section() {
        let toml_content = r#"
preset = "opensource"

[hooks]
pre_commit = false
pre_push = true
fail_on_warnings = true
"#;
        let config: Config = toml::from_str(toml_content).unwrap();
        assert!(!config.hooks.pre_commit);
        assert!(config.hooks.pre_push);
        assert!(config.hooks.fail_on_warnings);
    }

    #[test]
    fn test_config_without_hooks_section_uses_defaults() {
        let toml_content = r#"
preset = "enterprise"
"#;
        let config: Config = toml::from_str(toml_content).unwrap();
        assert!(config.hooks.pre_commit);
        assert!(config.hooks.pre_push);
        assert!(!config.hooks.fail_on_warnings);
    }

    #[test]
    fn test_config_to_toml_includes_hooks() {
        let config = Config::default();
        let toml_str = config.to_toml().unwrap();
        assert!(toml_str.contains("pre_commit"));
        assert!(toml_str.contains("pre_push"));
        assert!(toml_str.contains("fail_on_warnings"));
    }

    #[test]
    fn test_glob_match_double_star_slash_prefix_starts_with_suffix() {
        assert!(glob_match("**/lib.rs", "lib.rs"));
        assert!(glob_match("**/src/main.rs", "src/main.rs"));
    }

    #[test]
    fn test_glob_match_double_star_after_prefix_strip() {
        assert!(glob_match("src/**/test", "src/deep/nested/test"));
        assert!(glob_match("src/**/test", "src/test"));
        assert!(!glob_match("src/**/test", "other/test"));
    }

    #[test]
    fn test_glob_match_double_star_fallback_ends_with() {
        assert!(!glob_match("foo/**/bar", "baz/bar"));
    }

    #[test]
    fn test_glob_match_single_star_first_part_not_at_start() {
        assert!(!glob_match("foo*bar", "Xfoobar"));
    }

    #[test]
    fn test_glob_match_single_star_part_not_found() {
        assert!(!glob_match("abc*xyz", "abcdef"));
    }

    #[test]
    fn test_config_load_from_file_with_hooks() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        fs::write(
            &config_path,
            r#"
preset = "enterprise"

[hooks]
pre_commit = false
pre_push = true
fail_on_warnings = true
"#,
        )
        .unwrap();

        let config = Config::load_from_file(&config_path).unwrap();
        assert_eq!(config.preset, "enterprise");
        assert!(!config.hooks.pre_commit);
        assert!(config.hooks.pre_push);
        assert!(config.hooks.fail_on_warnings);
    }

    #[test]
    fn test_config_load_from_file_without_hooks_uses_defaults() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        fs::write(&config_path, r#"preset = "strict""#).unwrap();

        let config = Config::load_from_file(&config_path).unwrap();
        assert_eq!(config.preset, "strict");
        assert!(config.hooks.pre_commit);
        assert!(config.hooks.pre_push);
        assert!(!config.hooks.fail_on_warnings);
    }
}
