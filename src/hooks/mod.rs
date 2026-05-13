//! Git hooks management module
//!
//! This module provides functionality for installing and managing Git hooks
//! that integrate RepoLens into the development workflow. Supported hooks:
//! - **pre-commit**: Checks for exposed secrets before each commit
//! - **pre-push**: Runs a full audit before pushing to a remote

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{ActionError, RepoLensError};

/// Name of the pre-commit hook file
const PRE_COMMIT_HOOK: &str = "pre-commit";

/// Name of the pre-push hook file
const PRE_PUSH_HOOK: &str = "pre-push";

/// Suffix used for backing up existing hooks
const BACKUP_SUFFIX: &str = ".repolens-backup";

/// Configuration for Git hooks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HooksConfig {
    /// Whether to install the pre-commit hook
    #[serde(default = "default_true")]
    pub pre_commit: bool,
    /// Whether to install the pre-push hook
    #[serde(default = "default_true")]
    pub pre_push: bool,
    /// Whether warnings should cause hook failure
    #[serde(default)]
    pub fail_on_warnings: bool,
}

fn default_true() -> bool {
    true
}

impl Default for HooksConfig {
    fn default() -> Self {
        Self {
            pre_commit: true,
            pre_push: true,
            fail_on_warnings: false,
        }
    }
}

/// Manages the installation and removal of Git hooks
#[derive(Debug)]
pub struct HooksManager {
    /// Path to the .git/hooks directory
    hooks_dir: PathBuf,
    /// Hook configuration
    config: HooksConfig,
}

impl HooksManager {
    /// Create a new HooksManager for the given repository root
    ///
    /// # Arguments
    ///
    /// * `repo_root` - The root directory of the git repository
    /// * `config` - Hook configuration
    ///
    /// # Errors
    ///
    /// Returns an error if the `.git/hooks` directory cannot be found or created
    pub fn new(repo_root: &Path, config: HooksConfig) -> Result<Self, RepoLensError> {
        let hooks_dir = find_hooks_dir(repo_root)?;
        Ok(Self { hooks_dir, config })
    }

    /// Install the configured Git hooks
    ///
    /// # Arguments
    ///
    /// * `force` - Whether to overwrite existing hooks (after backing them up)
    ///
    /// # Returns
    ///
    /// A vector of messages describing what was installed
    pub fn install(&self, force: bool) -> Result<Vec<String>, RepoLensError> {
        let mut messages = Vec::new();

        // Ensure hooks directory exists
        fs::create_dir_all(&self.hooks_dir).map_err(|e| {
            RepoLensError::Action(ActionError::DirectoryCreate {
                path: self.hooks_dir.display().to_string(),
                source: e,
            })
        })?;

        if self.config.pre_commit {
            let msg = self.install_hook(
                PRE_COMMIT_HOOK,
                &generate_pre_commit_hook(&self.config),
                force,
            )?;
            messages.push(msg);
        }

        if self.config.pre_push {
            let msg =
                self.install_hook(PRE_PUSH_HOOK, &generate_pre_push_hook(&self.config), force)?;
            messages.push(msg);
        }

        Ok(messages)
    }

    /// Remove installed RepoLens hooks, restoring backups if they exist
    ///
    /// # Returns
    ///
    /// A vector of messages describing what was removed
    pub fn remove(&self) -> Result<Vec<String>, RepoLensError> {
        let mut messages = Vec::new();

        let msg = self.remove_hook(PRE_COMMIT_HOOK)?;
        messages.push(msg);

        let msg = self.remove_hook(PRE_PUSH_HOOK)?;
        messages.push(msg);

        Ok(messages)
    }

    /// Install a single hook file
    fn install_hook(
        &self,
        hook_name: &str,
        content: &str,
        force: bool,
    ) -> Result<String, RepoLensError> {
        let hook_path = self.hooks_dir.join(hook_name);

        // Check if hook already exists
        if hook_path.exists() {
            if is_repolens_hook(&hook_path)? {
                // Already a RepoLens hook, overwrite it
                write_hook_file(&hook_path, content)?;
                return Ok(format!("Updated existing RepoLens {} hook", hook_name));
            }

            if !force {
                return Err(RepoLensError::Action(ActionError::ExecutionFailed {
                    message: format!(
                        "Hook '{}' already exists. Use --force to overwrite (existing hook will be backed up)",
                        hook_name
                    ),
                }));
            }

            // Backup existing hook
            let backup_path = self
                .hooks_dir
                .join(format!("{}{}", hook_name, BACKUP_SUFFIX));
            fs::copy(&hook_path, &backup_path).map_err(|e| {
                RepoLensError::Action(ActionError::FileWrite {
                    path: backup_path.display().to_string(),
                    source: e,
                })
            })?;

            write_hook_file(&hook_path, content)?;
            return Ok(format!(
                "Installed {} hook (existing hook backed up to {}{})",
                hook_name, hook_name, BACKUP_SUFFIX
            ));
        }

        write_hook_file(&hook_path, content)?;
        Ok(format!("Installed {} hook", hook_name))
    }

    /// Remove a single hook file, restoring backup if available
    fn remove_hook(&self, hook_name: &str) -> Result<String, RepoLensError> {
        let hook_path = self.hooks_dir.join(hook_name);
        let backup_path = self
            .hooks_dir
            .join(format!("{}{}", hook_name, BACKUP_SUFFIX));

        if !hook_path.exists() {
            return Ok(format!("No {} hook to remove", hook_name));
        }

        if !is_repolens_hook(&hook_path)? {
            return Ok(format!(
                "Skipped {} hook (not installed by RepoLens)",
                hook_name
            ));
        }

        fs::remove_file(&hook_path).map_err(|e| {
            RepoLensError::Action(ActionError::FileWrite {
                path: hook_path.display().to_string(),
                source: e,
            })
        })?;

        // Restore backup if it exists
        if backup_path.exists() {
            fs::rename(&backup_path, &hook_path).map_err(|e| {
                RepoLensError::Action(ActionError::FileWrite {
                    path: hook_path.display().to_string(),
                    source: e,
                })
            })?;
            return Ok(format!(
                "Removed {} hook and restored previous hook from backup",
                hook_name
            ));
        }

        Ok(format!("Removed {} hook", hook_name))
    }

    /// Get the path to the hooks directory
    #[allow(dead_code)]
    pub fn hooks_dir(&self) -> &Path {
        &self.hooks_dir
    }
}

/// Find the `.git/hooks` directory for a repository
///
/// This function handles both standard repositories and worktrees.
fn find_hooks_dir(repo_root: &Path) -> Result<PathBuf, RepoLensError> {
    let git_dir = repo_root.join(".git");

    if git_dir.is_dir() {
        return Ok(git_dir.join("hooks"));
    }

    // Handle git worktrees: .git is a file containing "gitdir: <path>"
    if git_dir.is_file() {
        let content = fs::read_to_string(&git_dir).map_err(|e| {
            RepoLensError::Action(ActionError::ExecutionFailed {
                message: format!("Failed to read .git file: {}", e),
            })
        })?;

        if let Some(gitdir_path) = content.strip_prefix("gitdir: ") {
            let gitdir_path = gitdir_path.trim();
            let gitdir = if Path::new(gitdir_path).is_absolute() {
                PathBuf::from(gitdir_path)
            } else {
                repo_root.join(gitdir_path)
            };
            return Ok(gitdir.join("hooks"));
        }
    }

    Err(RepoLensError::Action(ActionError::ExecutionFailed {
        message: format!(
            "Could not find .git directory in {}. Is this a git repository?",
            repo_root.display()
        ),
    }))
}

/// Check if a hook file was installed by RepoLens
fn is_repolens_hook(hook_path: &Path) -> Result<bool, RepoLensError> {
    let content = fs::read_to_string(hook_path).map_err(|e| {
        RepoLensError::Action(ActionError::ExecutionFailed {
            message: format!("Failed to read hook file '{}': {}", hook_path.display(), e),
        })
    })?;
    Ok(content.contains("# RepoLens Git Hook"))
}

/// Write a hook file and set executable permissions
fn write_hook_file(path: &Path, content: &str) -> Result<(), RepoLensError> {
    fs::write(path, content).map_err(|e| {
        RepoLensError::Action(ActionError::FileWrite {
            path: path.display().to_string(),
            source: e,
        })
    })?;

    // Set executable permissions on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = fs::Permissions::from_mode(0o755);
        fs::set_permissions(path, permissions).map_err(|e| {
            RepoLensError::Action(ActionError::FileWrite {
                path: path.display().to_string(),
                source: e,
            })
        })?;
    }

    Ok(())
}

/// Generate the content of the pre-commit hook script
///
/// The pre-commit hook runs a secrets scan to prevent committing exposed credentials.
pub fn generate_pre_commit_hook(config: &HooksConfig) -> String {
    let fail_on_warnings = if config.fail_on_warnings {
        " --fail-on-warnings"
    } else {
        ""
    };

    format!(
        r#"#!/bin/sh
# RepoLens Git Hook - pre-commit
# This hook was automatically installed by RepoLens.
# It checks for exposed secrets before allowing a commit.
#
# To skip this hook, use: git commit --no-verify

set -e

echo "RepoLens: Checking for exposed secrets..."

if ! command -v repolens >/dev/null 2>&1; then
    echo "Warning: repolens is not installed or not in PATH. Skipping pre-commit check."
    echo "Install it with: cargo install repolens"
    exit 0
fi

if ! repolens plan --only secrets --format terminal{fail_on_warnings} 2>/dev/null; then
    echo ""
    echo "RepoLens: Secrets detected! Commit aborted."
    echo "Please remove or ignore the detected secrets before committing."
    echo "To skip this check, use: git commit --no-verify"
    exit 1
fi

echo "RepoLens: No secrets detected. Proceeding with commit."
"#
    )
}

/// Generate the content of the pre-push hook script
///
/// The pre-push hook runs a full audit to prevent pushing code with issues.
pub fn generate_pre_push_hook(config: &HooksConfig) -> String {
    let fail_on_warnings = if config.fail_on_warnings {
        " --fail-on-warnings"
    } else {
        ""
    };

    format!(
        r#"#!/bin/sh
# RepoLens Git Hook - pre-push
# This hook was automatically installed by RepoLens.
# It runs a full audit before allowing a push.
#
# To skip this hook, use: git push --no-verify

set -e

echo "RepoLens: Running full audit before push..."

if ! command -v repolens >/dev/null 2>&1; then
    echo "Warning: repolens is not installed or not in PATH. Skipping pre-push check."
    echo "Install it with: cargo install repolens"
    exit 0
fi

if ! repolens plan --format terminal{fail_on_warnings} 2>/dev/null; then
    echo ""
    echo "RepoLens: Audit issues found! Push aborted."
    echo "Please fix the issues before pushing."
    echo "To skip this check, use: git push --no-verify"
    exit 1
fi

echo "RepoLens: Audit passed. Proceeding with push."
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Helper to create a temporary git repository structure
    fn create_temp_repo() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        let git_dir = temp_dir.path().join(".git");
        fs::create_dir_all(git_dir.join("hooks")).unwrap();
        temp_dir
    }

    /// Helper to create a temporary git worktree structure
    fn create_temp_worktree() -> (TempDir, TempDir) {
        // Main repo
        let main_repo = TempDir::new().unwrap();
        let main_git_dir = main_repo.path().join(".git");
        fs::create_dir_all(main_git_dir.join("worktrees").join("wt1")).unwrap();
        fs::create_dir_all(main_git_dir.join("hooks")).unwrap();

        // Worktree
        let worktree = TempDir::new().unwrap();
        let wt_git_dir = main_git_dir.join("worktrees").join("wt1");
        fs::create_dir_all(&wt_git_dir).unwrap();

        // .git file in worktree pointing to the worktree git dir
        let git_file_content = format!("gitdir: {}", wt_git_dir.display());
        fs::write(worktree.path().join(".git"), git_file_content).unwrap();
        fs::create_dir_all(wt_git_dir.join("hooks")).unwrap();

        (main_repo, worktree)
    }

    #[test]
    fn test_hooks_config_default() {
        let config = HooksConfig::default();
        assert!(config.pre_commit);
        assert!(config.pre_push);
        assert!(!config.fail_on_warnings);
    }

    #[test]
    fn test_find_hooks_dir_standard_repo() {
        let temp_dir = create_temp_repo();
        let hooks_dir = find_hooks_dir(temp_dir.path()).unwrap();
        assert_eq!(hooks_dir, temp_dir.path().join(".git/hooks"));
    }

    #[test]
    fn test_find_hooks_dir_worktree() {
        let (_main_repo, worktree) = create_temp_worktree();
        let hooks_dir = find_hooks_dir(worktree.path()).unwrap();
        assert!(hooks_dir.to_string_lossy().contains("hooks"));
    }

    #[test]
    fn test_find_hooks_dir_not_a_repo() {
        let temp_dir = TempDir::new().unwrap();
        let result = find_hooks_dir(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_hooks_manager_new() {
        let temp_dir = create_temp_repo();
        let config = HooksConfig::default();
        let manager = HooksManager::new(temp_dir.path(), config);
        assert!(manager.is_ok());
    }

    #[test]
    fn test_hooks_manager_new_not_a_repo() {
        let temp_dir = TempDir::new().unwrap();
        let config = HooksConfig::default();
        let result = HooksManager::new(temp_dir.path(), config);
        assert!(result.is_err());
    }

    #[test]
    fn test_install_all_hooks() {
        let temp_dir = create_temp_repo();
        let config = HooksConfig::default();
        let manager = HooksManager::new(temp_dir.path(), config).unwrap();

        let messages = manager.install(false).unwrap();
        assert_eq!(messages.len(), 2);

        // Verify files exist
        assert!(temp_dir.path().join(".git/hooks/pre-commit").exists());
        assert!(temp_dir.path().join(".git/hooks/pre-push").exists());
    }

    #[test]
    fn test_install_pre_commit_only() {
        let temp_dir = create_temp_repo();
        let config = HooksConfig {
            pre_commit: true,
            pre_push: false,
            fail_on_warnings: false,
        };
        let manager = HooksManager::new(temp_dir.path(), config).unwrap();

        let messages = manager.install(false).unwrap();
        assert_eq!(messages.len(), 1);
        assert!(messages[0].contains("pre-commit"));

        assert!(temp_dir.path().join(".git/hooks/pre-commit").exists());
        assert!(!temp_dir.path().join(".git/hooks/pre-push").exists());
    }

    #[test]
    fn test_install_pre_push_only() {
        let temp_dir = create_temp_repo();
        let config = HooksConfig {
            pre_commit: false,
            pre_push: true,
            fail_on_warnings: false,
        };
        let manager = HooksManager::new(temp_dir.path(), config).unwrap();

        let messages = manager.install(false).unwrap();
        assert_eq!(messages.len(), 1);
        assert!(messages[0].contains("pre-push"));

        assert!(!temp_dir.path().join(".git/hooks/pre-commit").exists());
        assert!(temp_dir.path().join(".git/hooks/pre-push").exists());
    }

    #[test]
    fn test_install_existing_hook_without_force() {
        let temp_dir = create_temp_repo();
        let hook_path = temp_dir.path().join(".git/hooks/pre-commit");
        fs::write(&hook_path, "#!/bin/sh\necho 'existing hook'").unwrap();

        let config = HooksConfig {
            pre_commit: true,
            pre_push: false,
            fail_on_warnings: false,
        };
        let manager = HooksManager::new(temp_dir.path(), config).unwrap();

        let result = manager.install(false);
        assert!(result.is_err());
    }

    #[test]
    fn test_install_existing_hook_with_force() {
        let temp_dir = create_temp_repo();
        let hook_path = temp_dir.path().join(".git/hooks/pre-commit");
        fs::write(&hook_path, "#!/bin/sh\necho 'existing hook'").unwrap();

        let config = HooksConfig {
            pre_commit: true,
            pre_push: false,
            fail_on_warnings: false,
        };
        let manager = HooksManager::new(temp_dir.path(), config).unwrap();

        let messages = manager.install(true).unwrap();
        assert_eq!(messages.len(), 1);
        assert!(messages[0].contains("backed up"));

        // Verify backup exists
        let backup_path = temp_dir
            .path()
            .join(".git/hooks/pre-commit.repolens-backup");
        assert!(backup_path.exists());
        let backup_content = fs::read_to_string(&backup_path).unwrap();
        assert!(backup_content.contains("existing hook"));

        // Verify new hook is installed
        let new_content = fs::read_to_string(&hook_path).unwrap();
        assert!(new_content.contains("RepoLens Git Hook"));
    }

    #[test]
    fn test_install_overwrites_existing_repolens_hook() {
        let temp_dir = create_temp_repo();
        let hook_path = temp_dir.path().join(".git/hooks/pre-commit");
        fs::write(
            &hook_path,
            "#!/bin/sh\n# RepoLens Git Hook\necho 'old version'",
        )
        .unwrap();

        let config = HooksConfig {
            pre_commit: true,
            pre_push: false,
            fail_on_warnings: false,
        };
        let manager = HooksManager::new(temp_dir.path(), config).unwrap();

        let messages = manager.install(false).unwrap();
        assert_eq!(messages.len(), 1);
        assert!(messages[0].contains("Updated"));

        // No backup should be created for RepoLens hooks
        let backup_path = temp_dir
            .path()
            .join(".git/hooks/pre-commit.repolens-backup");
        assert!(!backup_path.exists());
    }

    #[test]
    fn test_remove_hooks() {
        let temp_dir = create_temp_repo();
        let config = HooksConfig::default();
        let manager = HooksManager::new(temp_dir.path(), config).unwrap();

        // First install
        manager.install(false).unwrap();

        // Then remove
        let messages = manager.remove().unwrap();
        assert_eq!(messages.len(), 2);

        assert!(!temp_dir.path().join(".git/hooks/pre-commit").exists());
        assert!(!temp_dir.path().join(".git/hooks/pre-push").exists());
    }

    #[test]
    fn test_remove_nonexistent_hooks() {
        let temp_dir = create_temp_repo();
        let config = HooksConfig::default();
        let manager = HooksManager::new(temp_dir.path(), config).unwrap();

        let messages = manager.remove().unwrap();
        assert_eq!(messages.len(), 2);
        assert!(messages[0].contains("No"));
        assert!(messages[1].contains("No"));
    }

    #[test]
    fn test_remove_non_repolens_hook() {
        let temp_dir = create_temp_repo();
        let hook_path = temp_dir.path().join(".git/hooks/pre-commit");
        fs::write(&hook_path, "#!/bin/sh\necho 'custom hook'").unwrap();

        let config = HooksConfig::default();
        let manager = HooksManager::new(temp_dir.path(), config).unwrap();

        let messages = manager.remove().unwrap();
        assert!(messages[0].contains("Skipped"));

        // Hook should still exist
        assert!(hook_path.exists());
    }

    #[test]
    fn test_remove_with_backup_restoration() {
        let temp_dir = create_temp_repo();
        let hook_path = temp_dir.path().join(".git/hooks/pre-commit");
        let backup_path = temp_dir
            .path()
            .join(".git/hooks/pre-commit.repolens-backup");

        // Create existing hook then force-install
        fs::write(&hook_path, "#!/bin/sh\necho 'original hook'").unwrap();
        let config = HooksConfig {
            pre_commit: true,
            pre_push: false,
            fail_on_warnings: false,
        };
        let manager = HooksManager::new(temp_dir.path(), config).unwrap();
        manager.install(true).unwrap();

        // Verify backup exists
        assert!(backup_path.exists());

        // Remove hook - should restore backup
        let messages = manager.remove().unwrap();
        assert!(messages[0].contains("restored"));

        // Original hook should be restored
        let content = fs::read_to_string(&hook_path).unwrap();
        assert!(content.contains("original hook"));

        // Backup should be gone
        assert!(!backup_path.exists());
    }

    #[test]
    fn test_generate_pre_commit_hook_content() {
        let config = HooksConfig::default();
        let content = generate_pre_commit_hook(&config);

        assert!(content.starts_with("#!/bin/sh"));
        assert!(content.contains("# RepoLens Git Hook"));
        assert!(content.contains("pre-commit"));
        assert!(content.contains("repolens plan --only secrets"));
        assert!(content.contains("--no-verify"));
        assert!(!content.contains("--fail-on-warnings"));
    }

    #[test]
    fn test_generate_pre_commit_hook_with_fail_on_warnings() {
        let config = HooksConfig {
            pre_commit: true,
            pre_push: true,
            fail_on_warnings: true,
        };
        let content = generate_pre_commit_hook(&config);
        assert!(content.contains("--fail-on-warnings"));
    }

    #[test]
    fn test_generate_pre_push_hook_content() {
        let config = HooksConfig::default();
        let content = generate_pre_push_hook(&config);

        assert!(content.starts_with("#!/bin/sh"));
        assert!(content.contains("# RepoLens Git Hook"));
        assert!(content.contains("pre-push"));
        assert!(content.contains("repolens plan"));
        assert!(content.contains("--no-verify"));
        assert!(!content.contains("--fail-on-warnings"));
    }

    #[test]
    fn test_generate_pre_push_hook_with_fail_on_warnings() {
        let config = HooksConfig {
            pre_commit: true,
            pre_push: true,
            fail_on_warnings: true,
        };
        let content = generate_pre_push_hook(&config);
        assert!(content.contains("--fail-on-warnings"));
    }

    #[test]
    fn test_is_repolens_hook() {
        let temp_dir = TempDir::new().unwrap();

        // RepoLens hook
        let repolens_hook = temp_dir.path().join("repolens-hook");
        fs::write(&repolens_hook, "#!/bin/sh\n# RepoLens Git Hook\necho test").unwrap();
        assert!(is_repolens_hook(&repolens_hook).unwrap());

        // Non-RepoLens hook
        let other_hook = temp_dir.path().join("other-hook");
        fs::write(&other_hook, "#!/bin/sh\necho test").unwrap();
        assert!(!is_repolens_hook(&other_hook).unwrap());
    }

    #[test]
    fn test_write_hook_file() {
        let temp_dir = TempDir::new().unwrap();
        let hook_path = temp_dir.path().join("test-hook");
        let content = "#!/bin/sh\necho 'test'";

        write_hook_file(&hook_path, content).unwrap();

        let written = fs::read_to_string(&hook_path).unwrap();
        assert_eq!(written, content);

        // Check permissions on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = fs::metadata(&hook_path).unwrap();
            let mode = metadata.permissions().mode();
            assert_eq!(mode & 0o755, 0o755);
        }
    }

    #[test]
    fn test_hooks_manager_hooks_dir() {
        let temp_dir = create_temp_repo();
        let config = HooksConfig::default();
        let manager = HooksManager::new(temp_dir.path(), config).unwrap();
        assert_eq!(manager.hooks_dir(), temp_dir.path().join(".git/hooks"));
    }

    #[test]
    fn test_install_creates_hooks_dir_if_missing() {
        let temp_dir = TempDir::new().unwrap();
        // Create .git dir but not hooks subdir
        let git_dir = temp_dir.path().join(".git");
        fs::create_dir_all(&git_dir).unwrap();

        let config = HooksConfig {
            pre_commit: true,
            pre_push: false,
            fail_on_warnings: false,
        };
        let manager = HooksManager::new(temp_dir.path(), config).unwrap();

        let messages = manager.install(false).unwrap();
        assert_eq!(messages.len(), 1);
        assert!(temp_dir.path().join(".git/hooks/pre-commit").exists());
    }

    #[test]
    fn test_install_no_hooks_selected() {
        let temp_dir = create_temp_repo();
        let config = HooksConfig {
            pre_commit: false,
            pre_push: false,
            fail_on_warnings: false,
        };
        let manager = HooksManager::new(temp_dir.path(), config).unwrap();

        let messages = manager.install(false).unwrap();
        assert!(messages.is_empty());
    }

    #[test]
    fn test_find_hooks_dir_invalid_git_file() {
        let temp_dir = TempDir::new().unwrap();
        // Create .git as a file with invalid content
        fs::write(temp_dir.path().join(".git"), "not a gitdir reference").unwrap();

        let result = find_hooks_dir(temp_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_hooks_config_serialize_deserialize() {
        let config = HooksConfig {
            pre_commit: true,
            pre_push: false,
            fail_on_warnings: true,
        };
        let toml_str = toml::to_string(&config).unwrap();
        let deserialized: HooksConfig = toml::from_str(&toml_str).unwrap();
        assert!(deserialized.pre_commit);
        assert!(!deserialized.pre_push);
        assert!(deserialized.fail_on_warnings);
    }

    #[test]
    fn test_hooks_config_deserialize_from_toml() {
        let toml_str = r#"
            pre_commit = false
            pre_push = true
            fail_on_warnings = true
        "#;
        let config: HooksConfig = toml::from_str(toml_str).unwrap();
        assert!(!config.pre_commit);
        assert!(config.pre_push);
        assert!(config.fail_on_warnings);
    }

    #[test]
    fn test_hooks_config_default_serde() {
        // Verify that default values work when fields are missing from TOML
        let toml_str = "";
        let config: HooksConfig = toml::from_str(toml_str).unwrap();
        assert!(config.pre_commit);
        assert!(config.pre_push);
        assert!(!config.fail_on_warnings);
    }

    #[test]
    fn test_find_hooks_dir_worktree_with_absolute_path() {
        // Test worktree with absolute gitdir path
        let main_repo = TempDir::new().unwrap();
        let main_git_dir = main_repo.path().join(".git");
        fs::create_dir_all(main_git_dir.join("worktrees").join("wt-abs")).unwrap();
        fs::create_dir_all(main_git_dir.join("hooks")).unwrap();

        let worktree = TempDir::new().unwrap();
        let wt_git_dir = main_git_dir.join("worktrees").join("wt-abs");
        fs::create_dir_all(wt_git_dir.join("hooks")).unwrap();

        // Write .git file with absolute path
        let git_file_content = format!("gitdir: {}", wt_git_dir.display());
        fs::write(worktree.path().join(".git"), git_file_content).unwrap();

        let hooks_dir = find_hooks_dir(worktree.path()).unwrap();
        assert!(hooks_dir.to_string_lossy().contains("hooks"));
        assert_eq!(hooks_dir, wt_git_dir.join("hooks"));
    }

    #[test]
    fn test_is_repolens_hook_nonexistent_file() {
        let temp_dir = TempDir::new().unwrap();
        let nonexistent = temp_dir.path().join("nonexistent");
        let result = is_repolens_hook(&nonexistent);
        assert!(result.is_err());
    }

    #[test]
    fn test_install_hook_content_contains_expected_strings() {
        let temp_dir = create_temp_repo();
        let config = HooksConfig {
            pre_commit: true,
            pre_push: true,
            fail_on_warnings: false,
        };
        let manager = HooksManager::new(temp_dir.path(), config).unwrap();
        manager.install(false).unwrap();

        // Verify pre-commit content
        let pre_commit_content =
            fs::read_to_string(temp_dir.path().join(".git/hooks/pre-commit")).unwrap();
        assert!(pre_commit_content.contains("#!/bin/sh"));
        assert!(pre_commit_content.contains("# RepoLens Git Hook"));
        assert!(pre_commit_content.contains("repolens plan --only secrets"));

        // Verify pre-push content
        let pre_push_content =
            fs::read_to_string(temp_dir.path().join(".git/hooks/pre-push")).unwrap();
        assert!(pre_push_content.contains("#!/bin/sh"));
        assert!(pre_push_content.contains("# RepoLens Git Hook"));
        assert!(pre_push_content.contains("repolens plan --format terminal"));
    }

    #[test]
    fn test_install_then_reinstall_repolens_hooks() {
        let temp_dir = create_temp_repo();
        let config = HooksConfig::default();
        let manager = HooksManager::new(temp_dir.path(), config).unwrap();

        // Install once
        let messages = manager.install(false).unwrap();
        assert_eq!(messages.len(), 2);
        assert!(messages[0].contains("Installed"));
        assert!(messages[1].contains("Installed"));

        // Install again - should update existing RepoLens hooks
        let messages = manager.install(false).unwrap();
        assert_eq!(messages.len(), 2);
        assert!(messages[0].contains("Updated"));
        assert!(messages[1].contains("Updated"));
    }

    #[test]
    fn test_remove_pre_push_hook() {
        let temp_dir = create_temp_repo();
        let config = HooksConfig::default();
        let manager = HooksManager::new(temp_dir.path(), config).unwrap();

        // Install both hooks
        manager.install(false).unwrap();
        assert!(temp_dir.path().join(".git/hooks/pre-push").exists());

        // Remove all hooks
        let messages = manager.remove().unwrap();
        assert_eq!(messages.len(), 2);
        assert!(messages[0].contains("Removed pre-commit"));
        assert!(messages[1].contains("Removed pre-push"));

        assert!(!temp_dir.path().join(".git/hooks/pre-commit").exists());
        assert!(!temp_dir.path().join(".git/hooks/pre-push").exists());
    }

    #[test]
    fn test_force_install_pre_push_with_existing() {
        let temp_dir = create_temp_repo();
        let hook_path = temp_dir.path().join(".git/hooks/pre-push");
        fs::write(&hook_path, "#!/bin/sh\necho 'existing pre-push'").unwrap();

        let config = HooksConfig {
            pre_commit: false,
            pre_push: true,
            fail_on_warnings: false,
        };
        let manager = HooksManager::new(temp_dir.path(), config).unwrap();

        // Force install should back up and replace
        let messages = manager.install(true).unwrap();
        assert_eq!(messages.len(), 1);
        assert!(messages[0].contains("backed up"));

        let backup_path = temp_dir.path().join(".git/hooks/pre-push.repolens-backup");
        assert!(backup_path.exists());
    }

    #[test]
    fn test_write_hook_file_error_on_invalid_path() {
        let result = write_hook_file(Path::new("/nonexistent/dir/hook"), "content");
        assert!(result.is_err());
    }

    #[test]
    fn test_hooks_config_clone() {
        let config = HooksConfig {
            pre_commit: false,
            pre_push: true,
            fail_on_warnings: true,
        };
        let cloned = config.clone();
        assert_eq!(cloned.pre_commit, config.pre_commit);
        assert_eq!(cloned.pre_push, config.pre_push);
        assert_eq!(cloned.fail_on_warnings, config.fail_on_warnings);
    }

    #[test]
    fn test_hooks_config_debug() {
        let config = HooksConfig::default();
        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("HooksConfig"));
        assert!(debug_str.contains("pre_commit"));
    }

    #[test]
    fn test_hooks_manager_debug() {
        let temp_dir = create_temp_repo();
        let config = HooksConfig::default();
        let manager = HooksManager::new(temp_dir.path(), config).unwrap();
        let debug_str = format!("{:?}", manager);
        assert!(debug_str.contains("HooksManager"));
    }

    #[test]
    fn test_find_hooks_dir_worktree_with_relative_path() {
        // Test worktree with relative gitdir path
        let temp_dir = TempDir::new().unwrap();

        // Create structure: temp_dir/.git/worktrees/wt-rel/hooks
        let main_git_dir = temp_dir.path().join(".git");
        let wt_git_dir = main_git_dir.join("worktrees").join("wt-rel");
        fs::create_dir_all(wt_git_dir.join("hooks")).unwrap();

        // Create worktree subdirectory
        let worktree_dir = temp_dir.path().join("worktree");
        fs::create_dir_all(&worktree_dir).unwrap();

        // Write .git file with relative path (relative from worktree_dir)
        // The relative path from worktree_dir to wt_git_dir is "../.git/worktrees/wt-rel"
        let git_file_content = "gitdir: ../.git/worktrees/wt-rel";
        fs::write(worktree_dir.join(".git"), git_file_content).unwrap();

        let hooks_dir = find_hooks_dir(&worktree_dir).unwrap();
        assert!(hooks_dir.to_string_lossy().contains("hooks"));
    }

    #[test]
    fn test_remove_hook_restores_backup_for_pre_push() {
        let temp_dir = create_temp_repo();
        let hook_path = temp_dir.path().join(".git/hooks/pre-push");
        let backup_path = temp_dir.path().join(".git/hooks/pre-push.repolens-backup");

        // Create existing hook then force-install
        fs::write(&hook_path, "#!/bin/sh\necho 'original pre-push'").unwrap();
        let config = HooksConfig {
            pre_commit: false,
            pre_push: true,
            fail_on_warnings: false,
        };
        let manager = HooksManager::new(temp_dir.path(), config).unwrap();
        manager.install(true).unwrap();

        // Verify backup exists
        assert!(backup_path.exists());

        // Remove hook - should restore backup
        let messages = manager.remove().unwrap();
        // First message is about pre-commit (no hook to remove)
        // Second message is about pre-push (restored)
        assert!(messages[1].contains("restored"));

        // Original hook should be restored
        let content = fs::read_to_string(&hook_path).unwrap();
        assert!(content.contains("original pre-push"));

        // Backup should be gone
        assert!(!backup_path.exists());
    }

    #[test]
    fn test_install_both_hooks_with_force_over_existing() {
        let temp_dir = create_temp_repo();
        let pre_commit_path = temp_dir.path().join(".git/hooks/pre-commit");
        let pre_push_path = temp_dir.path().join(".git/hooks/pre-push");

        // Create existing hooks
        fs::write(&pre_commit_path, "#!/bin/sh\necho 'old commit hook'").unwrap();
        fs::write(&pre_push_path, "#!/bin/sh\necho 'old push hook'").unwrap();

        let config = HooksConfig::default();
        let manager = HooksManager::new(temp_dir.path(), config).unwrap();

        let messages = manager.install(true).unwrap();
        assert_eq!(messages.len(), 2);
        assert!(messages[0].contains("backed up"));
        assert!(messages[1].contains("backed up"));

        // Verify both backups exist
        assert!(
            temp_dir
                .path()
                .join(".git/hooks/pre-commit.repolens-backup")
                .exists()
        );
        assert!(
            temp_dir
                .path()
                .join(".git/hooks/pre-push.repolens-backup")
                .exists()
        );

        // Verify new hooks are installed
        let commit_content = fs::read_to_string(&pre_commit_path).unwrap();
        assert!(commit_content.contains("RepoLens Git Hook"));
        let push_content = fs::read_to_string(&pre_push_path).unwrap();
        assert!(push_content.contains("RepoLens Git Hook"));
    }

    #[test]
    fn test_generate_hooks_scripts_structure() {
        let config = HooksConfig {
            pre_commit: true,
            pre_push: true,
            fail_on_warnings: false,
        };

        let pre_commit = generate_pre_commit_hook(&config);
        // Check script structure
        assert!(pre_commit.contains("set -e"));
        assert!(pre_commit.contains("command -v repolens"));
        assert!(pre_commit.contains("cargo install repolens"));
        assert!(pre_commit.contains("exit 0"));
        assert!(pre_commit.contains("exit 1"));

        let pre_push = generate_pre_push_hook(&config);
        assert!(pre_push.contains("set -e"));
        assert!(pre_push.contains("command -v repolens"));
        assert!(pre_push.contains("cargo install repolens"));
        assert!(pre_push.contains("exit 0"));
        assert!(pre_push.contains("exit 1"));
    }

    #[cfg(unix)]
    #[test]
    fn test_install_fails_when_hooks_dir_not_creatable() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = TempDir::new().unwrap();
        let git_dir = temp_dir.path().join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        // Make .git read-only so hooks dir can't be created
        fs::set_permissions(&git_dir, fs::Permissions::from_mode(0o444)).unwrap();

        let config = HooksConfig::default();
        let manager = HooksManager::new(temp_dir.path(), config).unwrap();

        let result = manager.install(false);
        assert!(result.is_err());

        // Restore permissions for cleanup
        fs::set_permissions(&git_dir, fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn test_remove_hook_fails_when_file_not_removable() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = create_temp_repo();
        let hooks_dir = temp_dir.path().join(".git/hooks");
        let config = HooksConfig::default();
        let manager = HooksManager::new(temp_dir.path(), config).unwrap();

        // Install hooks first
        manager.install(false).unwrap();

        // Make hooks directory read+execute only (can read files but can't delete)
        fs::set_permissions(&hooks_dir, fs::Permissions::from_mode(0o555)).unwrap();

        let result = manager.remove();
        assert!(result.is_err());

        // Restore permissions for cleanup
        fs::set_permissions(&hooks_dir, fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn test_install_force_fails_when_backup_not_possible() {
        use std::os::unix::fs::PermissionsExt;

        let temp_dir = create_temp_repo();
        let hooks_dir = temp_dir.path().join(".git/hooks");
        let hook_path = hooks_dir.join("pre-commit");

        // Create an existing non-RepoLens hook
        fs::write(&hook_path, "#!/bin/sh\necho 'existing'").unwrap();

        // Make the hook file writable but the directory read-only + execute only
        // This prevents creating new files (backup) but allows reading existing ones
        fs::set_permissions(&hooks_dir, fs::Permissions::from_mode(0o555)).unwrap();

        let config = HooksConfig {
            pre_commit: true,
            pre_push: false,
            fail_on_warnings: false,
        };
        let manager = HooksManager::new(temp_dir.path(), config).unwrap();

        let result = manager.install(true);
        assert!(result.is_err());

        // Restore permissions for cleanup
        fs::set_permissions(&hooks_dir, fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[test]
    fn test_remove_all_hooks_with_mixed_state() {
        let temp_dir = create_temp_repo();

        // Install only pre-commit as a RepoLens hook
        let config = HooksConfig {
            pre_commit: true,
            pre_push: false,
            fail_on_warnings: false,
        };
        let manager = HooksManager::new(temp_dir.path(), config).unwrap();
        manager.install(false).unwrap();

        // Add a non-RepoLens pre-push hook
        let pre_push_path = temp_dir.path().join(".git/hooks/pre-push");
        fs::write(&pre_push_path, "#!/bin/sh\necho 'custom push hook'").unwrap();

        // Remove all - should remove pre-commit but skip pre-push
        let full_config = HooksConfig::default();
        let full_manager = HooksManager::new(temp_dir.path(), full_config).unwrap();
        let messages = full_manager.remove().unwrap();

        assert!(messages[0].contains("Removed pre-commit"));
        assert!(messages[1].contains("Skipped"));

        // pre-commit should be gone, pre-push should remain
        assert!(!temp_dir.path().join(".git/hooks/pre-commit").exists());
        assert!(pre_push_path.exists());
    }
}
