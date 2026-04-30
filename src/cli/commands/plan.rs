//! Plan command - Analyze repository and show planned actions
//!
//! This module implements the `plan` command which analyzes a repository
//! and generates an action plan to fix detected issues.

use std::path::PathBuf;

use super::{OutputFormat, PlanArgs};
use crate::actions::planner::ActionPlanner;
use crate::cache::{delete_cache_directory, AuditCache};
use crate::cli::output::{
    CsvOutput, JsonOutput, JunitReport, NdjsonOutput, OutputRenderer, SarifOutput, TerminalOutput,
};
use crate::config::Config;
use crate::error::RepoLensError;
use crate::exit_codes;
use crate::rules::engine::RulesEngine;
use crate::rules::filter_valid_categories;
use crate::scanner::Scanner;
use crate::utils::format_duration;
use colored::Colorize;
use std::time::Duration;

/// Execute the plan command
///
/// Analyzes the repository, runs audit rules, generates an action plan,
/// and outputs the results in the requested format.
///
/// # Arguments
///
/// * `args` - Command line arguments for the plan command
///
/// # Returns
///
/// An exit code: 0 for success, 1 for critical issues, 2 for warnings
///
/// # Errors
///
/// Returns an error if the audit or plan generation fails
pub async fn execute(args: PlanArgs) -> Result<i32, RepoLensError> {
    eprintln!("{}", "Chargement de la configuration...".dimmed());
    // Load configuration
    let mut config = Config::load_or_default()?;

    // Handle cache directory override from CLI
    if let Some(ref cache_dir) = args.cache_dir {
        config.cache.directory = cache_dir.display().to_string();
    }

    // Disable cache if --no-cache is specified
    if args.no_cache {
        config.cache.enabled = false;
        eprintln!("{}", "Cache disabled.".dimmed());
    }

    // Clear cache if --clear-cache is specified
    let project_root = PathBuf::from(".");
    if args.clear_cache {
        eprintln!("{}", "Clearing cache...".dimmed());
        if let Err(e) = delete_cache_directory(&project_root, &config.cache) {
            eprintln!("{} Failed to clear cache: {}", "Warning:".yellow(), e);
        } else {
            eprintln!("{} {}", "✓".green(), "Cache cleared.".green());
        }
    }

    // Load or create cache
    let cache = if config.cache.enabled {
        let cache = AuditCache::load(&project_root, config.cache.clone());
        let stats = cache.stats();
        if stats.total_entries > 0 {
            eprintln!(
                "{} {} {}",
                "Cache:".dimmed(),
                stats.total_entries.to_string().cyan(),
                "entries loaded.".dimmed()
            );
        }
        Some(cache)
    } else {
        None
    };

    eprintln!("{}", "Analyse du dépôt...".dimmed());
    // Initialize scanner
    let scanner = Scanner::new(PathBuf::from("."));

    // Run the rules engine
    let mut engine = RulesEngine::new(config.clone());

    // Set cache in the engine
    if let Some(c) = cache {
        engine.set_cache(c);
    }

    // Apply filters if specified (with validation)
    if let Some(only) = args.only {
        let valid_only = filter_valid_categories(only);
        if !valid_only.is_empty() {
            engine.set_only_categories(valid_only);
        }
    }
    if let Some(skip) = args.skip {
        let valid_skip = filter_valid_categories(skip);
        if !valid_skip.is_empty() {
            engine.set_skip_categories(valid_skip);
        }
    }

    // Capture verbosity for progress callback
    let verbose = args.verbose;

    // Set up progress callback with timing info
    engine.set_progress_callback(Box::new(move |category_name, current, total, timing| {
        if let Some((findings_count, duration_ms)) = timing {
            // After execution - show timing info based on verbosity
            if verbose >= 1 {
                let duration = Duration::from_millis(duration_ms);
                let duration_str = format_duration(duration);
                eprintln!(
                    "  {} {} ({}/{}) - {} findings ({})",
                    "✓".green(),
                    category_name.cyan(),
                    current,
                    total,
                    findings_count,
                    duration_str.dimmed()
                );
            }
        } else {
            // Before execution
            if verbose == 0 {
                eprintln!(
                    "  {} {} ({}/{})...",
                    "→".dimmed(),
                    category_name.cyan(),
                    current,
                    total
                );
            }
        }
    }));

    // Execute audit with timing
    eprintln!("{}", "Exécution de l'audit...".dimmed());
    let (audit_results, timing) = engine.run_with_timing(&scanner).await?;

    // Save cache if enabled
    if let Some(cache) = engine.take_cache() {
        if let Err(e) = cache.save() {
            eprintln!("{} Failed to save cache: {}", "Warning:".yellow(), e);
        }
    }

    // Show completion with total timing
    eprintln!(
        "{} {} ({})",
        "✓".green(),
        "Audit terminé.".green(),
        timing.total_duration_formatted().dimmed()
    );

    // In verbose mode, show category timing summary
    if verbose >= 2 {
        eprintln!("\n{}", "Timing breakdown:".dimmed());
        for cat_timing in timing.categories() {
            eprintln!(
                "  {} {}: {} findings ({})",
                "•".dimmed(),
                cat_timing.name.cyan(),
                cat_timing.findings_count,
                cat_timing.duration_formatted().dimmed()
            );
        }
        eprintln!();
    }

    eprintln!("{}", "Génération du plan d'action...".dimmed());
    // Generate action plan
    let planner = ActionPlanner::new(config);
    let action_plan = planner.create_plan(&audit_results).await?;

    eprintln!("{}", "Génération du rapport...".dimmed());

    // Warn if CSV-only flags are set when format is not CSV/TSV.
    let format_is_csv_like = matches!(args.format, OutputFormat::Csv | OutputFormat::Tsv);
    if !format_is_csv_like && (args.csv_bom || args.csv_keep_newlines || args.csv_delimiter != ',')
    {
        eprintln!("[WARN] --csv-* flags are only meaningful with --format csv|tsv; ignoring.");
    }

    // Render output
    let output: Box<dyn OutputRenderer> = match args.format {
        OutputFormat::Terminal => Box::new(TerminalOutput::new()),
        OutputFormat::Json => Box::new(JsonOutput::new()),
        OutputFormat::Sarif => Box::new(SarifOutput::new()),
        OutputFormat::Csv => Box::new(
            CsvOutput::new()
                .with_delimiter(args.csv_delimiter as u8)
                .with_bom(args.csv_bom)
                .with_keep_newlines(args.csv_keep_newlines),
        ),
        OutputFormat::Tsv => Box::new(
            CsvOutput::new()
                .with_delimiter(b'\t')
                .with_bom(args.csv_bom)
                .with_keep_newlines(args.csv_keep_newlines),
        ),
        OutputFormat::Ndjson => Box::new(NdjsonOutput::new()),
        OutputFormat::Junit => Box::new(JunitReport::new()),
    };

    let rendered = output.render_plan(&audit_results, &action_plan)?;

    // Write output
    if let Some(output_path) = args.output {
        std::fs::write(&output_path, &rendered).map_err(|e| {
            RepoLensError::Action(crate::error::ActionError::FileWrite {
                path: output_path.display().to_string(),
                source: e,
            })
        })?;
        eprintln!("Plan written to: {}", output_path.display());
    } else {
        println!("{rendered}");
    }

    // Determine exit code based on findings
    let exit_code = if audit_results.has_critical() {
        exit_codes::CRITICAL_ISSUES
    } else if audit_results.has_warnings() {
        exit_codes::WARNINGS
    } else {
        exit_codes::SUCCESS
    };

    Ok(exit_code)
}
