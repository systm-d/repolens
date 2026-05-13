//! Init command - Initialize a new configuration file

use colored::Colorize;
use dialoguer::{Confirm, Select};
use std::fs;
use std::path::Path;

use super::InitArgs;
use crate::config::presets::VALID_PRESETS;
use crate::config::{Config, Preset};
use crate::error::{ActionError, RepoLensError};
use crate::exit_codes;
use crate::utils::permissions::set_secure_permissions;
use crate::utils::prerequisites::{
    CheckOptions, display_error_summary, display_report, display_warnings, run_all_checks,
};

const CONFIG_FILENAME: &str = ".repolens.toml";

pub async fn execute(args: InitArgs) -> Result<i32, RepoLensError> {
    let root = std::env::current_dir().map_err(|e| {
        RepoLensError::Action(ActionError::ExecutionFailed {
            message: format!("Failed to get current directory: {}", e),
        })
    })?;
    let config_path = Path::new(CONFIG_FILENAME);

    // Run prerequisite checks unless skipped
    if !args.skip_checks {
        let options = CheckOptions::default();
        let report = run_all_checks(&root, &options);
        display_report(&report, false);

        if !report.all_required_passed() {
            display_error_summary(&report);

            if args.non_interactive {
                return Ok(exit_codes::ERROR);
            }

            // Ask if user wants to continue anyway
            let continue_anyway = Confirm::new()
                .with_prompt("Continue anyway?")
                .default(false)
                .interact()
                .map_err(|e| {
                    RepoLensError::Action(ActionError::ExecutionFailed {
                        message: format!("Failed to get user input: {}", e),
                    })
                })?;

            if !continue_anyway {
                return Ok(exit_codes::ERROR);
            }

            println!();
        } else if report.has_warnings() {
            display_warnings(&report);
        }
    }

    // Check if config already exists
    if config_path.exists() && !args.force {
        if args.non_interactive {
            eprintln!(
                "{} Configuration file already exists. Use --force to overwrite.",
                "Error:".red().bold()
            );
            return Ok(exit_codes::ERROR);
        }

        let overwrite = Confirm::new()
            .with_prompt("Configuration file already exists. Overwrite?")
            .default(false)
            .interact()
            .map_err(|e| {
                RepoLensError::Action(ActionError::ExecutionFailed {
                    message: format!("Failed to get user input: {}", e),
                })
            })?;

        if !overwrite {
            println!("{}", "Aborted.".yellow());
            return Ok(exit_codes::SUCCESS);
        }
    }

    // Determine preset with validation
    let preset = if let Some(preset_name) = args.preset {
        match Preset::from_name(&preset_name) {
            Some(p) => p,
            None => {
                eprintln!(
                    "{} Unknown preset '{}'. Valid presets: {}",
                    "Error:".red().bold(),
                    preset_name,
                    VALID_PRESETS.join(", ")
                );
                return Ok(exit_codes::INVALID_ARGS);
            }
        }
    } else if args.non_interactive {
        Preset::OpenSource
    } else {
        select_preset()?
    };

    // Create configuration
    let config = Config::from_preset(preset);

    // Write configuration file
    let config_content = config.to_toml()?;
    fs::write(config_path, &config_content).map_err(|e| {
        RepoLensError::Action(ActionError::FileWrite {
            path: config_path.display().to_string(),
            source: e,
        })
    })?;

    // Set secure permissions (owner read/write only) on Unix systems
    set_secure_permissions(config_path).map_err(|e| {
        RepoLensError::Action(ActionError::ExecutionFailed {
            message: format!(
                "Failed to set secure permissions on {}: {}",
                CONFIG_FILENAME, e
            ),
        })
    })?;

    println!(
        "{} Created {} with preset '{}'",
        "Success:".green().bold(),
        CONFIG_FILENAME.cyan(),
        preset.name().yellow()
    );

    println!("\nNext steps:");
    println!("  1. Review and customize {}", CONFIG_FILENAME.cyan());
    println!("  2. Run {} to see planned actions", "repolens plan".cyan());
    println!("  3. Run {} to apply changes", "repolens apply".cyan());

    Ok(exit_codes::SUCCESS)
}

fn select_preset() -> Result<Preset, RepoLensError> {
    let presets = [
        (
            "opensource",
            "Open Source - Prepare repository for public release",
        ),
        ("enterprise", "Enterprise - Internal company standards"),
        ("strict", "Strict - Maximum security and compliance checks"),
    ];

    let selection = Select::new()
        .with_prompt("Select a preset")
        .items(&presets.iter().map(|(_, desc)| *desc).collect::<Vec<_>>())
        .default(0)
        .interact()
        .map_err(|e| {
            RepoLensError::Action(ActionError::ExecutionFailed {
                message: format!("Failed to get user input: {}", e),
            })
        })?;

    Ok(match selection {
        0 => Preset::OpenSource,
        1 => Preset::Enterprise,
        2 => Preset::Strict,
        _ => Preset::OpenSource,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::process::Command;
    use tempfile::TempDir;

    fn init_git_repo(root: &Path) {
        Command::new("git")
            .args(["init"])
            .current_dir(root)
            .output()
            .ok();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(root)
            .output()
            .ok();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(root)
            .output()
            .ok();
    }

    #[tokio::test]
    #[serial]
    async fn test_execute_creates_config() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().canonicalize().unwrap();
        init_git_repo(&root);

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&root).unwrap();

        let args = InitArgs {
            preset: Some("opensource".to_string()),
            non_interactive: true,
            force: false,
            skip_checks: true,
        };

        let result = execute(args).await;
        std::env::set_current_dir(&original_dir).unwrap();

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), exit_codes::SUCCESS);
        assert!(root.join(".repolens.toml").exists());
    }

    #[tokio::test]
    #[serial]
    async fn test_execute_with_enterprise_preset() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().canonicalize().unwrap();
        init_git_repo(&root);

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&root).unwrap();

        let args = InitArgs {
            preset: Some("enterprise".to_string()),
            non_interactive: true,
            force: false,
            skip_checks: true,
        };

        let result = execute(args).await;
        std::env::set_current_dir(&original_dir).unwrap();

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), exit_codes::SUCCESS);

        let config_content = fs::read_to_string(root.join(".repolens.toml")).unwrap();
        assert!(config_content.contains("enterprise"));
    }

    #[tokio::test]
    #[serial]
    async fn test_execute_with_strict_preset() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().canonicalize().unwrap();
        init_git_repo(&root);

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&root).unwrap();

        let args = InitArgs {
            preset: Some("strict".to_string()),
            non_interactive: true,
            force: false,
            skip_checks: true,
        };

        let result = execute(args).await;
        std::env::set_current_dir(&original_dir).unwrap();

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), exit_codes::SUCCESS);

        let config_content = fs::read_to_string(root.join(".repolens.toml")).unwrap();
        assert!(config_content.contains("strict"));
    }

    #[tokio::test]
    #[serial]
    async fn test_execute_invalid_preset() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().canonicalize().unwrap();
        init_git_repo(&root);

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&root).unwrap();

        let args = InitArgs {
            preset: Some("invalid_preset".to_string()),
            non_interactive: true,
            force: false,
            skip_checks: true,
        };

        let result = execute(args).await;
        std::env::set_current_dir(&original_dir).unwrap();

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), exit_codes::INVALID_ARGS);
    }

    #[tokio::test]
    #[serial]
    async fn test_execute_config_exists_no_force() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().canonicalize().unwrap();
        init_git_repo(&root);

        // Create existing config
        fs::write(
            root.join(".repolens.toml"),
            "[general]\npreset = \"opensource\"\n",
        )
        .unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&root).unwrap();

        let args = InitArgs {
            preset: Some("strict".to_string()),
            non_interactive: true,
            force: false,
            skip_checks: true,
        };

        let result = execute(args).await;
        std::env::set_current_dir(&original_dir).unwrap();

        // Should return ERROR because config exists and no --force
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), exit_codes::ERROR);

        // Original config should be unchanged
        let config_content = fs::read_to_string(root.join(".repolens.toml")).unwrap();
        assert!(config_content.contains("opensource"));
    }

    #[tokio::test]
    #[serial]
    async fn test_execute_config_exists_with_force() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().canonicalize().unwrap();
        init_git_repo(&root);

        // Create existing config
        fs::write(
            root.join(".repolens.toml"),
            "[general]\npreset = \"opensource\"\n",
        )
        .unwrap();

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&root).unwrap();

        let args = InitArgs {
            preset: Some("strict".to_string()),
            non_interactive: true,
            force: true,
            skip_checks: true,
        };

        let result = execute(args).await;
        std::env::set_current_dir(&original_dir).unwrap();

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), exit_codes::SUCCESS);

        // Config should be overwritten
        let config_content = fs::read_to_string(root.join(".repolens.toml")).unwrap();
        assert!(config_content.contains("strict"));
    }

    #[tokio::test]
    #[serial]
    async fn test_execute_default_preset_non_interactive() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path().canonicalize().unwrap();
        init_git_repo(&root);

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&root).unwrap();

        let args = InitArgs {
            preset: None, // No preset specified
            non_interactive: true,
            force: false,
            skip_checks: true,
        };

        let result = execute(args).await;
        std::env::set_current_dir(&original_dir).unwrap();

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), exit_codes::SUCCESS);

        // Should use opensource as default
        let config_content = fs::read_to_string(root.join(".repolens.toml")).unwrap();
        assert!(config_content.contains("opensource"));
    }

    #[test]
    fn test_preset_from_name() {
        assert!(Preset::from_name("opensource").is_some());
        assert!(Preset::from_name("enterprise").is_some());
        assert!(Preset::from_name("strict").is_some());
        assert!(Preset::from_name("invalid").is_none());
    }

    #[test]
    fn test_config_filename_constant() {
        assert_eq!(CONFIG_FILENAME, ".repolens.toml");
    }
}
