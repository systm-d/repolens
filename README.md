# RepoLens

A CLI tool to audit GitHub repositories for best practices, security, and compliance.

## Features

- Audit repositories for security issues and best practices
- Detect exposed secrets and credentials
- Check for required files (README, LICENSE, CONTRIBUTING, etc.)
- Validate GitHub workflows and Actions
- Verify license compliance across dependencies
- Generate actionable fix plans
- Apply fixes automatically or with dry-run mode
- Multiple output formats: terminal, JSON, SARIF, Markdown, HTML

## Installation

### Docker (Recommended)

The easiest way to use RepoLens without local installation:

```bash
# Pull the official image
docker pull ghcr.io/systm-d/repolens:latest

# Audit current directory
docker run --rm -v "$(pwd)":/repo ghcr.io/systm-d/repolens plan

# Generate a report
docker run --rm -v "$(pwd)":/repo ghcr.io/systm-d/repolens report --format json
```

See [docs/docker.md](docs/docker.md) for detailed Docker usage.

### Package Managers

#### Homebrew (macOS/Linux)

```bash
brew tap systm-d/repolens
brew install repolens
```

#### Scoop (Windows)

```powershell
scoop bucket add systm-d https://github.com/systm-d/scoop-bucket
scoop install repolens
```

#### AUR (Arch Linux)

```bash
yay -S repolens
```

### From crates.io

```bash
cargo install repolens
```

### Pre-built Binaries

Pre-built binaries are available for all major platforms. Download the latest release from the [Releases page](https://github.com/systm-d/repolens/releases).

#### Supported Platforms

| Platform | Architecture | Archive |
|----------|-------------|---------|
| Linux | x86_64 | `repolens-linux-x86_64.tar.gz` |
| Linux | ARM64 | `repolens-linux-arm64.tar.gz` |
| macOS | Intel x86_64 | `repolens-darwin-x86_64.tar.gz` |
| macOS | Apple Silicon ARM64 | `repolens-darwin-arm64.tar.gz` |
| Windows | x86_64 | `repolens-windows-x86_64.zip` |

#### Linux (x86_64)

```bash
curl -LO https://github.com/systm-d/repolens/releases/latest/download/repolens-linux-x86_64.tar.gz
tar xzf repolens-linux-x86_64.tar.gz
sudo mv repolens /usr/local/bin/
```

#### Linux (ARM64)

```bash
curl -LO https://github.com/systm-d/repolens/releases/latest/download/repolens-linux-arm64.tar.gz
tar xzf repolens-linux-arm64.tar.gz
sudo mv repolens /usr/local/bin/
```

#### macOS (Apple Silicon)

```bash
curl -LO https://github.com/systm-d/repolens/releases/latest/download/repolens-darwin-arm64.tar.gz
tar xzf repolens-darwin-arm64.tar.gz
sudo mv repolens /usr/local/bin/
```

#### macOS (Intel)

```bash
curl -LO https://github.com/systm-d/repolens/releases/latest/download/repolens-darwin-x86_64.tar.gz
tar xzf repolens-darwin-x86_64.tar.gz
sudo mv repolens /usr/local/bin/
```

#### Windows (x86_64)

```powershell
# Download the zip archive from the Releases page
Invoke-WebRequest -Uri https://github.com/systm-d/repolens/releases/latest/download/repolens-windows-x86_64.zip -OutFile repolens-windows-x86_64.zip
Expand-Archive repolens-windows-x86_64.zip -DestinationPath .
Move-Item repolens.exe C:\Users\$env:USERNAME\bin\
```

#### Verify Checksums

Each release includes a `checksums.sha256` file. After downloading your archive, verify its integrity:

```bash
# Download the checksums file
curl -LO https://github.com/systm-d/repolens/releases/latest/download/checksums.sha256

# Verify (Linux)
sha256sum -c checksums.sha256 --ignore-missing

# Verify (macOS)
shasum -a 256 -c checksums.sha256 --ignore-missing
```

#### Verify Installation

```bash
repolens --version
```

### From Source

```bash
# Clone repository
git clone https://github.com/systm-d/repolens.git
cd repolens

# Build
cargo build --release

# The binary will be at target/release/repolens
```

### Shell completions

Generate completions for your shell with `repolens completions <shell>`
(supports `bash`, `zsh`, `fish`, `powershell`, `elvish`, `nushell`):

```bash
# Bash (system-wide)
repolens completions bash | sudo tee /etc/bash_completion.d/repolens > /dev/null

# Zsh (drop into a directory in $fpath)
repolens completions zsh > "${fpath[1]}/_repolens"

# Fish
repolens completions fish > ~/.config/fish/completions/repolens.fish
```

See [docs/installation/completions.md](docs/installation/completions.md) for the full per-shell guide.

### Nightly Builds

Nightly builds are available for testing. See the [Releases page](https://github.com/systm-d/repolens/releases) for nightly builds (marked as pre-release).

**Warning**: Nightly builds may be unstable. Use at your own risk.

### Docker

RepoLens is available as a Docker image for easy deployment:

```bash
# Pull the latest image
docker pull ghcr.io/systm-d/repolens:latest

# Run on current directory
docker run --rm -v "$(pwd)":/repo ghcr.io/systm-d/repolens plan

# Generate a report
docker run --rm -v "$(pwd)":/repo ghcr.io/systm-d/repolens report
```

For GitHub API access, mount your GitHub CLI config:

```bash
docker run --rm \
  -v "$(pwd)":/repo \
  -v ~/.config/gh:/home/repolens/.config/gh:ro \
  ghcr.io/systm-d/repolens plan
```

See [docs/docker.md](docs/docker.md) for detailed Docker usage instructions.

## Prerequisites

RepoLens requires the following tools to be installed and configured:

| Tool | Required | Description |
|------|----------|-------------|
| Git | Yes | Must be installed and the directory must be a git repository |
| GitHub CLI (gh) | Yes | Must be installed and authenticated (`gh auth login`) |

When running `repolens init`, these prerequisites are automatically verified:

```
Checking prerequisites...

  ✓ Git installed
  ✓ Git repository
  ✓ GitHub CLI installed
  ✓ GitHub CLI authenticated
  ✓ Remote origin configured
  ✓ Remote is GitHub
```

If a required prerequisite fails, you'll see an error with a suggested fix:

```
  ✗ GitHub CLI installed
    GitHub CLI (gh) is not installed
    Fix: Install gh: https://cli.github.com/
```

Use `--skip-checks` to bypass prerequisite verification (not recommended).

## Usage

### Initialize Configuration

```bash
# Create default configuration
repolens init

# Use a preset
repolens init --preset opensource
repolens init --preset enterprise
repolens init --preset strict

# Skip prerequisite checks (not recommended)
repolens init --skip-checks
```

### Run Audit

```bash
# Generate audit plan
repolens plan

# Audit a different directory
repolens -C /path/to/project plan

# Output in different formats
repolens plan --format json
repolens plan --format sarif
repolens plan --format markdown

# Verbose mode with timing information
repolens plan -v      # Basic timing
repolens plan -vv     # Detailed timing per category
repolens plan -vvv    # Debug level
```

### Apply Fixes

```bash
# Preview changes (shows diff without applying)
repolens apply --dry-run

# Apply all fixes with confirmation prompt
repolens apply

# Interactive mode: select actions individually with diff preview
repolens apply --interactive
repolens apply -i

# Auto-accept all actions without confirmation
repolens apply --yes
repolens apply -y

# Apply specific categories only
repolens apply --only files,docs

# Skip specific categories
repolens apply --skip security
```

#### Interactive Mode

The interactive mode (`-i` or `--interactive`) provides an enhanced user experience:

1. **Visual Summary**: Displays a categorized overview of all planned actions
2. **Action Selection**: Use `MultiSelect` to choose which actions to apply (Space to toggle, Enter to confirm)
3. **Diff Preview**: Shows a colored diff (green for additions, red for deletions) for each selected action
4. **Progress Bar**: Displays real-time progress during execution
5. **Execution Summary**: Shows detailed results with success/failure counts

Example output:
```
==============================================================================
                     ACTION SUMMARY
==============================================================================

[F] GITIGNORE (1 action)
    + Update .gitignore with recommended entries
      - .env
      - *.key

[F] FILES (2 actions)
    + Create CONTRIBUTING.md from template
    + Create SECURITY.md from template

==============================================================================
  Total: 3 actions to apply
==============================================================================
```

### Generate Report

```bash
# Terminal report
repolens report

# Export report
repolens report --format html --output report.html

# JSON report with JSON Schema reference
repolens report --format json --schema

# JSON report with schema validation
repolens report --format json --schema --validate
```

### JSON Schema

RepoLens provides a JSON Schema (draft-07) that describes the structure of the JSON audit report output. This enables validation of report output and integration with tools that consume JSON Schema.

```bash
# Display the JSON Schema on stdout
repolens schema

# Save the JSON Schema to a file
repolens schema --output schemas/audit-report.schema.json
```

The schema defines the following structure:

- **repository_name**: Name of the audited repository
- **preset**: Audit preset used (opensource, enterprise, strict)
- **findings**: Array of audit findings, each with:
  - `rule_id`: Unique rule identifier (e.g., SEC001)
  - `category`: Finding category (secrets, files, docs, security, workflows, quality)
  - `severity`: Severity level (critical, warning, info)
  - `message`: Description of the finding
  - `location`: Optional file location
  - `description`: Optional detailed description
  - `remediation`: Optional suggested fix
- **metadata**: Report metadata (version, timestamp, schema_version)
- **summary**: Aggregated counts by severity and category

When using `--schema`, the JSON output includes a `$schema` field referencing the schema URI. When using `--validate`, the output is validated against the schema before being emitted.

### Comparing Audits

Compare two previously generated JSON audit reports to visualize improvements and regressions between runs.

```bash
# First, generate two JSON reports at different points in time
repolens report --format json --output report-before.json
# ... make changes ...
repolens report --format json --output report-after.json

# Compare the two reports (terminal output with colors)
repolens compare --base-file report-before.json --head-file report-after.json

# Output as JSON
repolens compare --base-file report-before.json --head-file report-after.json --format json

# Output as Markdown
repolens compare --base-file report-before.json --head-file report-after.json --format markdown

# Save comparison to a file
repolens compare --base-file report-before.json --head-file report-after.json --output comparison.md --format markdown

# Fail with exit code 1 if new issues are detected (useful in CI)
repolens compare --base-file baseline.json --head-file current.json --fail-on-regression
```

The comparison report includes:
- **Score summary**: Weighted score (Critical=10, Warning=3, Info=1) with diff
- **New issues**: Findings present in the head report but not in the base (regressions)
- **Resolved issues**: Findings present in the base report but not in the head (improvements)
- **Category breakdown**: Per-category count changes

## Configuration

Create a `.repolens.toml` file in your repository root:

```toml
[general]
preset = "opensource"

[rules]
secrets = true
files = true
docs = true
security = true
workflows = true
quality = true

[files.required]
readme = true
license = true
contributing = true
code_of_conduct = true
security = true
```

### Custom Rules

Define your own audit rules using regex patterns or shell commands:

```toml
# Detect TODO comments
[rules.custom."no-todo"]
pattern = "TODO"
severity = "warning"
files = ["**/*.rs"]
message = "TODO comment found"

# Check git status (shell command)
[rules.custom."check-git-status"]
command = "git status --porcelain"
severity = "warning"
invert = true  # Fail if uncommitted changes
message = "Working directory is not clean"
```

> **Security Warning**: Custom rules with shell commands execute arbitrary code on your system. Only use commands from trusted sources. Never commit or run `.repolens.toml` files from untrusted repositories without reviewing them first.

See the [Custom Rules documentation](wiki/Custom-Rules.md) for more examples and details.

### Cache

RepoLens includes a caching system to improve performance by avoiding re-auditing files that haven't changed. Cache entries are automatically invalidated when file content changes (detected via SHA256 hashing).

#### Cache Configuration

```toml
[cache]
# Enable/disable caching (default: true)
enabled = true
# Maximum age for cache entries in hours (default: 24)
max_age_hours = 24
# Cache directory (relative to project root or absolute path)
directory = ".repolens/cache"
```

#### Cache CLI Options

```bash
# Disable cache and force a complete re-audit
repolens plan --no-cache

# Clear the cache before running the audit
repolens plan --clear-cache

# Use a custom cache directory
repolens plan --cache-dir /tmp/repolens-cache
```

The same options are available for the `report` command.
```

### Environment Variables

RepoLens can be configured via environment variables. Priority order: CLI flags > Environment variables > Config file > Defaults.

| Variable | Description | Example |
|----------|-------------|---------|
| `REPOLENS_PRESET` | Default preset to use | `enterprise` |
| `REPOLENS_VERBOSE` | Verbosity level (0-3) | `2` |
| `REPOLENS_CONFIG` | Path to config file | `/path/to/.repolens.toml` |
| `REPOLENS_NO_CACHE` | Disable caching | `true` |
| `REPOLENS_GITHUB_TOKEN` | GitHub token for API calls | `ghp_xxx` |

```bash
# Example usage
export REPOLENS_PRESET=enterprise
export REPOLENS_VERBOSE=2
repolens plan
```

### Exit Codes

RepoLens uses standard exit codes for CI/CD integration:

| Code | Meaning | Example |
|------|---------|---------|
| 0 | Success | Audit completed, no critical issues |
| 1 | Critical issues | Secrets exposed, critical vulnerabilities |
| 2 | Warnings | Missing files, non-critical findings |
| 3 | Runtime error | File not found, network error |
| 4 | Invalid arguments | Unknown category, invalid preset |

```bash
# Example usage in CI/CD
repolens plan
case $? in
  0) echo "All clear!" ;;
  1) echo "Critical issues found - blocking release" && exit 1 ;;
  2) echo "Warnings found - review recommended" ;;
  3) echo "Error running audit" && exit 1 ;;
  4) echo "Invalid arguments" && exit 1 ;;
esac
```

### Git Hooks

RepoLens can install Git hooks to automatically check your code before commits and pushes.

#### Install Hooks

```bash
# Install all configured hooks (pre-commit + pre-push)
repolens install-hooks

# Install only the pre-commit hook
repolens install-hooks --pre-commit

# Install only the pre-push hook
repolens install-hooks --pre-push

# Force overwrite existing hooks (backs up originals)
repolens install-hooks --force
```

#### Remove Hooks

```bash
# Remove all RepoLens hooks (restores backups if they exist)
repolens install-hooks --remove
```

#### Hook Behavior

- **pre-commit**: Scans staged files for exposed secrets before each commit. If secrets are detected, the commit is aborted.
- **pre-push**: Runs a full audit before pushing. If issues are found, the push is aborted.

Both hooks can be bypassed with `--no-verify` (e.g., `git commit --no-verify`).

#### Configuration

Configure hooks in `.repolens.toml`:

```toml
[hooks]
# Install pre-commit hook (checks for exposed secrets)
pre_commit = true
# Install pre-push hook (runs full audit)
pre_push = true
# Whether warnings should cause hook failure
fail_on_warnings = false
```

When `fail_on_warnings` is `true`, hooks will also fail on warning-level findings, not just critical issues.

## Presets

| Preset | Description |
|--------|-------------|
| `opensource` | Standard open-source requirements |
| `enterprise` | Enterprise security and compliance |
| `strict` | Maximum security and documentation |

## Rules Categories

- **secrets**: Detect exposed API keys, tokens, passwords
- **files**: Check for required repository files
- **docs**: Documentation completeness and quality
- **security**: Security best practices, branch protection (SEC007-010)
- **workflows**: CI/CD and GitHub Actions validation
- **quality**: Code quality standards
- **licenses**: License compliance checking (LIC001-LIC004)
- **dependencies**: Vulnerability scanning via OSV API (DEP001-003)
- **git**: Git hygiene rules (GIT001-003)

### Git Hygiene Rules

| Rule | Severity | Description |
|------|----------|-------------|
| GIT001 | Warning | Large binary files detected (should use Git LFS) |
| GIT002 | Info | `.gitattributes` file missing |
| GIT003 | Warning | Sensitive files tracked (.env, *.key, *.pem, credentials) |

### Branch Protection Rules

| Rule | Severity | Description |
|------|----------|-------------|
| SEC007 | Info | `.github/settings.yml` missing |
| SEC008 | Warning | No branch protection rules in settings.yml |
| SEC009 | Warning | `required_pull_request_reviews` not configured |
| SEC010 | Warning | `required_status_checks` not configured |

### Dependency Rules

| Rule | Severity | Description |
|------|----------|-------------|
| DEP001 | Critical/Warning | Vulnerability detected in dependency |
| DEP002 | Warning | Outdated dependency version |
| DEP003 | Warning | Lock file missing for detected ecosystem |

### Supported Ecosystems

RepoLens supports vulnerability scanning for multiple ecosystems:

| Ecosystem | Manifest | Lock File | OSV Support |
|-----------|----------|-----------|-------------|
| Rust (Cargo) | `Cargo.toml` | `Cargo.lock` | Yes |
| Node.js (npm) | `package.json` | `package-lock.json` | Yes |
| Python (pip/poetry) | `pyproject.toml` | `poetry.lock` | Yes |
| Go | `go.mod` | `go.sum` | Yes |
| .NET (NuGet) | `*.csproj` | `packages.lock.json` | Yes |
| Ruby (Bundler) | `Gemfile` | `Gemfile.lock` | Yes |
| Dart/Flutter (Pub) | `pubspec.yaml` | `pubspec.lock` | Yes |
| Swift (SPM) | `Package.swift` | `Package.resolved` | No |
| iOS (CocoaPods) | `Podfile` | `Podfile.lock` | No |

### License Compliance Rules

RepoLens can detect and verify license compliance for your project and its dependencies:

| Rule | Severity | Description |
|------|----------|-------------|
| LIC001 | Warning | No project license detected |
| LIC002 | Critical/Warning | Dependency license incompatible or not allowed |
| LIC003 | Info | Dependency uses unknown/unrecognized license |
| LIC004 | Warning | Dependency has no license specified |

Supported dependency files:
- `Cargo.toml` (Rust)
- `package.json` / `node_modules/*/package.json` (Node.js)
- `requirements.txt` (Python)
- `go.mod` (Go)

Configure allowed and denied licenses in `.repolens.toml`:

```toml
["rules.licenses"]
enabled = true
allowed_licenses = ["MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC"]
denied_licenses = ["GPL-3.0", "AGPL-3.0"]
```

## GitHub Action

RepoLens is available as a GitHub Action to integrate repository auditing directly into your CI/CD workflows.

### Basic Usage

```yaml
name: RepoLens Audit
on: [push, pull_request]

jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: systm-d/repolens@main
        with:
          preset: 'opensource'
          format: 'terminal'
          fail-on: 'critical'
```

### Inputs

| Input | Description | Default |
|-------|-------------|---------|
| `preset` | Audit preset (`opensource`, `enterprise`, `strict`) | `opensource` |
| `format` | Output format (`terminal`, `json`, `sarif`, `markdown`, `html`) | `terminal` |
| `fail-on` | Fail on severity level (`critical`, `high`, `medium`, `low`, `none`) | `critical` |
| `config` | Path to a custom `.repolens.toml` config file | |
| `version` | RepoLens version to install (e.g. `1.0.0` or `latest`) | `latest` |
| `upload-artifact` | Upload report as a GitHub Actions artifact | `true` |
| `artifact-name` | Name of the uploaded artifact | `repolens-report` |

### Outputs

| Output | Description |
|--------|-------------|
| `report-path` | Path to the generated report file |
| `findings-count` | Total number of findings detected |
| `exit-code` | Exit code (`0`=success, `1`=critical, `2`=warnings) |

### SARIF Integration

Upload results to GitHub Advanced Security for visibility in the Security tab:

```yaml
- uses: systm-d/repolens@main
  id: audit
  with:
    format: 'sarif'
    fail-on: 'none'

- uses: github/codeql-action/upload-sarif@v3
  if: always()
  with:
    sarif_file: ${{ steps.audit.outputs.report-path }}
    category: 'repolens'
```

## CI/CD Integration

RepoLens integrates with all major CI/CD platforms. See [docs/ci-cd-integration.md](docs/ci-cd-integration.md) for detailed integration guides and examples.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## Development

See [DEVELOPMENT.md](DEVELOPMENT.md) for development setup, architecture, and contribution guidelines.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
