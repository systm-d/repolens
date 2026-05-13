# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [2.0.0] - 2026-05-13

### Removed (BREAKING CHANGES)

- **Output formats**: PDF (with branding), CSV, TSV, NDJSON, JUnit XML have been removed.
  Use JSON, Markdown, HTML, or SARIF instead.
- **CLI flags**: `--csv-delimiter`, `--csv-bom`, `--csv-keep-newlines`, `--branding` (PDF) have been removed.
- **`integrations/` directory** — template YAML for Azure DevOps, CircleCI, GitLab CI,
  Jenkins, and GitHub Actions removed. They were never backed by Rust provider implementations.
- **`examples/github-action/`** — referenced a nonexistent published GitHub Action.
- **Dependencies removed**: `printpdf`, `image`, `csv`, `quick-xml`, `lopdf` (dev). Compile
  times reduced accordingly.

### Changed

- Refocused on the core mission: audit GitHub repositories for best practices,
  security, and compliance. See `docs/superpowers/specs/2026-05-13-repolens-recentering-design.md`
  for the full rationale.
- Remaining output formats: terminal, JSON, Markdown, SARIF, HTML.

### Migration

If you used a removed format, your options are:

- **CSV / TSV / NDJSON** → use `--format json` and post-process with `jq` or your tool of choice.
- **JUnit XML** → use `--format sarif` (broader CI/CD support and a richer schema).
- **PDF with branding** → use `--format html` and print to PDF from your browser, or pipe to `wkhtmltopdf` / `weasyprint`.

## [1.4.0] - 2026-05-13

### Changed

- Migrated repository to `systm-d` GitHub organization (`github.com/systm-d/repolens`)
- Updated all URLs, Docker image references (`ghcr.io/systm-d/repolens`), packaging metadata (Homebrew, Scoop, AUR, Debian), and CI/CD integration examples

### Added

#### Repository Hygiene Rules
- New **metadata** rule category for repository metadata checks
  - `META001` (Info): Repository description is missing
  - `META002` (Info): No topics or tags configured
  - `META003` (Info): Website URL is not configured
  - `META004` (Info): Social preview image is missing
- New **issues** rule category for issue and PR hygiene
  - `ISSUE001` (Info): Stale issues (> 90 days without activity)
  - `ISSUE002` (Info): Stale pull requests (> 30 days without activity)
  - `ISSUE003` (Info): Issues without labels
  - `PR001` (Warning): Pull requests without reviewers assigned
  - `PR002` (Info): Abandoned draft PRs (> 14 days without activity)
- New **history** rule category for Git history quality
  - `HIST001` (Info): Commits not following conventional commit format
  - `HIST002` (Warning): Giant commits (> 50 files changed)
  - `HIST003` (Info): Unsigned commits (no GPG/SSH signature)
  - `HIST004` (Warning): Force push events detected

### Fixed

- Resolved 3 clippy warnings that broke the nightly build quality gate
- Pinned Docker alpine base image to 3.22 instead of `latest` tag
- Fixed false positive WF001 secret detection in sync-wiki workflow
- Configured secret scan exclusions for test/pattern/CI files in `.repolens.toml`
- Used git credential helper instead of embedding tokens in workflow URLs

## [1.3.0] - 2026-03-11

### Added

#### Security & Governance Rules
- GitHub security features audit (SEC011-014): vulnerability alerts, Dependabot, secret scanning, push protection
- GitHub Actions permissions audit (SEC015-017): allowed actions, workflow permissions, fork PR policies
- Access control audit (TEAM001-004, KEY001-002, APP001): collaborators, teams, deploy keys, installed apps
- Infrastructure audit (HOOK001-003, ENV001-003): webhooks, environments, protection rules
- CODEOWNERS validation (CODE001-003): presence, syntax, valid owners
- Release practices audit (REL001-003): releases, stale releases, unsigned tags

## [1.2.0] - 2026-02-11

### Added

#### Performance
- Comprehensive benchmark suite using Criterion
  - `scanner_benchmark.rs`: File system scanning performance tests
  - `rules_benchmark.rs`: Rules engine performance tests
  - `parse_benchmark.rs`: Parser performance tests with small/medium/large datasets
- Benchmarks cover file iteration, content reading, pattern matching, and rule execution

#### GitHub API Integration
- Direct GitHub API access via octocrab library
- GITHUB_TOKEN environment variable authentication (preferred method)
- Automatic fallback to gh CLI when octocrab unavailable
- New `get_repo_settings()` using octocrab with gh CLI fallback
- New `create_issue()` using octocrab with gh CLI fallback
- New prerequisite checks for GitHub authentication:
  - `check_github_token()`: Verifies GITHUB_TOKEN is set
  - `check_github_auth_available()`: Checks for any available auth method
- Utility functions: `is_github_token_available()`, `is_github_auth_available()`

### Changed

- GitHub CLI (gh) is now optional when GITHUB_TOKEN is set
- Prerequisites system updated to prefer GITHUB_TOKEN over gh CLI
- gh CLI checks changed from Required to Optional level

## [1.1.2] - 2026-02-11

### Fixed

- Improved OSV API error handling by filtering packages with empty names or versions
- Added debug logging for failed OSV API requests to help diagnose issues

## [1.1.1] - 2026-02-11

### Fixed

- Fixed scanner including directories in file list, causing "Is a directory" errors when scanning repositories with directories named like files (e.g., `openapi.json/`)

## [1.1.0] - 2026-02-11

### Added

#### Documentation
- Comprehensive module-level documentation with examples for all public APIs
- Enhanced contribution guide with detailed workflow and code standards
- Rustdoc documentation for all public types and functions

#### Testing
- 932 unit tests (up from 850)
- Test coverage at 86.63% for unit-testable code
- Tarpaulin configuration for accurate coverage reporting
- New tests for CLI commands, parsers, and utilities

### Changed

- Updated `regex` dependency to 1.12 for improved performance
- Refactored code organization for better maintainability
- Improved error messages and documentation strings

### Fixed

- Minor code quality improvements identified by enhanced test coverage

## [1.0.0] - 2026-02-08

### Added

#### Core Audit Engine
- Multi-category rule engine with parallel execution
- 50+ audit rules across 11 categories: secrets, files, docs, security, workflows, quality, dependencies, licenses, docker, git, custom
- Configurable presets: `opensource`, `enterprise`, `strict`
- Custom rules support via regex patterns or shell commands
- JSON Schema validation for audit reports

#### Security Features
- Secret detection (AWS, GitHub tokens, private keys, passwords, etc.)
- Dependency vulnerability scanning via OSV API (9 ecosystems supported)
- License compliance checking with allow/deny lists
- Branch protection validation
- Secure file permissions for configuration files (chmod 600 on Unix)

#### Supported Ecosystems
- **Rust** (Cargo): `Cargo.toml` / `Cargo.lock`
- **Node.js** (npm/yarn): `package.json` / `package-lock.json` / `yarn.lock`
- **Python** (pip/poetry): `requirements.txt` / `Pipfile` / `pyproject.toml`
- **Go**: `go.mod` / `go.sum`
- **Java** (Maven/Gradle): `pom.xml` / `build.gradle`
- **PHP** (Composer): `composer.json` / `composer.lock`
- **.NET** (NuGet): `*.csproj` / `packages.lock.json`
- **Ruby** (Bundler): `Gemfile` / `Gemfile.lock`
- **Dart/Flutter** (Pub): `pubspec.yaml` / `pubspec.lock`

#### CLI Features
- Commands: `init`, `plan`, `apply`, `report`, `compare`, `schema`, `install-hooks`, `generate-man`
- Output formats: Terminal (colored), JSON, SARIF, Markdown, HTML
- Verbose mode with timing breakdown (`-v`, `-vv`, `-vvv`)
- Directory option (`-C/--directory`) to audit different directories
- Environment variables configuration (`REPOLENS_*`)
- Standardized exit codes (0-4) for CI/CD integration
- Man page generation

#### Caching & Performance
- SHA256-based cache invalidation for faster audits
- Configurable cache TTL and directory
- `--no-cache` and `--clear-cache` options

#### Git Integration
- Pre-commit hook (secrets detection)
- Pre-push hook (full audit)
- Git hygiene rules (large binaries, gitattributes, sensitive files)

#### GitHub Integration
- Branch protection validation
- Automatic issue creation for warnings
- Repository settings validation
- GitHub Actions workflow validation

#### Compare & Diff
- Compare two audit reports
- Detect regressions and improvements
- Score-based comparison (Critical=10, Warning=3, Info=1)
- Multiple output formats (terminal, JSON, Markdown)

#### Distribution
- Docker image (multi-arch: amd64, arm64) on GitHub Container Registry
- Homebrew formula (macOS/Linux)
- Scoop manifest (Windows)
- AUR package (Arch Linux)
- Debian packaging

#### CI/CD Integration Templates
- GitHub Actions reusable workflow
- GitLab CI configuration
- CircleCI configuration
- Jenkins declarative pipeline
- Azure DevOps pipeline

#### Documentation
- Comprehensive wiki documentation
- Installation guide for all platforms
- Custom rules documentation with security considerations
- CI/CD integration guide

### Security

- CVE-2026-0007: Fixed integer overflow in `bytes` crate (updated to 1.11.1)
- Secure file permissions for `.repolens.toml`
- Shell command injection warnings for custom rules

### Testing

- 850+ unit tests
- 56 E2E integration tests
- Comprehensive error handling tests

---

[1.0.0]: https://github.com/systm-d/repolens/releases/tag/v1.0.0
