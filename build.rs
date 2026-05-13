//! Build script.
//!
//! Generates shell completion files into `target/completions/` when
//! `GENERATE_COMPLETIONS=1` is set or when building in `release` profile.
//! Man pages still rely on the runtime `repolens generate-man` subcommand.
//!
//! The build-time CLI definition below mirrors `src/cli/mod.rs` and
//! `src/cli/commands/mod.rs`. It is duplicated because Cargo build scripts
//! run before the main crate is compiled, so the runtime `Cli` struct is
//! not importable here. Keep the two definitions in sync when adding new
//! commands or flags so the generated completions stay accurate.

use clap::{Args, CommandFactory, Parser, Subcommand, ValueEnum, ValueHint};
use clap_complete::{Shell, generate_to};
use clap_complete_nushell::Nushell;
use std::env;
use std::path::{Path, PathBuf};

const VALID_CATEGORIES: &[&str] = &[
    "secrets",
    "files",
    "docs",
    "security",
    "workflows",
    "quality",
    "dependencies",
    "licenses",
    "docker",
    "git",
    "custom",
];

const VALID_PRESETS: &[&str] = &["opensource", "enterprise", "strict"];

#[derive(Parser)]
#[command(name = "repolens", version, about, propagate_version = true)]
struct Cli {
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    #[arg(short, long, global = true, value_name = "FILE", value_hint = ValueHint::FilePath)]
    config: Option<PathBuf>,

    #[arg(short = 'C', long, global = true, value_name = "DIR", value_hint = ValueHint::DirPath)]
    directory: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Init(InitArgs),
    Plan(PlanArgs),
    Apply(ApplyArgs),
    Report(ReportArgs),
    Schema(SchemaArgs),
    Compare(CompareArgs),
    InstallHooks(InstallHooksArgs),
    #[command(hide = true)]
    GenerateMan(GenerateManArgs),
    #[command(hide = true)]
    Completions(CompletionsArgs),
}

#[derive(Args)]
struct InitArgs {
    #[arg(
        short,
        long,
        value_name = "PRESET",
        value_parser = clap::builder::PossibleValuesParser::new(VALID_PRESETS)
    )]
    preset: Option<String>,
    #[arg(short, long)]
    force: bool,
    #[arg(long)]
    non_interactive: bool,
    #[arg(long)]
    skip_checks: bool,
}

#[derive(Args)]
struct PlanArgs {
    #[arg(short, long, default_value = "terminal")]
    format: OutputFormat,
    #[arg(
        long,
        value_delimiter = ',',
        value_parser = clap::builder::PossibleValuesParser::new(VALID_CATEGORIES)
    )]
    only: Option<Vec<String>>,
    #[arg(
        long,
        value_delimiter = ',',
        value_parser = clap::builder::PossibleValuesParser::new(VALID_CATEGORIES)
    )]
    skip: Option<Vec<String>>,
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,
    #[arg(long)]
    no_cache: bool,
    #[arg(long)]
    clear_cache: bool,
    #[arg(long, value_name = "DIR")]
    cache_dir: Option<PathBuf>,
}

#[derive(Args)]
struct ApplyArgs {
    #[arg(short, long)]
    yes: bool,
    #[arg(short, long)]
    interactive: bool,
    #[arg(long)]
    dry_run: bool,
    #[arg(
        long,
        value_delimiter = ',',
        value_parser = clap::builder::PossibleValuesParser::new(VALID_CATEGORIES)
    )]
    only: Option<Vec<String>>,
    #[arg(
        long,
        value_delimiter = ',',
        value_parser = clap::builder::PossibleValuesParser::new(VALID_CATEGORIES)
    )]
    skip: Option<Vec<String>>,
    #[arg(long)]
    create_pr: Option<bool>,
    #[arg(long)]
    no_pr: bool,
    #[arg(long, default_value_t = false)]
    no_issues: bool,
}

#[derive(Args)]
struct ReportArgs {
    #[arg(short, long, default_value = "markdown")]
    format: ReportFormat,
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,
    #[arg(long)]
    detailed: bool,
    #[arg(
        long,
        value_delimiter = ',',
        value_parser = clap::builder::PossibleValuesParser::new(VALID_CATEGORIES)
    )]
    only: Option<Vec<String>>,
    #[arg(
        long,
        value_delimiter = ',',
        value_parser = clap::builder::PossibleValuesParser::new(VALID_CATEGORIES)
    )]
    skip: Option<Vec<String>>,
    #[arg(long)]
    schema: bool,
    #[arg(long)]
    validate: bool,
    #[arg(long)]
    no_cache: bool,
    #[arg(long)]
    clear_cache: bool,
    #[arg(long, value_name = "DIR")]
    cache_dir: Option<PathBuf>,
}

#[derive(Args)]
struct SchemaArgs {
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,
}

#[derive(Args)]
struct CompareArgs {
    #[arg(long, value_name = "FILE")]
    base_file: PathBuf,
    #[arg(long, value_name = "FILE")]
    head_file: PathBuf,
    #[arg(short, long, default_value = "terminal")]
    format: CompareFormat,
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,
    #[arg(long)]
    fail_on_regression: bool,
}

#[derive(Args)]
struct InstallHooksArgs {
    #[arg(long)]
    pre_commit: bool,
    #[arg(long)]
    pre_push: bool,
    #[arg(long)]
    all: bool,
    #[arg(long)]
    remove: bool,
    #[arg(long)]
    force: bool,
}

#[derive(Args)]
struct GenerateManArgs {
    #[arg(short, long, default_value = ".")]
    output: PathBuf,
}

#[derive(Args)]
struct CompletionsArgs {
    #[arg(value_enum)]
    shell: ShellChoice,
}

#[derive(Clone, ValueEnum)]
enum OutputFormat {
    Terminal,
    Json,
    Sarif,
}

#[derive(Clone, ValueEnum)]
enum ReportFormat {
    Html,
    Markdown,
    Json,
}

#[derive(Clone, ValueEnum)]
enum CompareFormat {
    Terminal,
    Json,
    Markdown,
}

#[derive(Clone, ValueEnum)]
enum ShellChoice {
    Bash,
    Zsh,
    Fish,
    #[value(name = "powershell")]
    PowerShell,
    Elvish,
    Nushell,
}

fn write_completions(out_dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(out_dir)?;
    let mut cmd = Cli::command();
    let bin = "repolens";
    generate_to(Shell::Bash, &mut cmd, bin, out_dir)?;
    generate_to(Shell::Zsh, &mut cmd, bin, out_dir)?;
    generate_to(Shell::Fish, &mut cmd, bin, out_dir)?;
    generate_to(Shell::PowerShell, &mut cmd, bin, out_dir)?;
    generate_to(Shell::Elvish, &mut cmd, bin, out_dir)?;
    generate_to(Nushell, &mut cmd, bin, out_dir)?;
    Ok(())
}

fn main() {
    println!("cargo:rerun-if-env-changed=GENERATE_MAN");
    println!("cargo:rerun-if-env-changed=GENERATE_COMPLETIONS");
    println!("cargo:rerun-if-changed=src/cli/mod.rs");
    println!("cargo:rerun-if-changed=src/cli/commands/mod.rs");
    println!("cargo:rerun-if-changed=build.rs");

    let is_release = env::var("PROFILE").as_deref() == Ok("release");
    let want_completions = is_release || env::var("GENERATE_COMPLETIONS").is_ok();

    if want_completions {
        let out_dir = PathBuf::from("target").join("completions");
        if let Err(e) = write_completions(&out_dir) {
            println!(
                "cargo:warning=Failed to generate shell completions: {} ({})",
                e,
                out_dir.display()
            );
        }
    }
}
