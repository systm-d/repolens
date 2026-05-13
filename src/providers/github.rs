//! GitHub provider - Interactions with GitHub API via octocrab (with gh CLI fallback)
//!
//! This module provides GitHub API access using octocrab for direct API calls,
//! with automatic fallback to gh CLI when octocrab is unavailable.
//!
//! ## Authentication
//!
//! The provider supports two authentication methods (in order of preference):
//! 1. `GITHUB_TOKEN` environment variable - Used by octocrab
//! 2. `gh auth login` - Fallback via gh CLI

use crate::error::{ProviderError, RepoLensError};
use octocrab::Octocrab;
use serde::Deserialize;
use std::env;
use std::future::Future;
use std::process::Command;
use tokio::runtime::Runtime;

/// GitHub provider for repository operations
///
/// Provides access to GitHub API for:
/// - Repository information
/// - Branch protection settings
/// - Security features (vulnerability alerts, automated fixes)
/// - Issue and PR creation
pub struct GitHubProvider {
    repo_owner: String,
    repo_name: String,
    octocrab: Option<Octocrab>,
}

#[derive(Debug, Deserialize)]
pub struct RepoInfo {
    #[allow(dead_code)]
    name: String,
    #[allow(dead_code)]
    owner: RepoOwner,
    #[serde(rename = "hasIssuesEnabled")]
    pub has_issues_enabled: bool,
    #[serde(rename = "hasDiscussionsEnabled")]
    pub has_discussions_enabled: bool,
    #[serde(rename = "hasWikiEnabled")]
    pub has_wiki_enabled: bool,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct RepoOwner {
    login: String,
}

impl GitHubProvider {
    /// Create a new GitHub provider for the current repository
    ///
    /// Attempts to authenticate via GITHUB_TOKEN first, then falls back to gh CLI.
    pub fn new() -> Result<Self, RepoLensError> {
        let (owner, name) = Self::get_repo_info()?;

        // Try to create octocrab instance with GITHUB_TOKEN
        let octocrab = Self::create_octocrab_client();

        Ok(Self {
            repo_owner: owner,
            repo_name: name,
            octocrab,
        })
    }

    /// Create an octocrab client if GITHUB_TOKEN is available
    fn create_octocrab_client() -> Option<Octocrab> {
        env::var("GITHUB_TOKEN")
            .ok()
            .and_then(|token| Octocrab::builder().personal_token(token).build().ok())
    }

    /// Check if GitHub API is available (via token or gh CLI)
    pub fn is_available() -> bool {
        // Check GITHUB_TOKEN first
        if env::var("GITHUB_TOKEN").is_ok() {
            return true;
        }

        // Fall back to gh CLI check
        Command::new("gh")
            .args(["auth", "status"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Check if GITHUB_TOKEN is set
    #[allow(dead_code)]
    pub fn has_token() -> bool {
        env::var("GITHUB_TOKEN").is_ok()
    }

    /// Get repository owner and name from git remote
    fn get_repo_info() -> Result<(String, String), RepoLensError> {
        // First try parsing git remote URL directly
        if let Ok((owner, name)) = Self::get_repo_from_git_remote() {
            return Ok((owner, name));
        }

        // Fall back to gh CLI
        let output = Command::new("gh")
            .args([
                "repo",
                "view",
                "--json",
                "owner,name",
                "-q",
                ".owner.login + \"/\" + .name",
            ])
            .output()
            .map_err(|_| {
                RepoLensError::Provider(ProviderError::CommandFailed {
                    command: "gh repo view".to_string(),
                })
            })?;

        if !output.status.success() {
            return Err(RepoLensError::Provider(ProviderError::NotAuthenticated));
        }

        let full_name = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Self::parse_repo_name(&full_name)
    }

    /// Parse owner/name from git remote URL
    fn get_repo_from_git_remote() -> Result<(String, String), RepoLensError> {
        let output = Command::new("git")
            .args(["remote", "get-url", "origin"])
            .output()
            .map_err(|_| {
                RepoLensError::Provider(ProviderError::CommandFailed {
                    command: "git remote get-url origin".to_string(),
                })
            })?;

        if !output.status.success() {
            return Err(RepoLensError::Provider(ProviderError::CommandFailed {
                command: "git remote get-url origin".to_string(),
            }));
        }

        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Self::parse_github_url(&url)
    }

    /// Parse GitHub URL to extract owner and repo name
    fn parse_github_url(url: &str) -> Result<(String, String), RepoLensError> {
        // Handle SSH URLs: git@github.com:owner/repo.git
        if url.starts_with("git@github.com:") {
            let path = url.trim_start_matches("git@github.com:");
            let path = path.trim_end_matches(".git");
            return Self::parse_repo_name(path);
        }

        // Handle HTTPS URLs: https://github.com/owner/repo.git
        if url.contains("github.com") {
            let path = url
                .split("github.com/")
                .nth(1)
                .ok_or_else(|| {
                    RepoLensError::Provider(ProviderError::InvalidRepoName {
                        name: url.to_string(),
                    })
                })?
                .trim_end_matches(".git");
            return Self::parse_repo_name(path);
        }

        Err(RepoLensError::Provider(ProviderError::InvalidRepoName {
            name: url.to_string(),
        }))
    }

    /// Parse "owner/name" format
    fn parse_repo_name(full_name: &str) -> Result<(String, String), RepoLensError> {
        let parts: Vec<&str> = full_name.split('/').collect();
        if parts.len() != 2 {
            return Err(RepoLensError::Provider(ProviderError::InvalidRepoName {
                name: full_name.to_string(),
            }));
        }
        Ok((parts[0].to_string(), parts[1].to_string()))
    }

    /// Get the repository owner
    #[allow(dead_code)]
    pub fn owner(&self) -> &str {
        &self.repo_owner
    }

    /// Get the repository name
    #[allow(dead_code)]
    pub fn name(&self) -> &str {
        &self.repo_name
    }

    /// Get the full repository name (owner/name)
    pub fn full_name(&self) -> String {
        format!("{}/{}", self.repo_owner, self.repo_name)
    }

    /// Get a reference to the octocrab instance (if available)
    #[allow(dead_code)]
    pub fn octocrab(&self) -> Option<&Octocrab> {
        self.octocrab.as_ref()
    }

    /// Run an async future in a blocking fashion
    /// This is used to bridge async octocrab calls with synchronous code
    fn block_on<F: Future>(future: F) -> F::Output {
        Runtime::new()
            .expect("Failed to create tokio runtime")
            .block_on(future)
    }

    /// Get branch protection status
    pub fn get_branch_protection(
        &self,
        branch: &str,
    ) -> Result<Option<BranchProtection>, RepoLensError> {
        // Use gh CLI for branch protection (octocrab requires more complex setup)
        let output = Command::new("gh")
            .args([
                "api",
                &format!("repos/{}/branches/{}/protection", self.full_name(), branch),
            ])
            .output()
            .map_err(|_| {
                RepoLensError::Provider(ProviderError::CommandFailed {
                    command: format!(
                        "gh api repos/{}/branches/{}/protection",
                        self.full_name(),
                        branch
                    ),
                })
            })?;

        if !output.status.success() {
            // 404 means no protection
            return Ok(None);
        }

        let protection: BranchProtection = serde_json::from_slice(&output.stdout)?;
        Ok(Some(protection))
    }

    /// Get repository settings (discussions, issues, wiki, etc.)
    ///
    /// Uses octocrab when GITHUB_TOKEN is available, falls back to gh CLI otherwise.
    pub fn get_repo_settings(&self) -> Result<RepoInfo, RepoLensError> {
        // Try octocrab first if available
        if let Some(octo) = &self.octocrab {
            let owner = self.repo_owner.clone();
            let name = self.repo_name.clone();
            let octo = octo.clone();

            let result = Self::block_on(async move { octo.repos(&owner, &name).get().await });

            if let Ok(repo) = result {
                return Ok(RepoInfo {
                    name: repo.name.clone(),
                    owner: RepoOwner {
                        login: repo
                            .owner
                            .as_ref()
                            .map(|o| o.login.clone())
                            .unwrap_or_default(),
                    },
                    has_issues_enabled: repo.has_issues.unwrap_or(false),
                    // has_discussions not available in octocrab Repository model
                    // This is a limitation - gh CLI provides more complete data
                    has_discussions_enabled: false,
                    has_wiki_enabled: repo.has_wiki.unwrap_or(false),
                });
            }
            // Fall through to gh CLI if octocrab fails
        }

        // Fall back to gh CLI
        let output = Command::new("gh")
            .args([
                "repo",
                "view",
                "--json",
                "name,owner,hasIssuesEnabled,hasDiscussionsEnabled,hasWikiEnabled",
            ])
            .output()
            .map_err(|_| {
                RepoLensError::Provider(ProviderError::CommandFailed {
                    command: "gh repo view".to_string(),
                })
            })?;

        if !output.status.success() {
            return Err(RepoLensError::Provider(ProviderError::CommandFailed {
                command: "gh repo view".to_string(),
            }));
        }

        let repo_info: RepoInfo = serde_json::from_slice(&output.stdout)?;
        Ok(repo_info)
    }

    /// Check if vulnerability alerts are enabled
    pub fn has_vulnerability_alerts(&self) -> Result<bool, RepoLensError> {
        let output = Command::new("gh")
            .args([
                "api",
                &format!("repos/{}/vulnerability-alerts", self.full_name()),
            ])
            .output()
            .map_err(|_| {
                RepoLensError::Provider(ProviderError::CommandFailed {
                    command: format!("gh api repos/{}/vulnerability-alerts", self.full_name()),
                })
            })?;

        // gh api returns exit code 0 for HTTP 2xx (enabled), non-zero for HTTP 4xx (disabled)
        Ok(output.status.success())
    }

    /// Check if automated security fixes are enabled
    pub fn has_automated_security_fixes(&self) -> Result<bool, RepoLensError> {
        let output = Command::new("gh")
            .args([
                "api",
                &format!("repos/{}/automated-security-fixes", self.full_name()),
            ])
            .output()
            .map_err(|_| {
                RepoLensError::Provider(ProviderError::CommandFailed {
                    command: format!("gh api repos/{}/automated-security-fixes", self.full_name()),
                })
            })?;

        // gh api returns exit code 0 for HTTP 2xx (enabled), non-zero for HTTP 4xx (disabled)
        Ok(output.status.success())
    }

    /// Check if Dependabot security updates are enabled
    pub fn has_dependabot_security_updates(&self) -> Result<bool, RepoLensError> {
        // Dependabot security updates is determined by the automated-security-fixes endpoint
        // which returns enabled/disabled status
        let output = Command::new("gh")
            .args([
                "api",
                &format!("repos/{}/automated-security-fixes", self.full_name()),
            ])
            .output()
            .map_err(|_| {
                RepoLensError::Provider(ProviderError::CommandFailed {
                    command: format!("gh api repos/{}/automated-security-fixes", self.full_name()),
                })
            })?;

        if !output.status.success() {
            return Ok(false);
        }

        // Parse the response to check if enabled
        #[derive(Deserialize)]
        struct AutomatedSecurityFixes {
            enabled: bool,
        }

        let response: Result<AutomatedSecurityFixes, _> = serde_json::from_slice(&output.stdout);
        Ok(response.map(|r| r.enabled).unwrap_or(false))
    }

    /// Get secret scanning settings
    pub fn get_secret_scanning(&self) -> Result<SecretScanningSettings, RepoLensError> {
        // Get code security configuration via the repository endpoint
        let output = Command::new("gh")
            .args([
                "api",
                &format!("repos/{}", self.full_name()),
                "--jq",
                ".security_and_analysis",
            ])
            .output()
            .map_err(|_| {
                RepoLensError::Provider(ProviderError::CommandFailed {
                    command: format!("gh api repos/{}", self.full_name()),
                })
            })?;

        if !output.status.success() {
            // If the API call fails, assume secret scanning is disabled
            return Ok(SecretScanningSettings {
                enabled: false,
                push_protection_enabled: false,
            });
        }

        // Parse the security_and_analysis object
        #[derive(Deserialize)]
        struct SecurityStatus {
            status: String,
        }

        #[derive(Deserialize)]
        struct SecurityAndAnalysis {
            secret_scanning: Option<SecurityStatus>,
            secret_scanning_push_protection: Option<SecurityStatus>,
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let security: Result<SecurityAndAnalysis, _> = serde_json::from_str(&stdout);

        match security {
            Ok(s) => Ok(SecretScanningSettings {
                enabled: s
                    .secret_scanning
                    .map(|ss| ss.status == "enabled")
                    .unwrap_or(false),
                push_protection_enabled: s
                    .secret_scanning_push_protection
                    .map(|ss| ss.status == "enabled")
                    .unwrap_or(false),
            }),
            Err(_) => Ok(SecretScanningSettings {
                enabled: false,
                push_protection_enabled: false,
            }),
        }
    }

    /// Get actions permissions for the repository
    pub fn get_actions_permissions(&self) -> Result<ActionsPermissions, RepoLensError> {
        let output = Command::new("gh")
            .args([
                "api",
                &format!("repos/{}/actions/permissions", self.full_name()),
            ])
            .output()
            .map_err(|_| {
                RepoLensError::Provider(ProviderError::CommandFailed {
                    command: format!("gh api repos/{}/actions/permissions", self.full_name()),
                })
            })?;

        if !output.status.success() {
            // If the API call fails, return default (restrictive) permissions
            return Ok(ActionsPermissions {
                enabled: true,
                allowed_actions: Some("all".to_string()),
                default_workflow_permissions: Some("write".to_string()),
                can_approve_pull_request_reviews: Some(true),
            });
        }

        let permissions: ActionsPermissions = serde_json::from_slice(&output.stdout)?;
        Ok(permissions)
    }

    /// Get actions workflow permissions (default permissions for GITHUB_TOKEN)
    pub fn get_actions_workflow_permissions(&self) -> Result<ActionsPermissions, RepoLensError> {
        let output = Command::new("gh")
            .args([
                "api",
                &format!("repos/{}/actions/permissions/workflow", self.full_name()),
            ])
            .output()
            .map_err(|_| {
                RepoLensError::Provider(ProviderError::CommandFailed {
                    command: format!(
                        "gh api repos/{}/actions/permissions/workflow",
                        self.full_name()
                    ),
                })
            })?;

        if !output.status.success() {
            // If the API call fails, assume permissive defaults
            return Ok(ActionsPermissions {
                enabled: true,
                allowed_actions: None,
                default_workflow_permissions: Some("write".to_string()),
                can_approve_pull_request_reviews: Some(true),
            });
        }

        #[derive(Deserialize)]
        struct WorkflowPermissions {
            default_workflow_permissions: Option<String>,
            can_approve_pull_request_reviews: Option<bool>,
        }

        let perms: WorkflowPermissions = serde_json::from_slice(&output.stdout)?;
        Ok(ActionsPermissions {
            enabled: true,
            allowed_actions: None,
            default_workflow_permissions: perms.default_workflow_permissions,
            can_approve_pull_request_reviews: perms.can_approve_pull_request_reviews,
        })
    }

    /// Check if fork pull request workflows require approval
    pub fn get_fork_pr_workflows_policy(&self) -> Result<bool, RepoLensError> {
        // Check repository settings for requiring approval for fork PRs
        // This is available via the actions/permissions/access endpoint
        let output = Command::new("gh")
            .args([
                "api",
                &format!("repos/{}/actions/permissions/access", self.full_name()),
            ])
            .output()
            .map_err(|_| {
                RepoLensError::Provider(ProviderError::CommandFailed {
                    command: format!(
                        "gh api repos/{}/actions/permissions/access",
                        self.full_name()
                    ),
                })
            })?;

        if !output.status.success() {
            // If the API call fails, assume no approval required
            return Ok(false);
        }

        #[derive(Deserialize)]
        struct AccessLevel {
            access_level: Option<String>,
        }

        let access: Result<AccessLevel, _> = serde_json::from_slice(&output.stdout);
        // access_level "none" means only repo collaborators can run, which requires approval
        // "user" or "organization" means more permissive
        Ok(access
            .map(|a| a.access_level.as_deref() == Some("none"))
            .unwrap_or(false))
    }

    // ===== Access Control Methods =====

    /// List repository collaborators
    pub fn list_collaborators(&self) -> Result<Vec<Collaborator>, RepoLensError> {
        let output = Command::new("gh")
            .args([
                "api",
                &format!("repos/{}/collaborators", self.full_name()),
                "--paginate",
            ])
            .output()
            .map_err(|_| {
                RepoLensError::Provider(ProviderError::CommandFailed {
                    command: format!("gh api repos/{}/collaborators", self.full_name()),
                })
            })?;

        if !output.status.success() {
            // Return empty list if API call fails (may not have permission)
            return Ok(Vec::new());
        }

        let collaborators: Vec<Collaborator> = serde_json::from_slice(&output.stdout)?;
        Ok(collaborators)
    }

    /// List repository teams
    pub fn list_teams(&self) -> Result<Vec<Team>, RepoLensError> {
        let output = Command::new("gh")
            .args([
                "api",
                &format!("repos/{}/teams", self.full_name()),
                "--paginate",
            ])
            .output()
            .map_err(|_| {
                RepoLensError::Provider(ProviderError::CommandFailed {
                    command: format!("gh api repos/{}/teams", self.full_name()),
                })
            })?;

        if !output.status.success() {
            // Return empty list if API call fails (may not have permission)
            return Ok(Vec::new());
        }

        let teams: Vec<Team> = serde_json::from_slice(&output.stdout)?;
        Ok(teams)
    }

    /// List deploy keys
    pub fn list_deploy_keys(&self) -> Result<Vec<DeployKey>, RepoLensError> {
        let output = Command::new("gh")
            .args([
                "api",
                &format!("repos/{}/keys", self.full_name()),
                "--paginate",
            ])
            .output()
            .map_err(|_| {
                RepoLensError::Provider(ProviderError::CommandFailed {
                    command: format!("gh api repos/{}/keys", self.full_name()),
                })
            })?;

        if !output.status.success() {
            // Return empty list if API call fails (may not have permission)
            return Ok(Vec::new());
        }

        let keys: Vec<DeployKey> = serde_json::from_slice(&output.stdout)?;
        Ok(keys)
    }

    /// List GitHub App installations on this repository
    pub fn list_installations(&self) -> Result<Vec<Installation>, RepoLensError> {
        let output = Command::new("gh")
            .args(["api", &format!("repos/{}/installation", self.full_name())])
            .output()
            .map_err(|_| {
                RepoLensError::Provider(ProviderError::CommandFailed {
                    command: format!("gh api repos/{}/installation", self.full_name()),
                })
            })?;

        if !output.status.success() {
            // Return empty list if API call fails (may not have permission)
            return Ok(Vec::new());
        }

        // The installation endpoint returns a single installation, not an array
        let installation: Result<Installation, _> = serde_json::from_slice(&output.stdout);
        match installation {
            Ok(inst) => Ok(vec![inst]),
            Err(_) => Ok(Vec::new()),
        }
    }

    // ===== Infrastructure Methods =====

    /// List repository webhooks
    pub fn list_webhooks(&self) -> Result<Vec<Webhook>, RepoLensError> {
        let output = Command::new("gh")
            .args([
                "api",
                &format!("repos/{}/hooks", self.full_name()),
                "--paginate",
            ])
            .output()
            .map_err(|_| {
                RepoLensError::Provider(ProviderError::CommandFailed {
                    command: format!("gh api repos/{}/hooks", self.full_name()),
                })
            })?;

        if !output.status.success() {
            // Return empty list if API call fails (may not have permission)
            return Ok(Vec::new());
        }

        let webhooks: Vec<Webhook> = serde_json::from_slice(&output.stdout)?;
        Ok(webhooks)
    }

    /// List repository environments
    pub fn list_environments(&self) -> Result<Vec<Environment>, RepoLensError> {
        let output = Command::new("gh")
            .args(["api", &format!("repos/{}/environments", self.full_name())])
            .output()
            .map_err(|_| {
                RepoLensError::Provider(ProviderError::CommandFailed {
                    command: format!("gh api repos/{}/environments", self.full_name()),
                })
            })?;

        if !output.status.success() {
            // Return empty list if API call fails (may not have permission)
            return Ok(Vec::new());
        }

        #[derive(Deserialize)]
        struct EnvironmentsResponse {
            environments: Vec<Environment>,
        }

        let response: Result<EnvironmentsResponse, _> = serde_json::from_slice(&output.stdout);
        match response {
            Ok(r) => Ok(r.environments),
            Err(_) => Ok(Vec::new()),
        }
    }

    /// Get environment protection rules
    pub fn get_environment_protection(
        &self,
        environment_name: &str,
    ) -> Result<EnvironmentProtection, RepoLensError> {
        let output = Command::new("gh")
            .args([
                "api",
                &format!(
                    "repos/{}/environments/{}",
                    self.full_name(),
                    environment_name
                ),
            ])
            .output()
            .map_err(|_| {
                RepoLensError::Provider(ProviderError::CommandFailed {
                    command: format!(
                        "gh api repos/{}/environments/{}",
                        self.full_name(),
                        environment_name
                    ),
                })
            })?;

        if !output.status.success() {
            // Return empty protection if API call fails
            return Ok(EnvironmentProtection::default());
        }

        let protection: EnvironmentProtection =
            serde_json::from_slice(&output.stdout).unwrap_or_default();
        Ok(protection)
    }

    /// Ensure a label exists in the repository, creating it if necessary
    pub fn ensure_label(&self, label: &str, color: &str, description: &str) {
        // Check if label exists by trying to view it
        let check = Command::new("gh")
            .args(["label", "list", "--search", label, "--json", "name"])
            .output();

        if let Ok(output) = check {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains(label) {
                return;
            }
        }

        // Create the label
        let _ = Command::new("gh")
            .args([
                "label",
                "create",
                label,
                "--color",
                color,
                "--description",
                description,
            ])
            .output();
    }

    /// Create a GitHub issue in the repository
    ///
    /// Uses octocrab when GITHUB_TOKEN is available, falls back to gh CLI otherwise.
    ///
    /// # Arguments
    ///
    /// * `title` - The issue title
    /// * `body` - The issue body/description
    /// * `labels` - Labels to add to the issue
    ///
    /// # Returns
    ///
    /// The URL of the created issue
    pub fn create_issue(
        &self,
        title: &str,
        body: &str,
        labels: &[&str],
    ) -> Result<String, RepoLensError> {
        // Ensure all labels exist before creating the issue
        for label in labels {
            self.ensure_label(label, "d73a4a", "Created by RepoLens audit");
        }

        // Try octocrab first if available
        if let Some(octo) = &self.octocrab {
            let owner = self.repo_owner.clone();
            let name = self.repo_name.clone();
            let octo = octo.clone();
            let title = title.to_string();
            let body = body.to_string();
            let labels: Vec<String> = labels.iter().map(|s| s.to_string()).collect();

            let result = Self::block_on(async move {
                octo.issues(&owner, &name)
                    .create(&title)
                    .body(&body)
                    .labels(labels)
                    .send()
                    .await
            });

            if let Ok(issue) = result {
                return Ok(issue.html_url.to_string());
            }
            // Fall through to gh CLI if octocrab fails
        }

        // Fall back to gh CLI
        let mut args = vec!["issue", "create", "--title", title, "--body", body];
        for label in labels {
            args.push("--label");
            args.push(label);
        }

        let output = Command::new("gh").args(&args).output().map_err(|_| {
            RepoLensError::Provider(ProviderError::CommandFailed {
                command: format!("gh {}", args.join(" ")),
            })
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RepoLensError::Provider(ProviderError::CommandFailed {
                command: format!("Failed to create issue: {}", stderr),
            }));
        }

        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(url)
    }

    /// Create a pull request
    ///
    /// # Arguments
    ///
    /// * `title` - The PR title
    /// * `body` - The PR body/description
    /// * `head` - The branch to merge from
    /// * `base` - The base branch to merge into (defaults to default branch)
    ///
    /// # Returns
    ///
    /// The URL of the created pull request
    pub fn create_pull_request(
        &self,
        title: &str,
        body: &str,
        head: &str,
        base: Option<&str>,
    ) -> Result<String, RepoLensError> {
        let mut args = vec![
            "pr", "create", "--title", title, "--body", body, "--head", head,
        ];

        if let Some(base_branch) = base {
            args.push("--base");
            args.push(base_branch);
        }

        let output = Command::new("gh").args(&args).output().map_err(|_| {
            RepoLensError::Provider(ProviderError::CommandFailed {
                command: format!("gh {}", args.join(" ")),
            })
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RepoLensError::Provider(ProviderError::CommandFailed {
                command: format!("gh pr create: {}", stderr),
            }));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let url = stdout.trim().to_string();
        Ok(url)
    }
}

/// Secret scanning settings from GitHub API
#[derive(Debug, Clone, Deserialize)]
pub struct SecretScanningSettings {
    /// Whether secret scanning is enabled
    pub enabled: bool,
    /// Whether push protection is enabled
    pub push_protection_enabled: bool,
}

/// Actions permissions settings from GitHub API
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ActionsPermissions {
    /// Whether actions are enabled
    pub enabled: bool,
    /// Allowed actions: "all", "local_only", "selected"
    pub allowed_actions: Option<String>,
    /// Default workflow permissions: "read" or "write"
    pub default_workflow_permissions: Option<String>,
    /// Whether actions can approve pull request reviews
    pub can_approve_pull_request_reviews: Option<bool>,
}

/// Fork pull request workflow approval settings
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ForkPullRequestWorkflowsPolicy {
    /// Approval requirement: "none", "contributor", "everyone"
    pub default_workflow_permissions: Option<String>,
}

/// Branch protection settings from GitHub API
#[derive(Debug, Deserialize)]
pub struct BranchProtection {
    #[serde(rename = "required_status_checks")]
    pub required_status_checks: Option<StatusChecks>,

    #[serde(rename = "enforce_admins")]
    #[allow(dead_code)]
    pub enforce_admins: Option<EnforceAdmins>,

    #[serde(rename = "required_pull_request_reviews")]
    pub required_pull_request_reviews: Option<PullRequestReviews>,

    #[serde(rename = "required_linear_history")]
    #[allow(dead_code)]
    pub required_linear_history: Option<RequiredLinearHistory>,

    #[serde(rename = "allow_force_pushes")]
    pub allow_force_pushes: Option<AllowForcePushes>,

    #[serde(rename = "allow_deletions")]
    #[allow(dead_code)]
    pub allow_deletions: Option<AllowDeletions>,
}

#[derive(Debug, Deserialize)]
pub struct StatusChecks {
    #[allow(dead_code)]
    pub strict: bool,
    #[allow(dead_code)]
    pub contexts: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct EnforceAdmins {
    #[allow(dead_code)]
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct PullRequestReviews {
    #[serde(rename = "required_approving_review_count")]
    pub required_approving_review_count: u32,
}

#[derive(Debug, Deserialize)]
pub struct RequiredLinearHistory {
    #[allow(dead_code)]
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct AllowForcePushes {
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct AllowDeletions {
    #[allow(dead_code)]
    pub enabled: bool,
}

// ===== Access Control Structures =====

/// Repository collaborator from GitHub API
#[derive(Debug, Clone, Deserialize)]
pub struct Collaborator {
    pub login: String,
    #[serde(default)]
    pub permissions: CollaboratorPermissions,
    #[serde(rename = "type", default)]
    pub user_type: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[allow(dead_code)]
pub struct CollaboratorPermissions {
    #[serde(default)]
    pub admin: bool,
    #[serde(default)]
    pub push: bool,
    #[serde(default)]
    pub pull: bool,
}

/// Repository team from GitHub API
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Team {
    pub name: String,
    pub slug: String,
    #[serde(default)]
    pub permission: String,
}

/// Deploy key from GitHub API
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct DeployKey {
    pub id: u64,
    pub title: String,
    #[serde(default)]
    pub read_only: bool,
    pub created_at: Option<String>,
}

/// GitHub App installation from GitHub API
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Installation {
    pub id: u64,
    pub app_slug: Option<String>,
    #[serde(default)]
    pub permissions: InstallationPermissions,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[allow(dead_code)]
pub struct InstallationPermissions {
    #[serde(default)]
    pub contents: Option<String>,
    #[serde(default)]
    pub metadata: Option<String>,
    #[serde(default)]
    pub pull_requests: Option<String>,
    #[serde(default)]
    pub issues: Option<String>,
    #[serde(default)]
    pub actions: Option<String>,
    #[serde(default)]
    pub administration: Option<String>,
}

// ===== Infrastructure Structures =====

/// Webhook from GitHub API
#[derive(Debug, Clone, Deserialize)]
pub struct Webhook {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub active: bool,
    pub config: WebhookConfig,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[allow(dead_code)]
pub struct WebhookConfig {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub content_type: Option<String>,
    #[serde(default)]
    pub insecure_ssl: Option<String>,
    #[serde(default)]
    pub secret: Option<String>,
}

/// Environment from GitHub API
#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Environment {
    pub id: u64,
    pub name: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Environment protection rules from GitHub API
#[derive(Debug, Clone, Default, Deserialize)]
pub struct EnvironmentProtection {
    #[serde(default)]
    pub protection_rules: Vec<ProtectionRule>,
    #[serde(default)]
    pub deployment_branch_policy: Option<DeploymentBranchPolicy>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ProtectionRule {
    #[serde(rename = "type")]
    pub rule_type: String,
    #[serde(default)]
    pub wait_timer: Option<u32>,
    #[serde(default)]
    pub reviewers: Option<Vec<Reviewer>>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct Reviewer {
    #[serde(rename = "type")]
    pub reviewer_type: String,
    #[serde(default)]
    pub reviewer: Option<ReviewerDetails>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ReviewerDetails {
    pub login: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct DeploymentBranchPolicy {
    #[serde(default)]
    pub protected_branches: bool,
    #[serde(default)]
    pub custom_branch_policies: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a GitHubProvider with test values (bypasses API calls)
    fn test_provider() -> GitHubProvider {
        GitHubProvider {
            repo_owner: "test-owner".to_string(),
            repo_name: "test-repo".to_string(),
            octocrab: None,
        }
    }

    #[test]
    fn test_full_name() {
        let provider = test_provider();
        assert_eq!(provider.full_name(), "test-owner/test-repo");
    }

    #[test]
    fn test_owner_and_name() {
        let provider = test_provider();
        assert_eq!(provider.owner(), "test-owner");
        assert_eq!(provider.name(), "test-repo");
    }

    #[test]
    fn test_parse_github_url_https() {
        let result = GitHubProvider::parse_github_url("https://github.com/owner/repo.git");
        assert!(result.is_ok());
        let (owner, name) = result.unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(name, "repo");
    }

    #[test]
    fn test_parse_github_url_https_no_git() {
        let result = GitHubProvider::parse_github_url("https://github.com/owner/repo");
        assert!(result.is_ok());
        let (owner, name) = result.unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(name, "repo");
    }

    #[test]
    fn test_parse_github_url_ssh() {
        let result = GitHubProvider::parse_github_url("git@github.com:owner/repo.git");
        assert!(result.is_ok());
        let (owner, name) = result.unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(name, "repo");
    }

    #[test]
    fn test_parse_github_url_invalid() {
        let result = GitHubProvider::parse_github_url("https://gitlab.com/owner/repo.git");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_repo_name_valid() {
        let result = GitHubProvider::parse_repo_name("owner/repo");
        assert!(result.is_ok());
        let (owner, name) = result.unwrap();
        assert_eq!(owner, "owner");
        assert_eq!(name, "repo");
    }

    #[test]
    fn test_parse_repo_name_invalid() {
        let result = GitHubProvider::parse_repo_name("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_has_token_without_env() {
        // Clear the env var if set (in test isolation)
        let original = env::var("GITHUB_TOKEN").ok();
        // TODO: Audit that the environment access only happens in single-threaded code.
        unsafe { env::remove_var("GITHUB_TOKEN") };

        assert!(!GitHubProvider::has_token());

        // Restore original value
        if let Some(val) = original {
            // TODO: Audit that the environment access only happens in single-threaded code.
            unsafe { env::set_var("GITHUB_TOKEN", val) };
        }
    }

    #[test]
    fn test_is_available_returns_bool() {
        // Just verify is_available() doesn't panic and returns a bool
        let _result: bool = GitHubProvider::is_available();
    }

    #[test]
    fn test_octocrab_accessor() {
        let provider = test_provider();
        // Test provider has no octocrab instance
        assert!(provider.octocrab().is_none());
    }

    #[test]
    fn test_branch_protection_deserialization() {
        let json = r#"{
            "required_status_checks": {
                "strict": true,
                "contexts": ["ci/test", "ci/build"]
            },
            "enforce_admins": {
                "enabled": true
            },
            "required_pull_request_reviews": {
                "required_approving_review_count": 2
            },
            "required_linear_history": {
                "enabled": false
            },
            "allow_force_pushes": {
                "enabled": false
            },
            "allow_deletions": {
                "enabled": false
            }
        }"#;

        let protection: BranchProtection = serde_json::from_str(json).unwrap();

        assert!(protection.required_status_checks.is_some());
        let status_checks = protection.required_status_checks.unwrap();
        assert!(status_checks.strict);
        assert_eq!(status_checks.contexts.len(), 2);

        assert!(protection.required_pull_request_reviews.is_some());
        assert_eq!(
            protection
                .required_pull_request_reviews
                .unwrap()
                .required_approving_review_count,
            2
        );

        assert!(protection.allow_force_pushes.is_some());
        assert!(!protection.allow_force_pushes.unwrap().enabled);
    }

    #[test]
    fn test_branch_protection_minimal_deserialization() {
        let json = r#"{}"#;
        let protection: BranchProtection = serde_json::from_str(json).unwrap();

        assert!(protection.required_status_checks.is_none());
        assert!(protection.required_pull_request_reviews.is_none());
        assert!(protection.allow_force_pushes.is_none());
    }

    #[test]
    fn test_repo_info_deserialization() {
        let json = r#"{
            "name": "test-repo",
            "owner": {
                "login": "test-owner"
            },
            "hasIssuesEnabled": true,
            "hasDiscussionsEnabled": false,
            "hasWikiEnabled": true
        }"#;

        let repo_info: RepoInfo = serde_json::from_str(json).unwrap();
        assert!(repo_info.has_issues_enabled);
        assert!(!repo_info.has_discussions_enabled);
        assert!(repo_info.has_wiki_enabled);
    }

    #[test]
    fn test_provider_full_name_format() {
        let provider = GitHubProvider {
            repo_owner: "my-org".to_string(),
            repo_name: "my-repo".to_string(),
            octocrab: None,
        };
        assert_eq!(provider.full_name(), "my-org/my-repo");
    }

    #[test]
    fn test_status_checks_deserialization() {
        let json = r#"{
            "strict": false,
            "contexts": []
        }"#;
        let checks: StatusChecks = serde_json::from_str(json).unwrap();
        assert!(!checks.strict);
        assert!(checks.contexts.is_empty());
    }

    #[test]
    fn test_pull_request_reviews_deserialization() {
        let json = r#"{
            "required_approving_review_count": 1
        }"#;
        let reviews: PullRequestReviews = serde_json::from_str(json).unwrap();
        assert_eq!(reviews.required_approving_review_count, 1);
    }

    #[test]
    fn test_allow_force_pushes_deserialization() {
        let json_enabled = r#"{"enabled": true}"#;
        let json_disabled = r#"{"enabled": false}"#;

        let enabled: AllowForcePushes = serde_json::from_str(json_enabled).unwrap();
        let disabled: AllowForcePushes = serde_json::from_str(json_disabled).unwrap();

        assert!(enabled.enabled);
        assert!(!disabled.enabled);
    }

    #[test]
    fn test_secret_scanning_settings_deserialization() {
        let json = r#"{
            "enabled": true,
            "push_protection_enabled": false
        }"#;

        let settings: SecretScanningSettings = serde_json::from_str(json).unwrap();
        assert!(settings.enabled);
        assert!(!settings.push_protection_enabled);
    }

    #[test]
    fn test_secret_scanning_settings_both_enabled() {
        let json = r#"{
            "enabled": true,
            "push_protection_enabled": true
        }"#;

        let settings: SecretScanningSettings = serde_json::from_str(json).unwrap();
        assert!(settings.enabled);
        assert!(settings.push_protection_enabled);
    }

    #[test]
    fn test_secret_scanning_settings_both_disabled() {
        let json = r#"{
            "enabled": false,
            "push_protection_enabled": false
        }"#;

        let settings: SecretScanningSettings = serde_json::from_str(json).unwrap();
        assert!(!settings.enabled);
        assert!(!settings.push_protection_enabled);
    }

    #[test]
    fn test_actions_permissions_deserialization_full() {
        let json = r#"{
            "enabled": true,
            "allowed_actions": "selected",
            "default_workflow_permissions": "read",
            "can_approve_pull_request_reviews": false
        }"#;

        let perms: ActionsPermissions = serde_json::from_str(json).unwrap();
        assert!(perms.enabled);
        assert_eq!(perms.allowed_actions, Some("selected".to_string()));
        assert_eq!(perms.default_workflow_permissions, Some("read".to_string()));
        assert_eq!(perms.can_approve_pull_request_reviews, Some(false));
    }

    #[test]
    fn test_actions_permissions_deserialization_minimal() {
        let json = r#"{
            "enabled": false
        }"#;

        let perms: ActionsPermissions = serde_json::from_str(json).unwrap();
        assert!(!perms.enabled);
        assert!(perms.allowed_actions.is_none());
        assert!(perms.default_workflow_permissions.is_none());
        assert!(perms.can_approve_pull_request_reviews.is_none());
    }

    #[test]
    fn test_actions_permissions_all_allowed() {
        let json = r#"{
            "enabled": true,
            "allowed_actions": "all",
            "default_workflow_permissions": "write",
            "can_approve_pull_request_reviews": true
        }"#;

        let perms: ActionsPermissions = serde_json::from_str(json).unwrap();
        assert!(perms.enabled);
        assert_eq!(perms.allowed_actions, Some("all".to_string()));
        assert_eq!(
            perms.default_workflow_permissions,
            Some("write".to_string())
        );
        assert_eq!(perms.can_approve_pull_request_reviews, Some(true));
    }

    #[test]
    fn test_actions_permissions_local_only() {
        let json = r#"{
            "enabled": true,
            "allowed_actions": "local_only"
        }"#;

        let perms: ActionsPermissions = serde_json::from_str(json).unwrap();
        assert!(perms.enabled);
        assert_eq!(perms.allowed_actions, Some("local_only".to_string()));
    }

    // ===== Access Control Tests =====

    #[test]
    fn test_collaborator_deserialization() {
        let json = r#"{
            "login": "octocat",
            "permissions": {
                "admin": true,
                "push": true,
                "pull": true
            },
            "type": "User"
        }"#;

        let collab: Collaborator = serde_json::from_str(json).unwrap();
        assert_eq!(collab.login, "octocat");
        assert!(collab.permissions.admin);
        assert!(collab.permissions.push);
        assert!(collab.permissions.pull);
        assert_eq!(collab.user_type, "User");
    }

    #[test]
    fn test_collaborator_minimal() {
        let json = r#"{"login": "testuser"}"#;

        let collab: Collaborator = serde_json::from_str(json).unwrap();
        assert_eq!(collab.login, "testuser");
        assert!(!collab.permissions.admin);
        assert!(!collab.permissions.push);
        assert!(!collab.permissions.pull);
    }

    #[test]
    fn test_team_deserialization() {
        let json = r#"{
            "name": "Developers",
            "slug": "developers",
            "permission": "push"
        }"#;

        let team: Team = serde_json::from_str(json).unwrap();
        assert_eq!(team.name, "Developers");
        assert_eq!(team.slug, "developers");
        assert_eq!(team.permission, "push");
    }

    #[test]
    fn test_team_admin_permission() {
        let json = r#"{
            "name": "Admins",
            "slug": "admins",
            "permission": "admin"
        }"#;

        let team: Team = serde_json::from_str(json).unwrap();
        assert_eq!(team.permission, "admin");
    }

    #[test]
    fn test_deploy_key_deserialization() {
        let json = r#"{
            "id": 12345,
            "title": "Production Deploy Key",
            "read_only": false,
            "created_at": "2023-01-15T10:30:00Z"
        }"#;

        let key: DeployKey = serde_json::from_str(json).unwrap();
        assert_eq!(key.id, 12345);
        assert_eq!(key.title, "Production Deploy Key");
        assert!(!key.read_only);
        assert_eq!(key.created_at, Some("2023-01-15T10:30:00Z".to_string()));
    }

    #[test]
    fn test_deploy_key_read_only() {
        let json = r#"{
            "id": 67890,
            "title": "CI Deploy Key",
            "read_only": true
        }"#;

        let key: DeployKey = serde_json::from_str(json).unwrap();
        assert!(key.read_only);
        assert!(key.created_at.is_none());
    }

    #[test]
    fn test_installation_deserialization() {
        let json = r#"{
            "id": 999,
            "app_slug": "my-github-app",
            "permissions": {
                "contents": "write",
                "metadata": "read",
                "pull_requests": "write",
                "issues": "write",
                "actions": "read",
                "administration": "read"
            }
        }"#;

        let inst: Installation = serde_json::from_str(json).unwrap();
        assert_eq!(inst.id, 999);
        assert_eq!(inst.app_slug, Some("my-github-app".to_string()));
        assert_eq!(inst.permissions.contents, Some("write".to_string()));
        assert_eq!(inst.permissions.administration, Some("read".to_string()));
    }

    #[test]
    fn test_installation_minimal() {
        let json = r#"{"id": 123}"#;

        let inst: Installation = serde_json::from_str(json).unwrap();
        assert_eq!(inst.id, 123);
        assert!(inst.app_slug.is_none());
        assert!(inst.permissions.contents.is_none());
    }

    // ===== Infrastructure Tests =====

    #[test]
    fn test_webhook_deserialization() {
        let json = r#"{
            "id": 111,
            "name": "web",
            "active": true,
            "config": {
                "url": "https://example.com/webhook",
                "content_type": "json",
                "insecure_ssl": "0",
                "secret": "********"
            }
        }"#;

        let hook: Webhook = serde_json::from_str(json).unwrap();
        assert_eq!(hook.id, 111);
        assert_eq!(hook.name, "web");
        assert!(hook.active);
        assert_eq!(
            hook.config.url,
            Some("https://example.com/webhook".to_string())
        );
        assert_eq!(hook.config.content_type, Some("json".to_string()));
    }

    #[test]
    fn test_webhook_non_https() {
        let json = r#"{
            "id": 222,
            "name": "web",
            "active": true,
            "config": {
                "url": "http://insecure.example.com/hook"
            }
        }"#;

        let hook: Webhook = serde_json::from_str(json).unwrap();
        assert!(
            hook.config
                .url
                .as_ref()
                .map(|u| u.starts_with("http://"))
                .unwrap_or(false)
        );
    }

    #[test]
    fn test_webhook_inactive() {
        let json = r#"{
            "id": 333,
            "name": "web",
            "active": false,
            "config": {}
        }"#;

        let hook: Webhook = serde_json::from_str(json).unwrap();
        assert!(!hook.active);
    }

    #[test]
    fn test_webhook_no_secret() {
        let json = r#"{
            "id": 444,
            "name": "web",
            "active": true,
            "config": {
                "url": "https://example.com/hook"
            }
        }"#;

        let hook: Webhook = serde_json::from_str(json).unwrap();
        assert!(hook.config.secret.is_none());
    }

    #[test]
    fn test_environment_deserialization() {
        let json = r#"{
            "id": 555,
            "name": "production",
            "created_at": "2023-06-01T00:00:00Z",
            "updated_at": "2023-06-15T12:00:00Z"
        }"#;

        let env: Environment = serde_json::from_str(json).unwrap();
        assert_eq!(env.id, 555);
        assert_eq!(env.name, "production");
        assert!(env.created_at.is_some());
    }

    #[test]
    fn test_environment_protection_deserialization() {
        let json = r#"{
            "protection_rules": [
                {
                    "type": "required_reviewers",
                    "reviewers": [
                        {
                            "type": "User",
                            "reviewer": {
                                "login": "reviewer1"
                            }
                        }
                    ]
                },
                {
                    "type": "wait_timer",
                    "wait_timer": 30
                }
            ],
            "deployment_branch_policy": {
                "protected_branches": true,
                "custom_branch_policies": false
            }
        }"#;

        let prot: EnvironmentProtection = serde_json::from_str(json).unwrap();
        assert_eq!(prot.protection_rules.len(), 2);
        assert_eq!(prot.protection_rules[0].rule_type, "required_reviewers");
        assert_eq!(prot.protection_rules[1].rule_type, "wait_timer");
        assert_eq!(prot.protection_rules[1].wait_timer, Some(30));
        assert!(prot.deployment_branch_policy.is_some());
        let policy = prot.deployment_branch_policy.unwrap();
        assert!(policy.protected_branches);
        assert!(!policy.custom_branch_policies);
    }

    #[test]
    fn test_environment_protection_empty() {
        let json = r#"{}"#;

        let prot: EnvironmentProtection = serde_json::from_str(json).unwrap();
        assert!(prot.protection_rules.is_empty());
        assert!(prot.deployment_branch_policy.is_none());
    }

    #[test]
    fn test_environment_protection_default() {
        let prot = EnvironmentProtection::default();
        assert!(prot.protection_rules.is_empty());
        assert!(prot.deployment_branch_policy.is_none());
    }

    #[test]
    fn test_protection_rule_with_reviewers() {
        let json = r#"{
            "type": "required_reviewers",
            "reviewers": [
                {
                    "type": "Team",
                    "reviewer": {
                        "name": "security-team"
                    }
                }
            ]
        }"#;

        let rule: ProtectionRule = serde_json::from_str(json).unwrap();
        assert_eq!(rule.rule_type, "required_reviewers");
        assert!(rule.reviewers.is_some());
        let reviewers = rule.reviewers.unwrap();
        assert_eq!(reviewers.len(), 1);
        assert_eq!(reviewers[0].reviewer_type, "Team");
    }

    #[test]
    fn test_deployment_branch_policy() {
        let json = r#"{
            "protected_branches": false,
            "custom_branch_policies": true
        }"#;

        let policy: DeploymentBranchPolicy = serde_json::from_str(json).unwrap();
        assert!(!policy.protected_branches);
        assert!(policy.custom_branch_policies);
    }
}
