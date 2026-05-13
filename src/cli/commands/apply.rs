//! Apply command - Apply planned changes to the repository

use colored::Colorize;
use console::Term;
use dialoguer::{Confirm, MultiSelect};
use indicatif::{ProgressBar, ProgressStyle};
use similar::{ChangeTag, TextDiff};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::ApplyArgs;
use crate::actions::executor::ActionExecutor;
use crate::actions::git;
use crate::actions::plan::{Action, ActionOperation, ActionPlan};
use crate::actions::planner::ActionPlanner;
use crate::config::Config;
use crate::error::RepoLensError;
use crate::exit_codes;
use crate::providers::github::GitHubProvider;
use crate::rules::engine::RulesEngine;
use crate::rules::results::{AuditResults, Severity};
use crate::scanner::Scanner;

/// Display a visual summary of the actions to be applied
fn display_action_summary(actions: &[Action], audit_results: &AuditResults) {
    let term_width = Term::stdout().size().1 as usize;
    let separator = "=".repeat(term_width.min(80));

    println!();
    println!("{}", separator.dimmed());
    println!(
        "{}",
        "                     ACTION SUMMARY                     "
            .bold()
            .cyan()
    );
    println!("{}", separator.dimmed());
    println!();

    // Group actions by category
    let mut categories: HashMap<&str, Vec<&Action>> = HashMap::new();
    for action in actions {
        categories
            .entry(action.category())
            .or_default()
            .push(action);
    }

    // Display actions grouped by category
    for (category, category_actions) in &categories {
        let category_icon = get_category_icon(category);
        println!(
            "{} {} {}",
            category_icon,
            category.to_uppercase().bold(),
            format!(
                "({} action{})",
                category_actions.len(),
                if category_actions.len() > 1 { "s" } else { "" }
            )
            .dimmed()
        );

        for action in category_actions {
            println!("    {} {}", "+".green(), action.description());
            for detail in action.details() {
                println!("      {} {}", "-".dimmed(), detail.dimmed());
            }
        }
        println!();
    }

    // Display warning issue creation preview
    let mut warning_categories: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    for finding in audit_results.findings_by_severity(Severity::Warning) {
        *warning_categories
            .entry(finding.category.clone())
            .or_insert(0) += 1;
    }

    if !warning_categories.is_empty() {
        let total_warnings: usize = warning_categories.values().sum();
        println!(
            "[I] {} {}",
            "ISSUES".bold(),
            format!(
                "({} warning{} → GitHub issues)",
                total_warnings,
                if total_warnings > 1 { "s" } else { "" }
            )
            .dimmed()
        );
        for (category, count) in &warning_categories {
            println!(
                "    {} {} ({} warning{})",
                "+".green(),
                category,
                count,
                if *count > 1 { "s" } else { "" }
            );
        }
        println!();
    }

    let total = actions.len()
        + if warning_categories.is_empty() {
            0
        } else {
            warning_categories.len()
        };
    println!("{}", separator.dimmed());
    println!(
        "  {} {} action{} to apply",
        "Total:".bold(),
        total.to_string().cyan().bold(),
        if total > 1 { "s" } else { "" }
    );
    println!("{}", separator.dimmed());
    println!();
}

/// Get an icon for each category
fn get_category_icon(category: &str) -> &'static str {
    match category.to_lowercase().as_str() {
        "gitignore" | "files" => "[F]",
        "security" => "[S]",
        "github" => "[G]",
        "docs" | "documentation" => "[D]",
        "workflows" => "[W]",
        "quality" => "[Q]",
        _ => "[*]",
    }
}

/// Display a diff between old and new content
fn display_diff(old_content: &str, new_content: &str, filename: &str) {
    let term_width = Term::stdout().size().1 as usize;
    let separator = "-".repeat(term_width.min(60));

    println!();
    println!("  {} {}", "Diff for:".dimmed(), filename.cyan().bold());
    println!("  {}", separator.dimmed());

    let diff = TextDiff::from_lines(old_content, new_content);

    for change in diff.iter_all_changes() {
        let line = change.value().trim_end_matches('\n');
        match change.tag() {
            ChangeTag::Delete => {
                println!("  {} {}", "-".red().bold(), line.red());
            }
            ChangeTag::Insert => {
                println!("  {} {}", "+".green().bold(), line.green());
            }
            ChangeTag::Equal => {
                println!("  {} {}", " ".dimmed(), line.dimmed());
            }
        }
    }

    println!("  {}", separator.dimmed());
    println!();
}

/// Preview what an action will change (before/after diff)
fn preview_action_diff(action: &Action) {
    match action.operation() {
        ActionOperation::UpdateGitignore { entries } => {
            let gitignore_path = Path::new(".gitignore");
            let old_content = if gitignore_path.exists() {
                fs::read_to_string(gitignore_path).unwrap_or_default()
            } else {
                String::new()
            };

            let mut new_content = old_content.clone();
            if !new_content.is_empty() && !new_content.ends_with('\n') {
                new_content.push('\n');
            }
            if !old_content.is_empty() {
                new_content.push_str("\n# Added by repolens\n");
            }
            for entry in entries {
                // Check if entry already exists
                if !old_content.lines().any(|l| l.trim() == entry.trim()) {
                    new_content.push_str(entry);
                    new_content.push('\n');
                }
            }

            display_diff(&old_content, &new_content, ".gitignore");
        }
        ActionOperation::CreateFile {
            path,
            template,
            variables,
        } => {
            let file_path = Path::new(path);
            let old_content = if file_path.exists() {
                fs::read_to_string(file_path).unwrap_or_default()
            } else {
                "(file does not exist)".to_string()
            };

            // Generate a preview of the new content
            let new_content = format!(
                "(New file from template: {})\n\n{}",
                template,
                if variables.is_empty() {
                    "(No variable substitutions)".to_string()
                } else {
                    variables
                        .iter()
                        .map(|(k, v)| format!("  {} = {}", k, v))
                        .collect::<Vec<_>>()
                        .join("\n")
                }
            );

            display_diff(&old_content, &new_content, path);
        }
        ActionOperation::ConfigureBranchProtection { branch, settings } => {
            let old_content = "(Current branch protection settings)".to_string();
            let new_content = format!(
                "Branch: {}\n\
                 Required approvals: {}\n\
                 Require status checks: {}\n\
                 Require conversation resolution: {}\n\
                 Require linear history: {}\n\
                 Block force push: {}\n\
                 Block deletions: {}\n\
                 Enforce for admins: {}\n\
                 Require signed commits: {}",
                branch,
                settings.required_approvals,
                settings.require_status_checks,
                settings.require_conversation_resolution,
                settings.require_linear_history,
                settings.block_force_push,
                settings.block_deletions,
                settings.enforce_admins,
                settings.require_signed_commits
            );

            display_diff(
                &old_content,
                &new_content,
                &format!("Branch protection: {}", branch),
            );
        }
        ActionOperation::UpdateGitHubSettings { settings } => {
            let old_content = "(Current repository settings)".to_string();
            let mut changes = Vec::new();
            if let Some(v) = settings.enable_discussions {
                changes.push(format!("Enable discussions: {}", v));
            }
            if let Some(v) = settings.enable_issues {
                changes.push(format!("Enable issues: {}", v));
            }
            if let Some(v) = settings.enable_wiki {
                changes.push(format!("Enable wiki: {}", v));
            }
            if let Some(v) = settings.enable_vulnerability_alerts {
                changes.push(format!("Enable vulnerability alerts: {}", v));
            }
            if let Some(v) = settings.enable_automated_security_fixes {
                changes.push(format!("Enable automated security fixes: {}", v));
            }
            let new_content = if changes.is_empty() {
                "(No changes)".to_string()
            } else {
                changes.join("\n")
            };

            display_diff(&old_content, &new_content, "GitHub repository settings");
        }
    }
}

/// Run interactive mode for action selection
fn run_interactive_selection(actions: &[Action]) -> Result<Vec<usize>, RepoLensError> {
    let items: Vec<String> = actions
        .iter()
        .map(|a| format!("[{}] {}", a.category(), a.description()))
        .collect();

    // Default all items to selected
    let defaults: Vec<bool> = vec![true; items.len()];

    println!();
    println!(
        "{}",
        "Select the actions to apply (Space to toggle, Enter to confirm):".bold()
    );
    println!();

    let selections = MultiSelect::new()
        .items(&items)
        .defaults(&defaults)
        .interact()
        .map_err(|e| {
            RepoLensError::Action(crate::error::ActionError::ExecutionFailed {
                message: format!("Failed to get user selection: {}", e),
            })
        })?;

    Ok(selections)
}

/// Show diff preview for selected actions
fn show_diff_previews(actions: &[Action], selected_indices: &[usize]) -> Result<(), RepoLensError> {
    println!();
    println!("{}", "Preview of changes:".bold().cyan());

    for &idx in selected_indices {
        if let Some(action) = actions.get(idx) {
            preview_action_diff(action);
        }
    }

    Ok(())
}

/// Create a progress bar for multiple actions
fn create_progress_bar(total: u64) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.enable_steady_tick(Duration::from_millis(100));
    pb
}

/// Create a spinner for individual actions
fn create_spinner(message: &str) -> ProgressBar {
    let sp = ProgressBar::new_spinner();
    sp.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    sp.set_message(message.to_string());
    sp.enable_steady_tick(Duration::from_millis(80));
    sp
}

pub async fn execute(args: ApplyArgs) -> Result<i32, RepoLensError> {
    // Load configuration
    let config = Config::load_or_default()?;

    // Initialize scanner
    let scanner = Scanner::new(PathBuf::from("."));

    // Run the rules engine to get current state
    let engine = RulesEngine::new(config.clone());
    let audit_results = engine.run(&scanner).await?;

    // Generate action plan
    let planner = ActionPlanner::new(config.clone());
    let mut action_plan = planner.create_plan(&audit_results).await?;

    // Apply filters if specified
    if let Some(only) = &args.only {
        action_plan.filter_only(only);
    }
    if let Some(skip) = &args.skip {
        action_plan.filter_skip(skip);
    }

    // Check if there are any actions or warning issues to create
    let has_warnings = audit_results.has_warnings();
    if action_plan.is_empty() && (!has_warnings || args.no_issues) {
        println!("{}", "No actions to perform.".green());
        return Ok(exit_codes::SUCCESS);
    }

    // If only warning issues to create (no plan actions), handle that directly
    if action_plan.is_empty() && has_warnings && !args.no_issues {
        create_warning_issues(&audit_results);
        return Ok(exit_codes::SUCCESS);
    }

    // Display visual summary
    display_action_summary(action_plan.actions(), &audit_results);

    // Dry run mode
    if args.dry_run {
        println!("{}", "Dry run mode - no changes made.".yellow());

        // Show what would happen
        println!();
        println!("{}", "Preview of changes that would be made:".bold());
        for action in action_plan.actions() {
            preview_action_diff(action);
        }

        return Ok(exit_codes::SUCCESS);
    }

    // Determine which actions to execute based on mode
    let actions_to_execute: Vec<&Action>;

    if args.interactive {
        // Interactive mode: let user select actions
        let selected_indices = run_interactive_selection(action_plan.actions())?;

        if selected_indices.is_empty() {
            println!("{}", "No actions selected.".yellow());
            return Ok(exit_codes::SUCCESS);
        }

        // Show diff previews for selected actions
        show_diff_previews(action_plan.actions(), &selected_indices)?;

        // Confirm after seeing diffs
        let confirm = Confirm::new()
            .with_prompt(format!(
                "Apply these {} selected action{}?",
                selected_indices.len(),
                if selected_indices.len() > 1 { "s" } else { "" }
            ))
            .default(false)
            .interact()
            .map_err(|e| {
                RepoLensError::Action(crate::error::ActionError::ExecutionFailed {
                    message: format!("Failed to get user input: {}", e),
                })
            })?;

        if !confirm {
            println!("{}", "Aborted.".yellow());
            return Ok(exit_codes::SUCCESS);
        }

        actions_to_execute = selected_indices
            .iter()
            .filter_map(|&i| action_plan.actions().get(i))
            .collect();
    } else if args.yes {
        // Auto-accept mode: execute all actions without confirmation
        actions_to_execute = action_plan.actions().iter().collect();
    } else {
        // Standard mode: confirm before execution
        let confirm = Confirm::new()
            .with_prompt("Apply these changes?")
            .default(false)
            .interact()
            .map_err(|e| {
                RepoLensError::Action(crate::error::ActionError::ExecutionFailed {
                    message: format!("Failed to get user input: {}", e),
                })
            })?;

        if !confirm {
            println!("{}", "Aborted.".yellow());
            return Ok(exit_codes::SUCCESS);
        }

        actions_to_execute = action_plan.actions().iter().collect();
    }

    // Create a filtered action plan with only selected actions
    let mut filtered_plan = ActionPlan::new();
    for action in actions_to_execute {
        filtered_plan.add(action.clone());
    }

    // Execute actions with progress bar
    let executor = ActionExecutor::new(config);

    println!();
    println!("{}", "Executing actions...".bold());
    println!();

    let progress_bar = create_progress_bar(filtered_plan.actions().len() as u64);
    let mut results = Vec::new();

    for action in filtered_plan.actions() {
        let spinner = create_spinner(action.description());

        // Execute the single action
        let single_plan = {
            let mut p = ActionPlan::new();
            p.add(action.clone());
            p
        };

        let action_results = executor.execute(&single_plan).await?;
        spinner.finish_and_clear();

        if let Some(result) = action_results.into_iter().next() {
            if result.success {
                progress_bar.println(format!(
                    "  {} {}",
                    "[OK]".green().bold(),
                    action.description()
                ));
            } else {
                progress_bar.println(format!(
                    "  {} {} - {}",
                    "[FAIL]".red().bold(),
                    action.description(),
                    result.error.as_deref().unwrap_or("Unknown error")
                ));
            }
            results.push(result);
        }

        progress_bar.inc(1);
    }

    progress_bar.finish_and_clear();

    // Display results summary
    println!();
    let mut success_count = 0;
    let mut error_count = 0;

    for result in &results {
        if result.success {
            success_count += 1;
        } else {
            error_count += 1;
        }
    }

    // Final summary
    let term_width = Term::stdout().size().1 as usize;
    let separator = "=".repeat(term_width.min(80));

    println!("{}", separator.dimmed());
    println!(
        "{}",
        "                     EXECUTION SUMMARY                     "
            .bold()
            .cyan()
    );
    println!("{}", separator.dimmed());
    println!();
    println!(
        "  {} {} succeeded",
        "[OK]".green().bold(),
        success_count.to_string().green().bold()
    );
    println!(
        "  {} {} failed",
        "[FAIL]".red().bold(),
        error_count.to_string().red().bold()
    );
    println!();
    println!("{}", separator.dimmed());

    // Create GitHub issues for warning findings (unless --no-issues is set)
    if !args.no_issues {
        create_warning_issues(&audit_results);
    }

    // Handle git operations and PR creation if there were successful file changes
    if success_count > 0 {
        let repo_root = PathBuf::from(".");
        let should_create_pr = if args.no_pr {
            false
        } else {
            args.create_pr
                .unwrap_or_else(|| git::is_git_repository(&repo_root))
        };

        if should_create_pr && git::is_git_repository(&repo_root) {
            if let Err(e) = handle_git_operations(&repo_root, &filtered_plan, &results).await {
                eprintln!(
                    "{} {}",
                    "[WARN]".yellow().bold(),
                    format!("Failed to create PR: {}", e).yellow()
                );
                // Don't fail the whole command if PR creation fails
            }
        }
    }

    // Determine exit code based on results:
    // - All actions succeeded: SUCCESS (0)
    // - Some actions failed: WARNINGS (2)
    // - All actions failed: CRITICAL_ISSUES (1)
    let exit_code = if error_count == 0 {
        exit_codes::SUCCESS
    } else if success_count == 0 {
        // All actions failed
        exit_codes::CRITICAL_ISSUES
    } else {
        // Some actions failed (partial success)
        exit_codes::WARNINGS
    };

    Ok(exit_code)
}

/// Create GitHub issues for warning findings grouped by category
fn create_warning_issues(audit_results: &AuditResults) {
    // Check if GitHub CLI is available
    if !GitHubProvider::is_available() {
        println!(
            "{} {}",
            "[WARN]".yellow().bold(),
            "GitHub CLI not available, skipping issue creation.".yellow()
        );
        return;
    }

    let github_provider = match GitHubProvider::new() {
        Ok(provider) => provider,
        Err(e) => {
            println!(
                "{} {}",
                "[WARN]".yellow().bold(),
                format!("Unable to create issues: {}. Skipping.", e).yellow()
            );
            return;
        }
    };

    // Group warnings by category
    let mut warning_categories: HashMap<String, Vec<&crate::rules::results::Finding>> =
        HashMap::new();
    for finding in audit_results.findings_by_severity(Severity::Warning) {
        warning_categories
            .entry(finding.category.clone())
            .or_default()
            .push(finding);
    }

    if warning_categories.is_empty() {
        return;
    }

    let term_width = Term::stdout().size().1 as usize;
    let separator = "=".repeat(term_width.min(50));

    println!();
    println!("{}", separator.dimmed());
    println!("{}", "  ISSUES CREATED".bold().cyan());
    println!();

    let mut sorted_categories: Vec<_> = warning_categories.into_iter().collect();
    sorted_categories.sort_by(|a, b| a.0.cmp(&b.0));

    for (category, findings) in &sorted_categories {
        let count = findings.len();

        let title = format!("[RepoLens] {} warning(s) -- {}", count, category);

        // Build markdown table body
        let mut body =
            String::from("| Rule ID | Message | Location |\n|---------|---------|----------|\n");
        for finding in findings {
            let location = finding.location.as_deref().unwrap_or("-");
            body.push_str(&format!(
                "| {} | {} | {} |\n",
                finding.rule_id, finding.message, location
            ));
        }

        let labels = vec!["repolens-audit"];

        match github_provider.create_issue(&title, &body, &labels) {
            Ok(url) => {
                println!(
                    "  {} {} ({} warning{}) -> {}",
                    "[OK]".green().bold(),
                    category,
                    count,
                    if count > 1 { "s" } else { "" },
                    url.cyan()
                );
            }
            Err(e) => {
                println!(
                    "  {} {} -- {}",
                    "[FAIL]".red().bold(),
                    category,
                    format!("Failed to create issue: {}", e).red()
                );
            }
        }
    }

    println!();
    println!("{}", separator.dimmed());
}

/// Handle git operations: create branch, commit, push, and create PR
async fn handle_git_operations(
    repo_root: &Path,
    action_plan: &ActionPlan,
    results: &[crate::actions::executor::ActionResult],
) -> Result<(), RepoLensError> {
    // Check if there are any file-related changes by checking the action plan
    let has_file_changes = action_plan.actions().iter().any(|action| {
        matches!(
            action.operation(),
            ActionOperation::CreateFile { .. } | ActionOperation::UpdateGitignore { .. }
        )
    });

    if !has_file_changes {
        // Only file changes trigger PR creation
        return Ok(());
    }

    // Check if there are actual changes to commit
    if !git::has_changes(repo_root)? {
        println!(
            "{}",
            "No file changes detected, skipping PR creation.".dimmed()
        );
        return Ok(());
    }

    println!();
    println!("{}", "Creating branch and preparing PR...".dimmed());

    // Create new branch
    let branch_name = git::create_branch(repo_root)?;
    println!(
        "  {} Branch created: {}",
        "[OK]".green().bold(),
        branch_name.cyan()
    );

    // Collect file paths from actions that modify files
    let file_paths: Vec<String> = action_plan
        .actions()
        .iter()
        .filter_map(|action| match action.operation() {
            ActionOperation::CreateFile { path, .. } => Some(path.clone()),
            ActionOperation::UpdateGitignore { .. } => Some(".gitignore".to_string()),
            _ => None,
        })
        .collect();

    // Stage only the specific files changed by actions
    git::stage_files(repo_root, &file_paths)?;
    println!("  {} Changes staged", "[OK]".green().bold());

    // Create commit message
    let commit_message = format!(
        "chore: apply RepoLens fixes\n\n{}\n\nActions applied:\n{}",
        "This commit contains automatic fixes applied by RepoLens.",
        action_plan
            .actions()
            .iter()
            .map(|a| format!("- {}", a.description()))
            .collect::<Vec<_>>()
            .join("\n")
    );

    // Create commit
    git::create_commit(repo_root, &commit_message)?;
    println!("  {} Commit created", "[OK]".green().bold());

    // Push branch
    git::push_branch(repo_root, &branch_name)?;
    println!("  {} Branch pushed to origin", "[OK]".green().bold());

    // Create PR - check if GitHub CLI is available first
    if !GitHubProvider::is_available() {
        println!(
            "{} {}",
            "[WARN]".yellow().bold(),
            "GitHub CLI not available, PR not created. Changes are in the local branch.".yellow()
        );
        return Ok(());
    }

    let github_provider = match GitHubProvider::new() {
        Ok(provider) => provider,
        Err(e) => {
            println!(
                "{} {}",
                "[WARN]".yellow().bold(),
                format!(
                    "Unable to create PR: {}. Changes are in the local branch.",
                    e
                )
                .yellow()
            );
            return Ok(());
        }
    };

    let default_branch = git::get_default_branch(repo_root).unwrap_or_else(|| "main".to_string());

    let pr_title = format!(
        "RepoLens: Automatic fixes ({})",
        chrono::Local::now().format("%Y-%m-%d %H:%M")
    );

    let pr_body = format!(
        "# RepoLens Automatic Fixes\n\n\
        This PR contains automatic fixes applied by RepoLens.\n\n\
        ## Actions Applied\n\n\
        {}\n\n\
        ## Details\n\n\
        {}",
        action_plan
            .actions()
            .iter()
            .map(|a| format!("- **{}**: {}", a.category(), a.description()))
            .collect::<Vec<_>>()
            .join("\n"),
        results
            .iter()
            .filter(|r| r.success)
            .map(|r| format!("- [OK] {}", r.action_name))
            .collect::<Vec<_>>()
            .join("\n")
    );

    let pr_url = github_provider.create_pull_request(
        &pr_title,
        &pr_body,
        &branch_name,
        Some(&default_branch),
    )?;

    println!();
    println!(
        "{} {}",
        "[OK]".green().bold(),
        format!("Pull Request created: {}", pr_url.cyan()).green()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::plan::{
        Action, ActionOperation, BranchProtectionSettings, GitHubRepoSettings,
    };
    use std::collections::HashMap;

    #[test]
    fn test_get_category_icon() {
        assert_eq!(get_category_icon("gitignore"), "[F]");
        assert_eq!(get_category_icon("files"), "[F]");
        assert_eq!(get_category_icon("security"), "[S]");
        assert_eq!(get_category_icon("github"), "[G]");
        assert_eq!(get_category_icon("docs"), "[D]");
        assert_eq!(get_category_icon("documentation"), "[D]");
        assert_eq!(get_category_icon("workflows"), "[W]");
        assert_eq!(get_category_icon("quality"), "[Q]");
        assert_eq!(get_category_icon("unknown"), "[*]");
    }

    #[test]
    fn test_display_diff_additions() {
        // This test verifies that the diff display function runs without panic
        // Visual output is not directly testable, but we ensure no errors occur
        let old = "line1\nline2\n";
        let new = "line1\nline2\nline3\n";
        display_diff(old, new, "test.txt");
    }

    #[test]
    fn test_display_diff_deletions() {
        let old = "line1\nline2\nline3\n";
        let new = "line1\nline3\n";
        display_diff(old, new, "test.txt");
    }

    #[test]
    fn test_display_diff_modifications() {
        let old = "line1\nold_line\nline3\n";
        let new = "line1\nnew_line\nline3\n";
        display_diff(old, new, "test.txt");
    }

    #[test]
    fn test_display_action_summary() {
        let actions = vec![
            Action::new(
                "test-1",
                "gitignore",
                "Update .gitignore",
                ActionOperation::UpdateGitignore {
                    entries: vec![".env".to_string()],
                },
            ),
            Action::new(
                "test-2",
                "files",
                "Create README.md",
                ActionOperation::CreateFile {
                    path: "README.md".to_string(),
                    template: "README".to_string(),
                    variables: HashMap::new(),
                },
            ),
        ];

        let results = AuditResults::new("test-repo", "opensource");

        // This test verifies the summary display function runs without panic
        display_action_summary(&actions, &results);
    }

    #[test]
    fn test_preview_action_diff_gitignore() {
        let action = Action::new(
            "test-gitignore",
            "gitignore",
            "Update .gitignore",
            ActionOperation::UpdateGitignore {
                entries: vec![".env".to_string(), "*.key".to_string()],
            },
        );

        // This test verifies the preview function runs without panic
        preview_action_diff(&action);
    }

    #[test]
    fn test_preview_action_diff_create_file() {
        let mut variables = HashMap::new();
        variables.insert("author".to_string(), "Test Author".to_string());

        let action = Action::new(
            "test-file",
            "files",
            "Create LICENSE",
            ActionOperation::CreateFile {
                path: "LICENSE".to_string(),
                template: "LICENSE/MIT".to_string(),
                variables,
            },
        );

        preview_action_diff(&action);
    }

    #[test]
    fn test_preview_action_diff_branch_protection() {
        let action = Action::new(
            "test-branch",
            "security",
            "Configure branch protection",
            ActionOperation::ConfigureBranchProtection {
                branch: "main".to_string(),
                settings: BranchProtectionSettings::default(),
            },
        );

        preview_action_diff(&action);
    }

    #[test]
    fn test_preview_action_diff_github_settings() {
        let action = Action::new(
            "test-github",
            "github",
            "Update GitHub settings",
            ActionOperation::UpdateGitHubSettings {
                settings: GitHubRepoSettings {
                    enable_discussions: Some(true),
                    enable_issues: Some(true),
                    enable_wiki: Some(false),
                    enable_vulnerability_alerts: Some(true),
                    enable_automated_security_fixes: Some(true),
                },
            },
        );

        preview_action_diff(&action);
    }

    #[test]
    fn test_create_progress_bar() {
        let pb = create_progress_bar(10);
        pb.inc(1);
        pb.finish_and_clear();
    }

    #[test]
    fn test_create_spinner() {
        let sp = create_spinner("Testing...");
        sp.finish_and_clear();
    }
}
