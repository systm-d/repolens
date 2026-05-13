# RepoLens Installation Guide

This guide covers all available methods to install RepoLens on your system.

## Quick Install

### Using Cargo (All Platforms)

If you have Rust installed, the easiest way to install RepoLens is via Cargo:

```bash
cargo install repolens
```

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
