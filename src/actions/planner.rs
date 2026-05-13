//! Action planner - Creates action plans based on audit results
//!
//! This module provides functionality to generate action plans from audit results.
//! It analyzes findings and creates appropriate actions to fix issues.

use std::collections::HashMap;

use crate::config::Config;
use crate::providers::github::GitHubProvider;
use crate::rules::results::AuditResults;

use super::plan::{
    Action, ActionOperation, ActionPlan, BranchProtectionSettings, GitHubRepoSettings,
};

/// Parameters for planning file creation
struct FileCreationParams<'a> {
    rule_id: &'a str,
    file_path: &'a str,
    template: &'a str,
    action_id: &'a str,
    action_description: &'a str,
    detail: Option<&'a str>,
}

/// Creates action plans based on audit results and configuration
///
/// The `ActionPlanner` analyzes audit findings and generates a plan of actions
/// to fix detected issues. Actions can include:
/// - Creating missing files (LICENSE, CONTRIBUTING.md, etc.)
/// - Updating .gitignore
/// - Configuring branch protection
/// - Updating GitHub repository settings
pub struct ActionPlanner {
    /// Configuration for action planning
    config: Config,
}

impl ActionPlanner {
    /// Create a new action planner with the given configuration
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration that determines which actions to plan
    ///
    /// # Returns
    ///
    /// A new `ActionPlanner` instance
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Create an action plan based on audit results
    ///
    /// Analyzes the audit results and generates actions to fix detected issues.
    /// Only actions enabled in the configuration will be included.
    ///
    /// # Arguments
    ///
    /// * `results` - The audit results to analyze
    ///
    /// # Returns
    ///
    /// An `ActionPlan` containing all planned actions
    pub async fn create_plan(
        &self,
        results: &AuditResults,
    ) -> Result<ActionPlan, crate::error::RepoLensError> {
        let mut plan = ActionPlan::new();

        // Plan gitignore updates
        if self.config.actions.gitignore {
            if let Some(action) = self.plan_gitignore_update(results) {
                plan.add(action);
            }
        }

        // Plan license creation
        if self.config.actions.license.enabled {
            if let Some(action) = self.plan_license_creation(results) {
                plan.add(action);
            }
        }

        // Plan CONTRIBUTING creation
        if self.config.actions.contributing {
            if let Some(action) = self.plan_contributing_creation(results) {
                plan.add(action);
            }
        }

        // Plan CODE_OF_CONDUCT creation
        if self.config.actions.code_of_conduct {
            if let Some(action) = self.plan_code_of_conduct_creation(results) {
                plan.add(action);
            }
        }

        // Plan SECURITY.md creation
        if self.config.actions.security_policy {
            if let Some(action) = self.plan_security_creation(results) {
                plan.add(action);
            }
        }

        // Plan branch protection (only if not already configured)
        if self.config.actions.branch_protection.enabled {
            if let Some(action) = self.plan_branch_protection_if_needed().await? {
                plan.add(action);
            }
        }

        // Plan GitHub settings (only if not already configured)
        if let Some(action) = self.plan_github_settings_if_needed().await? {
            plan.add(action);
        }

        Ok(plan)
    }

    /// Plan .gitignore updates based on findings
    ///
    /// Collects entries that should be added to .gitignore from audit findings.
    /// The findings already contain language-specific recommendations based on
    /// detected languages in the repository.
    ///
    /// # Arguments
    ///
    /// * `results` - The audit results
    ///
    /// # Returns
    ///
    /// An `Action` to update .gitignore, or `None` if no updates are needed
    fn plan_gitignore_update(&self, results: &AuditResults) -> Option<Action> {
        // Collect entries that should be added to .gitignore from findings
        // These findings are already language-aware thanks to check_gitignore
        let mut entries = Vec::new();

        // Extract patterns from FILE003 findings
        for finding in results.findings_by_category("files") {
            if finding.rule_id == "FILE003" {
                // Extract the pattern from the message
                // Format: ".gitignore missing recommended entry: <pattern>"
                if let Some(pattern) = finding.message.split("entry: ").nth(1) {
                    entries.push(pattern.trim().to_string());
                }
            }
        }

        if entries.is_empty() {
            return None;
        }

        Some(
            Action::new(
                "gitignore-update",
                "gitignore",
                "Add entries to .gitignore",
                ActionOperation::UpdateGitignore {
                    entries: entries.clone(),
                },
            )
            .with_details(entries),
        )
    }

    /// Plan LICENSE file creation
    ///
    /// Creates a LICENSE file if one is missing and license creation is enabled.
    ///
    /// # Arguments
    ///
    /// * `results` - The audit results
    ///
    /// # Returns
    ///
    /// An `Action` to create LICENSE, or `None` if not needed
    fn plan_license_creation(&self, results: &AuditResults) -> Option<Action> {
        // Check if LICENSE is missing
        let needs_license = results
            .findings_by_category("docs")
            .any(|f| f.rule_id == "DOC004");

        if !needs_license {
            return None;
        }

        let license_type = &self.config.actions.license.license_type;
        let mut variables = HashMap::new();

        if let Some(author) = &self.config.actions.license.author {
            variables.insert("author".to_string(), author.clone());
        }

        let year = self
            .config
            .actions
            .license
            .year
            .clone()
            .unwrap_or_else(|| chrono::Utc::now().format("%Y").to_string());
        variables.insert("year".to_string(), year);

        Some(
            Action::new(
                "license-create",
                "file",
                "Create LICENSE file",
                ActionOperation::CreateFile {
                    path: "LICENSE".to_string(),
                    template: format!("LICENSE/{}", license_type),
                    variables,
                },
            )
            .with_detail(format!("License type: {}", license_type)),
        )
    }

    /// Generic helper to plan file creation from template
    ///
    /// # Arguments
    ///
    /// * `results` - The audit results
    /// * `params` - Parameters for file creation
    ///
    /// # Returns
    ///
    /// An `Action` if the file needs to be created, `None` otherwise
    fn plan_file_creation(
        &self,
        results: &AuditResults,
        params: FileCreationParams<'_>,
    ) -> Option<Action> {
        let needs_file = results
            .findings_by_category("docs")
            .any(|f| f.rule_id == params.rule_id);

        if !needs_file {
            return None;
        }

        let mut action = Action::new(
            params.action_id,
            "file",
            params.action_description,
            ActionOperation::CreateFile {
                path: params.file_path.to_string(),
                template: params.template.to_string(),
                variables: HashMap::new(),
            },
        );

        if let Some(detail) = params.detail {
            action = action.with_detail(detail);
        }

        Some(action)
    }

    fn plan_contributing_creation(&self, results: &AuditResults) -> Option<Action> {
        self.plan_file_creation(
            results,
            FileCreationParams {
                rule_id: "DOC005",
                file_path: "CONTRIBUTING.md",
                template: "CONTRIBUTING.md",
                action_id: "contributing-create",
                action_description: "Create CONTRIBUTING.md",
                detail: None,
            },
        )
    }

    fn plan_code_of_conduct_creation(&self, results: &AuditResults) -> Option<Action> {
        self.plan_file_creation(
            results,
            FileCreationParams {
                rule_id: "DOC006",
                file_path: "CODE_OF_CONDUCT.md",
                template: "CODE_OF_CONDUCT.md",
                action_id: "coc-create",
                action_description: "Create CODE_OF_CONDUCT.md",
                detail: Some("Using Contributor Covenant template"),
            },
        )
    }

    fn plan_security_creation(&self, results: &AuditResults) -> Option<Action> {
        self.plan_file_creation(
            results,
            FileCreationParams {
                rule_id: "DOC007",
                file_path: "SECURITY.md",
                template: "SECURITY.md",
                action_id: "security-create",
                action_description: "Create SECURITY.md",
                detail: None,
            },
        )
    }

    /// Plan branch protection configuration if needed
    ///
    /// Checks the current branch protection status and creates an action only if
    /// the current settings don't match the desired configuration.
    ///
    /// # Returns
    ///
    /// An `Action` to configure branch protection, or `None` if already configured correctly
    async fn plan_branch_protection_if_needed(
        &self,
    ) -> Result<Option<Action>, crate::error::RepoLensError> {
        let bp = &self.config.actions.branch_protection;

        // Try to get current branch protection status
        let provider = match GitHubProvider::new() {
            Ok(p) => p,
            Err(_) => {
                // If GitHub CLI is not available, still plan the action
                // (it will fail gracefully during apply)
                return Ok(Some(self.create_branch_protection_action()));
            }
        };

        let current_protection = match provider.get_branch_protection(&bp.branch) {
            Ok(Some(protection)) => protection,
            Ok(None) => {
                // No protection exists, plan the action
                return Ok(Some(self.create_branch_protection_action()));
            }
            Err(_) => {
                // Error fetching protection, plan the action to be safe
                return Ok(Some(self.create_branch_protection_action()));
            }
        };

        // Check if current protection matches desired settings
        let needs_update = {
            // Check required approvals
            let current_approvals = current_protection
                .required_pull_request_reviews
                .as_ref()
                .map(|r| r.required_approving_review_count)
                .unwrap_or(0);
            let needs_approvals = current_approvals != bp.required_approvals;

            // Check status checks
            let has_status_checks = current_protection.required_status_checks.is_some();
            let needs_status_checks = has_status_checks != bp.require_status_checks;

            // Check force push blocking
            // If block_force_push is true, we need allow_force_pushes.enabled to be false
            // If block_force_push is false, we need allow_force_pushes.enabled to be true
            let allows_force_push = current_protection
                .allow_force_pushes
                .as_ref()
                .map(|a| a.enabled)
                .unwrap_or(true);
            // We need to update if: (block_force_push && allows_force_push) || (!block_force_push && !allows_force_push)
            // Which simplifies to: allows_force_push == block_force_push
            let needs_force_push_block = allows_force_push == bp.block_force_push;

            needs_approvals || needs_status_checks || needs_force_push_block
        };

        if needs_update {
            Ok(Some(self.create_branch_protection_action()))
        } else {
            Ok(None)
        }
    }

    /// Create a branch protection action
    fn create_branch_protection_action(&self) -> Action {
        let bp = &self.config.actions.branch_protection;

        let settings = BranchProtectionSettings {
            required_approvals: bp.required_approvals,
            require_status_checks: bp.require_status_checks,
            require_conversation_resolution: true,
            require_linear_history: true,
            block_force_push: bp.block_force_push,
            block_deletions: true,
            enforce_admins: true,
            require_signed_commits: bp.require_signed_commits,
        };

        let mut details = vec![
            format!("Require PR reviews: {}", bp.required_approvals),
            format!("Require status checks: {}", bp.require_status_checks),
            format!("Block force push: {}", bp.block_force_push),
        ];

        if bp.require_signed_commits {
            details.push("Require signed commits".to_string());
        }

        Action::new(
            "branch-protection",
            "github",
            format!("Enable branch protection on '{}'", bp.branch),
            ActionOperation::ConfigureBranchProtection {
                branch: bp.branch.clone(),
                settings,
            },
        )
        .with_details(details)
    }

    /// Plan GitHub repository settings updates if needed
    ///
    /// Checks the current repository settings and creates an action only if
    /// the current settings don't match the desired configuration.
    ///
    /// # Returns
    ///
    /// An `Action` to update GitHub settings, or `None` if already configured correctly
    async fn plan_github_settings_if_needed(
        &self,
    ) -> Result<Option<Action>, crate::error::RepoLensError> {
        let gs = &self.config.actions.github_settings;

        // Try to get current repository settings
        let provider = match GitHubProvider::new() {
            Ok(p) => p,
            Err(_) => {
                // If GitHub CLI is not available, still plan the action
                // (it will fail gracefully during apply)
                return Ok(Some(self.create_github_settings_action()));
            }
        };

        let current_settings = match provider.get_repo_settings() {
            Ok(settings) => settings,
            Err(_) => {
                // Error fetching settings, plan the action to be safe
                return Ok(Some(self.create_github_settings_action()));
            }
        };

        // Check vulnerability alerts
        let current_vuln_alerts = match provider.has_vulnerability_alerts() {
            Ok(enabled) => enabled,
            Err(e) => {
                tracing::debug!("Could not check vulnerability alerts status: {:?}", e);
                // If we can't check, assume it needs to be set (safer)
                true
            }
        };
        let needs_vuln_alerts = current_vuln_alerts != gs.vulnerability_alerts;

        // Check automated security fixes
        let current_auto_fixes = match provider.has_automated_security_fixes() {
            Ok(enabled) => enabled,
            Err(e) => {
                tracing::debug!("Could not check automated security fixes status: {:?}", e);
                // If we can't check, assume it needs to be set (safer)
                true
            }
        };
        let needs_auto_fixes = current_auto_fixes != gs.automated_security_fixes;

        // Check discussions
        let needs_discussions = current_settings.has_discussions_enabled != gs.discussions;

        // Check issues (if configured)
        let needs_issues = current_settings.has_issues_enabled != gs.issues;

        // Check wiki (if configured)
        let needs_wiki = current_settings.has_wiki_enabled != gs.wiki;

        tracing::debug!(
            "GitHub settings check: discussions={} (current={}, desired={}), vuln_alerts={} (current={}, desired={}), auto_fixes={} (current={}, desired={})",
            needs_discussions,
            current_settings.has_discussions_enabled,
            gs.discussions,
            needs_vuln_alerts,
            current_vuln_alerts,
            gs.vulnerability_alerts,
            needs_auto_fixes,
            current_auto_fixes,
            gs.automated_security_fixes
        );

        // Only create action if something needs to be changed
        if needs_discussions || needs_issues || needs_wiki || needs_vuln_alerts || needs_auto_fixes
        {
            Ok(Some(self.create_github_settings_action_filtered(
                needs_discussions,
                needs_issues,
                needs_wiki,
                needs_vuln_alerts,
                needs_auto_fixes,
            )))
        } else {
            Ok(None)
        }
    }

    /// Create a GitHub settings action with all settings
    fn create_github_settings_action(&self) -> Action {
        let gs = &self.config.actions.github_settings;

        let settings = GitHubRepoSettings {
            enable_discussions: Some(gs.discussions),
            enable_issues: Some(gs.issues),
            enable_wiki: Some(gs.wiki),
            enable_vulnerability_alerts: Some(gs.vulnerability_alerts),
            enable_automated_security_fixes: Some(gs.automated_security_fixes),
        };

        let mut details = Vec::new();

        if gs.discussions {
            details.push("Enable discussions".to_string());
        }
        if gs.vulnerability_alerts {
            details.push("Enable vulnerability alerts".to_string());
        }
        if gs.automated_security_fixes {
            details.push("Enable automated security fixes".to_string());
        }

        Action::new(
            "github-settings",
            "github",
            "Update repository settings",
            ActionOperation::UpdateGitHubSettings { settings },
        )
        .with_details(details)
    }

    /// Create a GitHub settings action with only settings that need to be changed
    fn create_github_settings_action_filtered(
        &self,
        needs_discussions: bool,
        needs_issues: bool,
        needs_wiki: bool,
        needs_vuln_alerts: bool,
        needs_auto_fixes: bool,
    ) -> Action {
        let gs = &self.config.actions.github_settings;

        let settings = GitHubRepoSettings {
            enable_discussions: if needs_discussions {
                Some(gs.discussions)
            } else {
                None
            },
            enable_issues: if needs_issues { Some(gs.issues) } else { None },
            enable_wiki: if needs_wiki { Some(gs.wiki) } else { None },
            enable_vulnerability_alerts: if needs_vuln_alerts {
                Some(gs.vulnerability_alerts)
            } else {
                None
            },
            enable_automated_security_fixes: if needs_auto_fixes {
                Some(gs.automated_security_fixes)
            } else {
                None
            },
        };

        let mut details = Vec::new();

        if needs_discussions && gs.discussions {
            details.push("Enable discussions".to_string());
        }
        if needs_vuln_alerts && gs.vulnerability_alerts {
            details.push("Enable vulnerability alerts".to_string());
        }
        if needs_auto_fixes && gs.automated_security_fixes {
            details.push("Enable automated security fixes".to_string());
        }

        Action::new(
            "github-settings",
            "github",
            "Update repository settings",
            ActionOperation::UpdateGitHubSettings { settings },
        )
        .with_details(details)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::rules::results::{AuditResults, Finding, Severity};

    #[tokio::test]
    async fn test_create_plan_includes_gitignore() {
        let config = Config::default();
        let planner = ActionPlanner::new(config);

        let mut results = AuditResults::new("test-repo", "opensource");
        results.add_finding(Finding::new(
            "FILE003",
            "files",
            Severity::Info,
            ".gitignore missing recommended entry: .env",
        ));

        let plan = planner.create_plan(&results).await.unwrap();

        assert!(!plan.is_empty());
        assert!(plan.actions().iter().any(|a| a.id() == "gitignore-update"));
    }

    #[tokio::test]
    async fn test_create_plan_includes_license() {
        let config = Config::default();
        let planner = ActionPlanner::new(config);

        let mut results = AuditResults::new("test-repo", "opensource");
        results.add_finding(Finding::new(
            "DOC004",
            "docs",
            Severity::Critical,
            "LICENSE file is missing",
        ));

        let plan = planner.create_plan(&results).await.unwrap();

        assert!(plan.actions().iter().any(|a| a.id() == "license-create"));
    }

    #[tokio::test]
    async fn test_create_plan_includes_contributing() {
        let config = Config::default();
        let planner = ActionPlanner::new(config);

        let mut results = AuditResults::new("test-repo", "opensource");
        results.add_finding(Finding::new(
            "DOC005",
            "docs",
            Severity::Warning,
            "CONTRIBUTING file is missing",
        ));

        let plan = planner.create_plan(&results).await.unwrap();

        assert!(
            plan.actions()
                .iter()
                .any(|a| a.id() == "contributing-create")
        );
    }

    #[tokio::test]
    async fn test_create_plan_filters_by_config() {
        let mut config = Config::default();
        config.actions.contributing = false;

        let planner = ActionPlanner::new(config);

        let mut results = AuditResults::new("test-repo", "opensource");
        results.add_finding(Finding::new(
            "DOC005",
            "docs",
            Severity::Warning,
            "CONTRIBUTING file is missing",
        ));

        let plan = planner.create_plan(&results).await.unwrap();

        // Should not include contributing because it's disabled in config
        assert!(
            !plan
                .actions()
                .iter()
                .any(|a| a.id() == "contributing-create")
        );
    }

    #[tokio::test]
    async fn test_create_plan_includes_code_of_conduct() {
        let config = Config::default();
        let planner = ActionPlanner::new(config);

        let mut results = AuditResults::new("test-repo", "opensource");
        results.add_finding(Finding::new(
            "DOC006",
            "docs",
            Severity::Warning,
            "CODE_OF_CONDUCT file is missing",
        ));

        let plan = planner.create_plan(&results).await.unwrap();

        assert!(plan.actions().iter().any(|a| a.id() == "coc-create"));
    }

    #[tokio::test]
    async fn test_create_plan_includes_security_policy() {
        let config = Config::default();
        let planner = ActionPlanner::new(config);

        let mut results = AuditResults::new("test-repo", "opensource");
        results.add_finding(Finding::new(
            "DOC007",
            "docs",
            Severity::Warning,
            "SECURITY.md is missing",
        ));

        let plan = planner.create_plan(&results).await.unwrap();

        assert!(plan.actions().iter().any(|a| a.id() == "security-create"));
    }

    #[tokio::test]
    async fn test_create_plan_includes_branch_protection() {
        let config = Config::default();
        let planner = ActionPlanner::new(config);
        let results = AuditResults::new("test-repo", "opensource");

        let plan = planner.create_plan(&results).await.unwrap();

        assert!(plan.actions().iter().any(|a| a.id() == "branch-protection"));
    }

    #[tokio::test]
    async fn test_create_plan_includes_github_settings() {
        let config = Config::default();
        let planner = ActionPlanner::new(config);
        let results = AuditResults::new("test-repo", "opensource");

        let plan = planner.create_plan(&results).await.unwrap();

        assert!(plan.actions().iter().any(|a| a.id() == "github-settings"));
    }

    #[tokio::test]
    async fn test_create_plan_no_gitignore_needed() {
        let config = Config::default();
        let planner = ActionPlanner::new(config);
        let results = AuditResults::new("test-repo", "opensource");

        let plan = planner.create_plan(&results).await.unwrap();

        // No FILE003 findings, so no gitignore update
        assert!(!plan.actions().iter().any(|a| a.id() == "gitignore-update"));
    }

    #[tokio::test]
    async fn test_create_plan_license_with_author_and_year() {
        let mut config = Config::default();
        config.actions.license.author = Some("Test Author".to_string());
        config.actions.license.year = Some("2024".to_string());

        let planner = ActionPlanner::new(config);

        let mut results = AuditResults::new("test-repo", "opensource");
        results.add_finding(Finding::new(
            "DOC004",
            "docs",
            Severity::Critical,
            "LICENSE file is missing",
        ));

        let plan = planner.create_plan(&results).await.unwrap();

        assert!(plan.actions().iter().any(|a| a.id() == "license-create"));
    }

    #[tokio::test]
    async fn test_branch_protection_with_signed_commits() {
        let mut config = Config::default();
        config.actions.branch_protection.require_signed_commits = true;

        let planner = ActionPlanner::new(config);
        let results = AuditResults::new("test-repo", "opensource");

        let plan = planner.create_plan(&results).await.unwrap();

        let bp_action = plan
            .actions()
            .iter()
            .find(|a| a.id() == "branch-protection");
        assert!(bp_action.is_some());
    }

    #[tokio::test]
    async fn test_create_plan_multiple_gitignore_entries() {
        let config = Config::default();
        let planner = ActionPlanner::new(config);

        let mut results = AuditResults::new("test-repo", "opensource");
        results.add_finding(Finding::new(
            "FILE003",
            "files",
            Severity::Info,
            ".gitignore missing recommended entry: .env",
        ));
        results.add_finding(Finding::new(
            "FILE003",
            "files",
            Severity::Info,
            ".gitignore missing recommended entry: *.log",
        ));
        results.add_finding(Finding::new(
            "FILE003",
            "files",
            Severity::Info,
            ".gitignore missing recommended entry: node_modules/",
        ));

        let plan = planner.create_plan(&results).await.unwrap();

        let gitignore_action = plan
            .actions()
            .iter()
            .find(|a| a.id() == "gitignore-update")
            .expect("Should have gitignore action");

        // Should collect all entries
        match gitignore_action.operation() {
            ActionOperation::UpdateGitignore { entries } => {
                assert_eq!(entries.len(), 3);
                assert!(entries.contains(&".env".to_string()));
                assert!(entries.contains(&"*.log".to_string()));
                assert!(entries.contains(&"node_modules/".to_string()));
            }
            _ => panic!("Expected UpdateGitignore operation"),
        }
    }

    #[tokio::test]
    async fn test_plan_license_uses_default_year() {
        let mut config = Config::default();
        config.actions.license.year = None; // No year specified, should use current year

        let planner = ActionPlanner::new(config);

        let mut results = AuditResults::new("test-repo", "opensource");
        results.add_finding(Finding::new(
            "DOC004",
            "docs",
            Severity::Critical,
            "LICENSE file is missing",
        ));

        let plan = planner.create_plan(&results).await.unwrap();

        let license_action = plan
            .actions()
            .iter()
            .find(|a| a.id() == "license-create")
            .expect("Should have license action");

        match license_action.operation() {
            ActionOperation::CreateFile { variables, .. } => {
                assert!(variables.contains_key("year"));
                // Year should be the current year (4 digits)
                assert_eq!(variables.get("year").unwrap().len(), 4);
            }
            _ => panic!("Expected CreateFile operation"),
        }
    }

    #[tokio::test]
    async fn test_plan_all_docs_disabled() {
        let mut config = Config::default();
        config.actions.contributing = false;
        config.actions.code_of_conduct = false;
        config.actions.security_policy = false;
        config.actions.license.enabled = false;
        config.actions.gitignore = false;
        config.actions.branch_protection.enabled = false;

        let planner = ActionPlanner::new(config);

        let mut results = AuditResults::new("test-repo", "opensource");
        results.add_finding(Finding::new(
            "DOC004",
            "docs",
            Severity::Critical,
            "LICENSE file is missing",
        ));
        results.add_finding(Finding::new(
            "DOC005",
            "docs",
            Severity::Warning,
            "CONTRIBUTING file is missing",
        ));
        results.add_finding(Finding::new(
            "DOC006",
            "docs",
            Severity::Warning,
            "CODE_OF_CONDUCT file is missing",
        ));
        results.add_finding(Finding::new(
            "DOC007",
            "docs",
            Severity::Warning,
            "SECURITY.md is missing",
        ));
        results.add_finding(Finding::new(
            "FILE003",
            "files",
            Severity::Info,
            ".gitignore missing recommended entry: .env",
        ));

        let plan = planner.create_plan(&results).await.unwrap();

        // Only github-settings should be present (always planned)
        assert!(!plan.actions().iter().any(|a| a.id() == "license-create"));
        assert!(
            !plan
                .actions()
                .iter()
                .any(|a| a.id() == "contributing-create")
        );
        assert!(!plan.actions().iter().any(|a| a.id() == "coc-create"));
        assert!(!plan.actions().iter().any(|a| a.id() == "security-create"));
        assert!(!plan.actions().iter().any(|a| a.id() == "gitignore-update"));
        assert!(!plan.actions().iter().any(|a| a.id() == "branch-protection"));
    }

    #[test]
    fn test_action_planner_new() {
        let config = Config::default();
        let planner = ActionPlanner::new(config.clone());
        // Verify planner was created (indirect test via create_plan)
        assert!(planner.config.actions.gitignore);
    }

    #[tokio::test]
    async fn test_create_branch_protection_action_directly() {
        let mut config = Config::default();
        config.actions.branch_protection.branch = "develop".to_string();
        config.actions.branch_protection.required_approvals = 2;
        config.actions.branch_protection.require_status_checks = false;
        config.actions.branch_protection.block_force_push = true;

        let planner = ActionPlanner::new(config);
        let action = planner.create_branch_protection_action();

        assert_eq!(action.id(), "branch-protection");
        assert!(action.description().contains("develop"));

        match action.operation() {
            ActionOperation::ConfigureBranchProtection { branch, settings } => {
                assert_eq!(branch, "develop");
                assert_eq!(settings.required_approvals, 2);
                assert!(!settings.require_status_checks);
                assert!(settings.block_force_push);
            }
            _ => panic!("Expected ConfigureBranchProtection operation"),
        }
    }

    #[test]
    fn test_create_github_settings_action_directly() {
        let mut config = Config::default();
        config.actions.github_settings.discussions = true;
        config.actions.github_settings.vulnerability_alerts = true;
        config.actions.github_settings.automated_security_fixes = true;

        let planner = ActionPlanner::new(config);
        let action = planner.create_github_settings_action();

        assert_eq!(action.id(), "github-settings");

        match action.operation() {
            ActionOperation::UpdateGitHubSettings { settings } => {
                assert_eq!(settings.enable_discussions, Some(true));
                assert_eq!(settings.enable_vulnerability_alerts, Some(true));
                assert_eq!(settings.enable_automated_security_fixes, Some(true));
            }
            _ => panic!("Expected UpdateGitHubSettings operation"),
        }
    }

    #[test]
    fn test_create_github_settings_action_filtered() {
        let config = Config::default();
        let planner = ActionPlanner::new(config);

        // Only discussions needs update
        let action = planner.create_github_settings_action_filtered(
            true,  // needs_discussions
            false, // needs_issues
            false, // needs_wiki
            false, // needs_vuln_alerts
            false, // needs_auto_fixes
        );

        match action.operation() {
            ActionOperation::UpdateGitHubSettings { settings } => {
                assert!(settings.enable_discussions.is_some());
                assert!(settings.enable_issues.is_none());
                assert!(settings.enable_wiki.is_none());
                assert!(settings.enable_vulnerability_alerts.is_none());
                assert!(settings.enable_automated_security_fixes.is_none());
            }
            _ => panic!("Expected UpdateGitHubSettings operation"),
        }
    }
}
