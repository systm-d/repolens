//! CLI commands module

pub mod apply;
pub mod compare;
pub mod completions;
pub mod generate_man;
pub mod init;
pub mod install_hooks;
pub mod plan;
pub mod report;
pub mod schema;

use clap::Args;
use std::path::PathBuf;

/// Arguments for the init command
#[derive(Args, Debug)]
pub struct InitArgs {
    /// Preset to use (opensource, enterprise, strict)
    #[arg(
        short,
        long,
        value_name = "PRESET",
        value_parser = clap::builder::PossibleValuesParser::new(["opensource", "enterprise", "strict"])
    )]
    pub preset: Option<String>,

    /// Force overwrite existing configuration
    #[arg(short, long)]
    pub force: bool,

    /// Skip interactive prompts
    #[arg(long)]
    pub non_interactive: bool,

    /// Skip prerequisite checks (git, gh, etc.)
    #[arg(long)]
    pub skip_checks: bool,
}

/// Arguments for the plan command
#[derive(Args, Debug)]
pub struct PlanArgs {
    /// Output format (terminal, json, sarif)
    #[arg(short, long, default_value = "terminal")]
    pub format: OutputFormat,

    /// Only check specific rule categories
    #[arg(
        long,
        value_delimiter = ',',
        value_parser = clap::builder::PossibleValuesParser::new(crate::rules::constants::VALID_CATEGORIES)
    )]
    pub only: Option<Vec<String>>,

    /// Skip specific rule categories
    #[arg(
        long,
        value_delimiter = ',',
        value_parser = clap::builder::PossibleValuesParser::new(crate::rules::constants::VALID_CATEGORIES)
    )]
    pub skip: Option<Vec<String>>,

    /// Output file (defaults to stdout)
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Disable cache and force a complete re-audit
    #[arg(long)]
    pub no_cache: bool,

    /// Clear the cache before running the audit
    #[arg(long)]
    pub clear_cache: bool,

    /// Custom cache directory path
    #[arg(long, value_name = "DIR")]
    pub cache_dir: Option<PathBuf>,

    /// Verbosity level (passed from global args)
    #[arg(skip)]
    pub verbose: u8,
}

/// Arguments for the apply command
#[derive(Args, Debug)]
pub struct ApplyArgs {
    /// Skip confirmation prompts and apply all actions automatically
    #[arg(short, long)]
    pub yes: bool,

    /// Enable interactive mode with action selection and diff preview
    #[arg(short, long)]
    pub interactive: bool,

    /// Dry run - show what would be done without making changes
    #[arg(long)]
    pub dry_run: bool,

    /// Only apply actions for specific rule categories
    #[arg(
        long,
        value_delimiter = ',',
        value_parser = clap::builder::PossibleValuesParser::new(crate::rules::constants::VALID_CATEGORIES)
    )]
    pub only: Option<Vec<String>>,

    /// Skip actions for specific rule categories
    #[arg(
        long,
        value_delimiter = ',',
        value_parser = clap::builder::PossibleValuesParser::new(crate::rules::constants::VALID_CATEGORIES)
    )]
    pub skip: Option<Vec<String>>,

    /// Create a pull request with the changes (default: true if in a git repository)
    #[arg(long)]
    pub create_pr: Option<bool>,

    /// Skip creating a pull request (overrides --create-pr)
    #[arg(long)]
    pub no_pr: bool,

    /// Skip automatic issue creation for warnings
    #[arg(long, default_value_t = false)]
    pub no_issues: bool,
}

/// Arguments for the report command
#[derive(Args, Debug)]
pub struct ReportArgs {
    /// Output format (html, markdown, json)
    #[arg(short, long, default_value = "markdown")]
    pub format: ReportFormat,

    /// Output file
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Include full details in report
    #[arg(long)]
    pub detailed: bool,

    /// Only check specific rule categories
    #[arg(
        long,
        value_delimiter = ',',
        value_parser = clap::builder::PossibleValuesParser::new(crate::rules::constants::VALID_CATEGORIES)
    )]
    pub only: Option<Vec<String>>,

    /// Skip specific rule categories
    #[arg(
        long,
        value_delimiter = ',',
        value_parser = clap::builder::PossibleValuesParser::new(crate::rules::constants::VALID_CATEGORIES)
    )]
    pub skip: Option<Vec<String>>,

    /// Include JSON Schema reference ($schema) in JSON output
    #[arg(long)]
    pub schema: bool,

    /// Validate JSON output against the JSON Schema before emitting
    #[arg(long)]
    pub validate: bool,

    /// Disable cache and force a complete re-audit
    #[arg(long)]
    pub no_cache: bool,

    /// Clear the cache before running the audit
    #[arg(long)]
    pub clear_cache: bool,

    /// Custom cache directory path
    #[arg(long, value_name = "DIR")]
    pub cache_dir: Option<PathBuf>,

    /// Verbosity level (passed from global args)
    #[arg(skip)]
    pub verbose: u8,
}

/// Arguments for the schema command
#[derive(Args, Debug)]
pub struct SchemaArgs {
    /// Output file (defaults to stdout)
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,
}

/// Arguments for the install-hooks command
#[derive(Args, Debug)]
pub struct InstallHooksArgs {
    /// Install only the pre-commit hook
    #[arg(long)]
    pub pre_commit: bool,

    /// Install only the pre-push hook
    #[arg(long)]
    pub pre_push: bool,

    /// Install all hooks (default behavior)
    #[arg(long)]
    pub all: bool,

    /// Remove installed hooks
    #[arg(long)]
    pub remove: bool,

    /// Force overwrite existing hooks (backs up originals)
    #[arg(long)]
    pub force: bool,
}

/// Output format for plan command
#[derive(Debug, Clone, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    Terminal,
    Json,
    Sarif,
}

/// Output format for report command
#[derive(Debug, Clone, PartialEq, Eq, clap::ValueEnum)]
pub enum ReportFormat {
    Html,
    Markdown,
    Json,
}

/// Arguments for the compare command
#[derive(Args, Debug)]
pub struct CompareArgs {
    /// Path to the base (before) report JSON file
    #[arg(long, value_name = "FILE")]
    pub base_file: PathBuf,

    /// Path to the head (after) report JSON file
    #[arg(long, value_name = "FILE")]
    pub head_file: PathBuf,

    /// Output format (terminal, json, markdown)
    #[arg(short, long, default_value = "terminal")]
    pub format: CompareFormat,

    /// Output file (defaults to stdout)
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// Exit with code 1 if regressions (new issues) are found
    #[arg(long)]
    pub fail_on_regression: bool,
}

/// Output format for compare command
#[derive(Debug, Clone, PartialEq, Eq, clap::ValueEnum)]
pub enum CompareFormat {
    Terminal,
    Json,
    Markdown,
}

/// Arguments for the generate-man command
#[derive(Args, Debug)]
pub struct GenerateManArgs {
    /// Output directory for man pages
    #[arg(short, long, default_value = ".")]
    pub output: PathBuf,
}

/// Supported shells for completion generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ShellChoice {
    Bash,
    Zsh,
    Fish,
    #[value(name = "powershell")]
    PowerShell,
    Elvish,
    Nushell,
}

/// Arguments for the completions command
#[derive(Args, Debug)]
pub struct CompletionsArgs {
    /// Target shell for which to generate the completion script
    #[arg(value_enum)]
    pub shell: ShellChoice,
}
