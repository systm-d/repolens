//! RepoLens - A CLI tool to audit and prepare repositories for open source or enterprise standards
//!
//! This is the main entry point for the CLI application.
//!
//! # Configuration Sources
//!
//! Configuration is loaded with the following priority (highest to lowest):
//! 1. CLI flags (-c, -C, -v, etc.)
//! 2. Environment variables (REPOLENS_CONFIG, REPOLENS_VERBOSE, etc.)
//! 3. Configuration file (.repolens.toml)
//! 4. Default values

use clap::Parser;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

mod actions;
mod cache;
mod cli;
mod compare;
mod config;
mod error;
mod hooks;
mod providers;
mod rules;
mod scanner;
mod utils;

use config::get_env_verbosity;
use error::RepoLensError;

// Use exit_codes from cli module
use cli::exit_codes;
use cli::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<(), RepoLensError> {
    // Parse CLI arguments. Map clap's argument-validation errors to our
    // INVALID_ARGS exit code so CI scripts can distinguish them from
    // WARNINGS (also exit code 2 in clap's default).
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            let exit_code = match err.kind() {
                clap::error::ErrorKind::DisplayHelp
                | clap::error::ErrorKind::DisplayVersion
                | clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => {
                    exit_codes::SUCCESS
                }
                _ => exit_codes::INVALID_ARGS,
            };
            // `print` writes to stdout for help/version, stderr for errors —
            // matches clap's default behavior.
            let _ = err.print();
            std::process::exit(exit_code);
        }
    };

    // Determine verbosity: CLI flag > env var > default (0)
    let verbosity = if cli.verbose > 0 {
        cli.verbose
    } else {
        get_env_verbosity().unwrap_or(0)
    };

    // Setup logging based on verbosity
    setup_logging(verbosity);

    // Change to specified directory if provided via -C flag
    if let Some(ref directory) = cli.directory {
        if !directory.exists() {
            eprintln!("Error: Directory '{}' does not exist", directory.display());
            std::process::exit(exit_codes::ERROR);
        }
        if let Err(e) = std::env::set_current_dir(directory) {
            eprintln!(
                "Error: Cannot access directory '{}': {}",
                directory.display(),
                e
            );
            std::process::exit(exit_codes::ERROR);
        }
    }

    // Execute the appropriate command
    let result = match cli.command {
        Commands::Init(args) => cli::commands::init::execute(args).await,
        Commands::Plan(mut args) => {
            args.verbose = verbosity;
            cli::commands::plan::execute(args).await
        }
        Commands::Apply(args) => cli::commands::apply::execute(args).await,
        Commands::Report(mut args) => {
            args.verbose = verbosity;
            cli::commands::report::execute(args).await
        }
        Commands::Schema(args) => cli::commands::schema::execute(args).await,
        Commands::Compare(args) => cli::commands::compare::execute(args).await,
        Commands::InstallHooks(args) => cli::commands::install_hooks::execute(args).await,
        Commands::GenerateMan(args) => cli::commands::generate_man::execute(args).await,
        Commands::Completions(args) => {
            cli::commands::completions::execute(args.shell, std::io::stdout())
                .map(|_| exit_codes::SUCCESS)
                .map_err(|e| {
                    RepoLensError::Config(error::ConfigError::Serialize {
                        message: format!("Failed to generate shell completions: {}", e),
                    })
                })
        }
    };

    // Handle exit codes for CI integration
    match result {
        Ok(exit_code) => std::process::exit(exit_code),
        Err(e) => {
            eprint!("{}", e.display_formatted());
            std::process::exit(exit_codes::ERROR);
        }
    }
}

fn setup_logging(verbosity: u8) {
    let filter = match verbosity {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter)))
        .init();
}
