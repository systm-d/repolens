//! Report command - Generate an audit report

use colored::Colorize;
use std::path::PathBuf;
use std::time::Duration;

use super::{ReportArgs, ReportFormat};
use crate::cache::{delete_cache_directory, AuditCache};
use crate::cli::output::{
    CsvOutput, HtmlReport, JsonOutput, MarkdownReport, PdfReport, ReportRenderer,
};
use crate::config::{BrandingConfig, Config};
use crate::error::RepoLensError;
use crate::exit_codes;
use crate::rules::engine::RulesEngine;
use crate::rules::filter_valid_categories;
use crate::scanner::Scanner;
use crate::utils::format_duration;

pub async fn execute(args: ReportArgs) -> Result<i32, RepoLensError> {
    // Load configuration
    let mut config = Config::load_or_default()?;

    // Handle cache directory override from CLI
    if let Some(ref cache_dir) = args.cache_dir {
        config.cache.directory = cache_dir.display().to_string();
    }

    // Disable cache if --no-cache is specified
    if args.no_cache {
        config.cache.enabled = false;
    }

    // Clear cache if --clear-cache is specified
    let project_root = PathBuf::from(".");
    if args.clear_cache {
        if let Err(e) = delete_cache_directory(&project_root, &config.cache) {
            eprintln!("{} Failed to clear cache: {}", "Warning:".yellow(), e);
        }
    }

    // Load or create cache
    let cache = if config.cache.enabled {
        Some(AuditCache::load(&project_root, config.cache.clone()))
    } else {
        None
    };

    // Initialize scanner
    let scanner = Scanner::new(PathBuf::from("."));

    // Run the rules engine
    let mut engine = RulesEngine::new(config);

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
        "Audit completed.".green(),
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

    // Warn if CSV-only flags are set when format is not CSV.
    let format_is_csv_like = matches!(args.format, ReportFormat::Csv);
    if !format_is_csv_like && (args.csv_bom || args.csv_keep_newlines || args.csv_delimiter != ',')
    {
        eprintln!("[WARN] --csv-* flags are only meaningful with --format csv; ignoring.");
    }

    // Warn early if --branding was passed for a format other than PDF.
    if args.branding.is_some() && args.format != ReportFormat::Pdf {
        tracing::warn!(
            "--branding is only applied to --format pdf; ignoring for --format {:?}",
            args.format
        );
    }

    let output_path = args.output.clone().unwrap_or_else(|| {
        let extension = match args.format {
            ReportFormat::Html => "html",
            ReportFormat::Markdown => "md",
            ReportFormat::Json => "json",
            ReportFormat::Csv => "csv",
            ReportFormat::Pdf => "pdf",
        };
        PathBuf::from(format!("repolens-report.{extension}"))
    });

    if matches!(args.format, ReportFormat::Pdf) {
        let mut renderer = PdfReport::new(args.detailed);
        if let Some(ref branding_path) = args.branding {
            match BrandingConfig::load_from_file(branding_path) {
                Ok(cfg) => renderer = renderer.with_branding(cfg),
                Err(e) => {
                    eprintln!(
                        "{} failed to load branding '{}': {} — using defaults",
                        "Warning:".yellow(),
                        branding_path.display(),
                        e
                    );
                }
            }
        }
        renderer.render_to_file(&audit_results, &output_path)?;
    } else {
        let renderer: Box<dyn ReportRenderer> = match args.format {
            ReportFormat::Html => Box::new(HtmlReport::new(args.detailed)),
            ReportFormat::Markdown => Box::new(MarkdownReport::new(args.detailed)),
            ReportFormat::Json => Box::new(
                JsonOutput::new()
                    .with_schema(args.schema)
                    .with_validation(args.validate),
            ),
            ReportFormat::Csv => Box::new(
                CsvOutput::new()
                    .with_delimiter(args.csv_delimiter as u8)
                    .with_bom(args.csv_bom)
                    .with_keep_newlines(args.csv_keep_newlines),
            ),
            ReportFormat::Pdf => unreachable!("handled above"),
        };

        let report = renderer.render_report(&audit_results)?;
        std::fs::write(&output_path, &report).map_err(|e| {
            RepoLensError::Action(crate::error::ActionError::FileWrite {
                path: output_path.display().to_string(),
                source: e,
            })
        })?;
    }

    println!(
        "{} Report written to: {}",
        "Success:".green().bold(),
        output_path.display().to_string().cyan()
    );

    // Return exit code based on findings
    let exit_code = if audit_results.has_critical() {
        exit_codes::CRITICAL_ISSUES
    } else if audit_results.has_warnings() {
        exit_codes::WARNINGS
    } else {
        exit_codes::SUCCESS
    };

    Ok(exit_code)
}
