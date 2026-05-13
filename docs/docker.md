# Docker Usage

RepoLens is available as a Docker image for easy deployment without installing dependencies.

## Quick Start

### Pull the Image

```bash
# Pull the latest image from GitHub Container Registry
docker pull ghcr.io/systm-d/repolens:latest

# Or pull a specific version
docker pull ghcr.io/systm-d/repolens:1.0.0
```

### Run RepoLens

```bash
# Run repolens on the current directory
docker run --rm -v "$(pwd)":/repo ghcr.io/systm-d/repolens plan

# Generate a report
docker run --rm -v "$(pwd)":/repo ghcr.io/systm-d/repolens report

# Export HTML report
docker run --rm -v "$(pwd)":/repo ghcr.io/systm-d/repolens report --format html --output /repo/report.html
```

## Using Docker Compose

For easier usage, use the provided `docker-compose.yml`:

```bash
# Build the image locally
docker compose build

# Run repolens commands
docker compose run --rm repolens plan
docker compose run --rm repolens report
docker compose run --rm repolens apply --dry-run
```

## GitHub CLI Authentication

RepoLens uses the GitHub CLI (`gh`) for API operations. To use GitHub features, mount your GitHub CLI config:

```bash
docker run --rm \
  -v "$(pwd)":/repo \
  -v ~/.config/gh:/home/repolens/.config/gh:ro \
  ghcr.io/systm-d/repolens plan
```

Alternatively, pass a GitHub token via environment variable:

```bash
docker run --rm \
  -v "$(pwd)":/repo \
  -e GITHUB_TOKEN=your_token_here \
  ghcr.io/systm-d/repolens plan
```

## Building Locally

To build the Docker image locally:

```bash
# Clone the repository
git clone https://github.com/systm-d/repolens.git
cd cli--repolens

# Build the image
docker build -t repolens .

# Run locally built image
docker run --rm -v "$(pwd)":/repo repolens plan
```

## Available Tags

| Tag | Description |
|-----|-------------|
| `latest` | Latest stable release |
| `X.Y.Z` | Specific version (e.g., `1.0.0`) |
| `X.Y` | Major.minor version (e.g., `1.0`) |
| `X` | Major version (e.g., `1`) |
| `sha-XXXXXX` | Specific commit SHA |

## Multi-Architecture Support

The Docker image supports multiple architectures:

- `linux/amd64` - Standard x86_64 Linux
- `linux/arm64` - ARM64/AArch64 (Apple Silicon, AWS Graviton, etc.)

Docker will automatically pull the correct architecture for your platform.

## Image Details

- **Base image:** Alpine Linux (minimal footprint)
- **Size:** Approximately 50-100 MB
- **User:** Non-root (`repolens:repolens`, UID/GID 1000)
- **Working directory:** `/repo`

### Included Software

- RepoLens binary
- Git (for repository operations)
- GitHub CLI (`gh`) for API operations
- CA certificates for HTTPS

## Examples

### Audit a Repository

```bash
# Initialize configuration
docker run --rm -v "$(pwd)":/repo ghcr.io/systm-d/repolens init --preset opensource

# Generate audit plan
docker run --rm -v "$(pwd)":/repo ghcr.io/systm-d/repolens plan

# View detailed report
docker run --rm -v "$(pwd)":/repo ghcr.io/systm-d/repolens report
```

### Generate Reports

```bash
# JSON report
docker run --rm -v "$(pwd)":/repo ghcr.io/systm-d/repolens report --format json > report.json

# HTML report
docker run --rm -v "$(pwd)":/repo ghcr.io/systm-d/repolens report --format html --output /repo/report.html

# SARIF report (for GitHub Security)
docker run --rm -v "$(pwd)":/repo ghcr.io/systm-d/repolens report --format sarif > report.sarif
```

### Apply Fixes (Dry Run)

```bash
# Preview changes without applying
docker run --rm -v "$(pwd)":/repo ghcr.io/systm-d/repolens apply --dry-run
```

### Interactive Mode

```bash
# Interactive mode requires TTY
docker run --rm -it -v "$(pwd)":/repo ghcr.io/systm-d/repolens apply --interactive
```

## CI/CD Integration

### GitHub Actions

```yaml
jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Run RepoLens audit
        run: |
          docker run --rm \
            -v "${{ github.workspace }}":/repo \
            ghcr.io/systm-d/repolens:latest \
            report --format sarif --output /repo/repolens.sarif

      - name: Upload SARIF
        uses: github/codeql-action/upload-sarif@v3
        with:
          sarif_file: repolens.sarif
```

### GitLab CI

```yaml
audit:
  image: ghcr.io/systm-d/repolens:latest
  script:
    - repolens report
```

## Troubleshooting

### Permission Denied

If you encounter permission errors, ensure the mounted directory is readable:

```bash
# Run with your user's UID/GID
docker run --rm \
  -v "$(pwd)":/repo \
  -u "$(id -u):$(id -g)" \
  ghcr.io/systm-d/repolens plan
```

### GitHub CLI Not Authenticated

Mount your GitHub CLI configuration:

```bash
docker run --rm \
  -v "$(pwd)":/repo \
  -v ~/.config/gh:/home/repolens/.config/gh:ro \
  ghcr.io/systm-d/repolens plan
```

### Git Operations Fail

Ensure the `.git` directory is included in the mount:

```bash
# Mount the entire repository including .git
docker run --rm -v "$(pwd)":/repo ghcr.io/systm-d/repolens plan
```
