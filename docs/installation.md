# RepoLens Installation Guide

This guide covers all available methods to install RepoLens on your system.

## Quick Install

### Using Cargo (All Platforms)

If you have Rust installed, the easiest way to install RepoLens is via Cargo:

```bash
cargo install repolens
```

### Docker (Recommended for quick use)

The simplest way to use RepoLens without a local installation:

```bash
# Pull the official image
docker pull ghcr.io/systm-d/repolens:latest

# Audit the current directory
docker run --rm -v "$(pwd)":/repo ghcr.io/systm-d/repolens plan

# Generate a report
docker run --rm -v "$(pwd)":/repo ghcr.io/systm-d/repolens report --format json
```

For GitHub API access, mount your gh configuration:

```bash
docker run --rm \
  -v "$(pwd)":/repo \
  -v ~/.config/gh:/home/repolens/.config/gh:ro \
  ghcr.io/systm-d/repolens plan
```

Available tags:
- `latest` - Latest stable version
- `v1.0.0`, `v1.1.0`, etc. - Specific versions
- `sha-abc1234` - Specific commit

See [docker.md](docker.md) for more details.

### Pre-built Binaries

Download pre-built binaries from the [GitHub Releases](https://github.com/systm-d/repolens/releases) page.

## Platform-Specific Installation

### macOS

#### Homebrew

```bash
# Add the tap (first time only)
brew tap systm-d/repolens

# Install RepoLens
brew install repolens
```

To upgrade to the latest version:

```bash
brew upgrade repolens
```

#### Manual Installation (macOS)

1. Download the appropriate binary from [releases](https://github.com/systm-d/repolens/releases):
   - Apple Silicon (M1/M2/M3): `repolens-aarch64-apple-darwin.tar.gz`
   - Intel: `repolens-x86_64-apple-darwin.tar.gz`

2. Extract and install:
   ```bash
   tar -xzf repolens-*.tar.gz
   sudo mv repolens /usr/local/bin/
   ```

3. Verify installation:
   ```bash
   repolens --version
   ```

### Linux

#### Arch Linux (AUR)

```bash
# Using yay
yay -S repolens

# Using paru
paru -S repolens

# Manual build
git clone https://aur.archlinux.org/repolens.git
cd repolens
makepkg -si
```

#### Debian/Ubuntu

Download the `.deb` package from [releases](https://github.com/systm-d/repolens/releases):

```bash
# Download the package
wget https://github.com/systm-d/repolens/releases/download/v1.0.0/repolens_1.0.0-1_amd64.deb

# Install
sudo dpkg -i repolens_1.0.0-1_amd64.deb

# Install dependencies if needed
sudo apt-get install -f
```

#### Fedora/RHEL/CentOS

Download the `.rpm` package from [releases](https://github.com/systm-d/repolens/releases) (when available):

```bash
sudo rpm -i repolens-1.0.0-1.x86_64.rpm
```

Or build from source (see below).

#### Manual Installation (Linux)

1. Download the appropriate binary from [releases](https://github.com/systm-d/repolens/releases):
   - x86_64: `repolens-x86_64-unknown-linux-gnu.tar.gz`
   - ARM64: `repolens-aarch64-unknown-linux-gnu.tar.gz`

2. Extract and install:
   ```bash
   tar -xzf repolens-*.tar.gz
   sudo mv repolens /usr/local/bin/
   ```

3. Verify installation:
   ```bash
   repolens --version
   ```

### Windows

#### Scoop

```powershell
# Add the bucket (first time only)
scoop bucket add repolens https://github.com/systm-d/scoop-repolens

# Install RepoLens
scoop install repolens
```

To upgrade:

```powershell
scoop update repolens
```

#### Chocolatey

(Coming soon)

#### Manual Installation (Windows)

1. Download `repolens-x86_64-pc-windows-msvc.zip` from [releases](https://github.com/systm-d/repolens/releases)

2. Extract the ZIP file

3. Add the extracted directory to your PATH, or move `repolens.exe` to a directory in your PATH

4. Verify installation:
   ```powershell
   repolens --version
   ```

### Verifying Checksums

Each release includes a `checksums.sha256` file to verify archive integrity:

```bash
# Download the checksums file
curl -LO https://github.com/systm-d/repolens/releases/latest/download/checksums.sha256

# Verify (Linux)
sha256sum -c checksums.sha256 --ignore-missing

# Verify (macOS)
shasum -a 256 -c checksums.sha256 --ignore-missing
```

## Using as a GitHub Action

RepoLens is available as an official GitHub Action to integrate auditing directly into your CI/CD workflows.

### Basic Usage

```yaml
name: RepoLens Audit
on: [push, pull_request]

jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: systm-d/repolens-action@v1
        with:
          preset: opensource
```

### Available Inputs

| Input | Description | Default |
|---|---|---|
| `preset` | Configuration preset (`opensource`, `enterprise`, `strict`) | `opensource` |
| `format` | Output format (`terminal`, `json`, `sarif`, `markdown`, `html`) | `terminal` |
| `output` | Output file path | - |
| `categories` | Categories to audit (comma-separated) | all |
| `exclude` | Categories to exclude (comma-separated) | - |
| `verbose` | Verbosity level (`0`-`3`) | `0` |
| `fail-on-error` | Fail the workflow if issues are detected | `false` |

### Available Outputs

| Output | Description |
|---|---|
| `score` | Overall audit score |
| `report-path` | Path to the generated report |
| `issues-count` | Number of issues detected |

### Advanced Example with SARIF Upload

```yaml
name: RepoLens Security Audit
on: [push]

jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: systm-d/repolens-action@v1
        id: audit
        with:
          preset: strict
          format: sarif
          output: repolens-results.sarif
          fail-on-error: true
      - uses: github/codeql-action/upload-sarif@v3
        if: always()
        with:
          sarif_file: repolens-results.sarif
```

### Multi-Preset Audit Example

```yaml
name: RepoLens Multi-Preset Audit
on: [pull_request]

jobs:
  audit:
    runs-on: ubuntu-latest
    strategy:
      matrix:
        preset: [opensource, enterprise, strict]
    steps:
      - uses: actions/checkout@v4
      - uses: systm-d/repolens-action@v1
        with:
          preset: ${{ matrix.preset }}
          format: markdown
          output: report-${{ matrix.preset }}.md
```

See [ci-cd-integration.md](ci-cd-integration.md) for more CI/CD integration examples.

## Building from Source

### Prerequisites

- [Rust](https://rustup.rs/) 1.74 or later
- Git
- OpenSSL development libraries (Linux only)

#### Installing Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

#### Installing OpenSSL (Linux)

Ubuntu/Debian:
```bash
sudo apt-get install libssl-dev pkg-config
```

Fedora/RHEL:
```bash
sudo dnf install openssl-devel
```

Arch Linux:
```bash
sudo pacman -S openssl
```

### Build Steps

```bash
# Clone the repository
git clone https://github.com/systm-d/repolens.git
cd repolens

# Build in release mode
cargo build --release

# The binary will be at target/release/repolens
./target/release/repolens --version

# Optional: Install to ~/.cargo/bin
cargo install --path .
```

## Verifying Installation

After installation, verify RepoLens is working:

```bash
# Check version
repolens --version

# Show help
repolens --help

# Initialize in a Git repository
cd /path/to/your/repo
repolens init
```

## Optional Dependencies

### GitHub CLI (gh)

RepoLens uses the GitHub CLI for some features. Install it for full functionality:

- macOS: `brew install gh`
- Linux: See [GitHub CLI installation](https://github.com/cli/cli#installation)
- Windows: `scoop install gh` or `choco install gh`

After installing, authenticate:

```bash
gh auth login
```

## Updating RepoLens

### Cargo

```bash
cargo install repolens --force
```

### Homebrew

```bash
brew upgrade repolens
```

### Scoop

```powershell
scoop update repolens
```

### AUR

```bash
yay -Syu repolens
```

## Uninstalling

### Cargo

```bash
cargo uninstall repolens
```

### Homebrew

```bash
brew uninstall repolens
```

### Scoop

```powershell
scoop uninstall repolens
```

### Debian/Ubuntu

```bash
sudo apt-get remove repolens
```

### Arch Linux

```bash
sudo pacman -R repolens
```

## Troubleshooting

### "command not found" after installation

Ensure the installation directory is in your PATH:

- **Cargo**: Add `~/.cargo/bin` to your PATH
- **Homebrew**: Usually automatic, try restarting your terminal
- **Manual**: Ensure the binary location is in your PATH

### Permission denied

```bash
chmod +x /path/to/repolens
```

### Build errors (from source)

```bash
# Clean and rebuild
cargo clean
cargo build --release
```

### Dependency issues (from source)

```bash
# Update dependencies
cargo update

# Check dependency tree
cargo tree
```

### OpenSSL errors on Linux

Install OpenSSL development libraries (see prerequisites above).

### Git not found

RepoLens requires Git to be installed and in your PATH:

```bash
# Verify Git is installed
git --version
```

## Getting Help

- [GitHub Issues](https://github.com/systm-d/repolens/issues)
- [Documentation](https://github.com/systm-d/repolens#readme)
