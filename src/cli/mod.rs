//! # CLI Module
//!
//! This module defines the command-line interface for RepoLens using `clap`.
//!
//! ## Commands
//!
//! | Command | Description |
//! |---------|-------------|
//! | `init` | Initialize a new configuration file |
//! | `plan` | Analyze repository and show planned actions |
//! | `apply` | Apply planned changes to the repository |
//! | `report` | Generate an audit report |
//! | `compare` | Compare two audit reports |
//! | `schema` | Display JSON Schema for report output |
//! | `install-hooks` | Install or remove Git hooks |
//!
//! ## Submodules
//!
//! - [`commands`] - Command implementations
//! - [`exit_codes`] - Standardized exit codes
//! - [`output`] - Report output formatters (JSON, Markdown, SARIF, Terminal)
//!
//! ## Global Options
//!
//! All commands support these global options:
//!
//! - `-v, --verbose` - Increase verbosity level (use multiple times: -v, -vv, -vvv)
//! - `-c, --config <FILE>` - Path to configuration file
//! - `-C, --directory <DIR>` - Working directory (defaults to current directory)
//!
//! ## Examples
//!
//! ```bash
//! # Initialize configuration
//! repolens init --preset opensource
//!
//! # Run audit and show plan
//! repolens plan
//!
//! # Generate JSON report
//! repolens report --format json -o report.json
//!
//! # Apply fixes
//! repolens apply --yes
//! ```

pub mod commands;
pub mod exit_codes;
pub mod output;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

use commands::{
    ApplyArgs, CompareArgs, CompletionsArgs, GenerateManArgs, InitArgs, InstallHooksArgs, PlanArgs,
    ReportArgs, SchemaArgs,
};

/// RepoLens - Audit and prepare repositories for open source or enterprise standards
#[derive(Parser, Debug)]
#[command(name = "repolens")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Increase verbosity level (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Path to configuration file
    #[arg(short, long, global = true, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// Working directory (defaults to current directory)
    #[arg(short = 'C', long, global = true, value_name = "DIR")]
    pub directory: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new configuration file
    Init(InitArgs),

    /// Analyze repository and show planned actions
    Plan(PlanArgs),

    /// Apply planned changes to the repository
    Apply(ApplyArgs),

    /// Generate an audit report
    Report(ReportArgs),

    /// Display the JSON Schema for audit report output
    Schema(SchemaArgs),

    /// Compare two audit reports
    Compare(CompareArgs),

    /// Install or remove Git hooks (pre-commit, pre-push)
    InstallHooks(InstallHooksArgs),

    /// Generate man page (hidden, for packaging)
    #[command(hide = true)]
    GenerateMan(GenerateManArgs),

    /// Generate shell completion scripts (hidden, for packaging)
    #[command(hide = true)]
    Completions(CompletionsArgs),
}
