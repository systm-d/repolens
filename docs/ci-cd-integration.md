# CI/CD Integration Guide

This guide provides comprehensive documentation for integrating RepoLens into your CI/CD pipelines across various platforms.

## Table of Contents

- [Overview](#overview)
- [Installation Methods](#installation-methods)
- [GitHub Actions](#github-actions)
- [GitLab CI/CD](#gitlab-cicd)
- [CircleCI](#circleci)
- [Jenkins](#jenkins)
- [Azure DevOps Pipelines](#azure-devops-pipelines)
- [Configuration Options](#configuration-options)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Overview

RepoLens can be integrated into any CI/CD platform to automatically audit your repository for:

- Security vulnerabilities and exposed secrets
- Required files and documentation
- Best practices compliance
- License compatibility
- Code quality standards

### Key Features

- **Fail on severity**: Configure the pipeline to fail based on finding severity
- **Multiple output formats**: JSON, SARIF, HTML, Markdown for different integrations
- **Caching**: Speed up subsequent runs with built-in caching
- **Presets**: Use `opensource`, `enterprise`, or `strict` presets

## Installation Methods

RepoLens can be installed in your CI/CD environment using two approaches:

### Docker Image (Recommended)

```bash
# Pull the official image
docker pull ghcr.io/systm-d/repolens:latest

# Run with your repository mounted
docker run -v $(pwd):/workspace -w /workspace ghcr.io/systm-d/repolens:latest plan
```

Available tags:
- `latest` - Latest stable release
- `v1.0.0` - Specific version
- `nightly` - Latest development build (unstable)

### Binary Installation

```bash
# Determine the latest version
VERSION=$(curl -s https://api.github.com/repos/systm-d/repolens/releases/latest | grep '"tag_name"' | sed -E 's/.*"v([^"]+)".*/\1/')

# Download and install
curl -LO "https://github.com/systm-d/repolens/releases/download/v${VERSION}/repolens-linux-x86_64.tar.gz"
tar xzf repolens-linux-x86_64.tar.gz
sudo mv repolens /usr/local/bin/

# Verify installation
repolens --version
```

## GitHub Actions

GitHub Actions is the recommended platform for RepoLens integration, with a dedicated action available.

### Using the Official Action

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
          format: 'json'
          fail-on: 'critical'
```

### Action Inputs

| Input | Description | Default |
|-------|-------------|---------|
| `preset` | Audit preset (`opensource`, `enterprise`, `strict`) | `opensource` |
| `format` | Output format (`terminal`, `json`, `sarif`, `markdown`, `html`) | `terminal` |
| `fail-on` | Fail on severity (`critical`, `high`, `medium`, `low`, `none`) | `critical` |
| `config` | Path to custom `.repolens.toml` | |
| `version` | RepoLens version | `latest` |
| `upload-artifact` | Upload report as artifact | `true` |
| `artifact-name` | Name of the artifact | `repolens-report` |

### Action Outputs

| Output | Description |
|--------|-------------|
| `report-path` | Path to the generated report file |
| `findings-count` | Total number of findings |
| `exit-code` | Exit code (`0`=success, `1`=critical, `2`=warnings) |

### SARIF Integration with GitHub Security

Upload results to GitHub's Security tab:

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

### Reusable Workflow

Create a reusable workflow in `.github/workflows/repolens.yml`:

```yaml
jobs:
  audit:
    uses: ./.github/workflows/repolens.yml
    with:
      preset: 'enterprise'
      format: 'sarif'
      upload-sarif: true
```

## GitLab CI/CD

### Basic Configuration

Add the following to your `.gitlab-ci.yml`:

```yaml
repolens-audit:
  image: ghcr.io/systm-d/repolens:latest
  stage: test
  script:
    - repolens plan --preset opensource --format json --output audit-report.json
  artifacts:
    reports:
      codequality: audit-report.json
    paths:
      - audit-report.json
    expire_in: 30 days
  rules:
    - if: $CI_PIPELINE_SOURCE == "merge_request_event"
    - if: $CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH
```

### Code Quality Integration

GitLab can display audit results in merge requests using the Code Quality report format:

```yaml
artifacts:
  reports:
    codequality: audit-report.json
```

### Security Scanning (GitLab Ultimate)

For GitLab Ultimate users, use SARIF format for security dashboard integration:

```yaml
repolens-security:
  image: ghcr.io/systm-d/repolens:latest
  stage: test
  script:
    - repolens plan --format sarif --output gl-sast-report.json
  artifacts:
    reports:
      sast: gl-sast-report.json
```

### Caching

Enable caching for faster subsequent runs:

```yaml
cache:
  key: repolens-cache-${CI_COMMIT_REF_SLUG}
  paths:
    - .repolens/cache/
```

## CircleCI

### Basic Configuration

Add the following to your `.circleci/config.yml`:

```yaml
version: 2.1

executors:
  repolens:
    docker:
      - image: ghcr.io/systm-d/repolens:latest

jobs:
  audit:
    executor: repolens
    steps:
      - checkout
      - run:
          name: Run RepoLens Audit
          command: repolens plan --format json --output audit-report.json
      - store_artifacts:
          path: audit-report.json
          destination: repolens-reports

workflows:
  version: 2
  ci:
    jobs:
      - audit
```

### Orb-Style Configuration

The template includes reusable commands and executors following CircleCI orb patterns:

```yaml
commands:
  run-audit:
    parameters:
      preset:
        type: string
        default: "opensource"
    steps:
      - run:
          name: Run RepoLens Audit
          command: repolens plan --preset << parameters.preset >> --format json --output audit-report.json
```

### Caching Binary

Cache the RepoLens binary for faster runs:

```yaml
- restore_cache:
    keys:
      - repolens-binary-v1-{{ .Branch }}

- run: # Install RepoLens if not cached

- save_cache:
    key: repolens-binary-v1-{{ .Branch }}
    paths:
      - /usr/local/bin/repolens
```

## Jenkins

### Declarative Pipeline

Use the following pipeline configuration:

```groovy
pipeline {
    agent {
        docker {
            image 'ghcr.io/systm-d/repolens:latest'
            args '-v ${WORKSPACE}:/workspace -w /workspace'
        }
    }

    stages {
        stage('Audit') {
            steps {
                sh 'repolens plan --format json --output audit-report.json'
            }
        }
    }

    post {
        always {
            archiveArtifacts artifacts: 'audit-report.json', allowEmptyArchive: true
            publishHTML(target: [
                reportDir: '.',
                reportFiles: 'audit-report.html',
                reportName: 'RepoLens Audit Report'
            ])
        }
    }
}
```

### Pipeline Parameters

Allow users to customize audit settings:

```groovy
parameters {
    choice(
        name: 'PRESET',
        choices: ['opensource', 'enterprise', 'strict'],
        description: 'RepoLens audit preset'
    )
    choice(
        name: 'FAIL_ON',
        choices: ['critical', 'high', 'medium', 'low', 'none'],
        description: 'Fail on severity level'
    )
}
```

### Required Plugins

- **Pipeline**: Core pipeline functionality
- **Docker Pipeline**: For Docker agent support
- **HTML Publisher**: For publishing HTML reports
- **Email Extension** (optional): For failure notifications

## Azure DevOps Pipelines

### Basic Configuration

Add the following to your `azure-pipelines.yml`:

```yaml
trigger:
  branches:
    include:
      - main

stages:
  - stage: Audit
    jobs:
      - job: RepoLensAudit
        pool:
          vmImage: 'ubuntu-latest'
        container:
          image: ghcr.io/systm-d/repolens:latest
        steps:
          - checkout: self
          - script: repolens plan --format json --output $(Build.ArtifactStagingDirectory)/audit-report.json
            displayName: 'Run RepoLens Audit'
          - publish: $(Build.ArtifactStagingDirectory)
            artifact: RepoLens-Reports
```

### Pipeline Variables

Configure audit settings via variables:

```yaml
variables:
  repolensPreset: 'opensource'
  repolensFormat: 'json'
  repolensFailOn: 'critical'
```

### Reusable Template

Create a template for use across multiple projects:

```yaml
# templates/repolens-audit.yml
parameters:
  - name: preset
    type: string
    default: 'opensource'

stages:
  - stage: RepoLensAudit
    jobs:
      - job: Audit
        container:
          image: ghcr.io/systm-d/repolens:latest
        steps:
          - script: repolens plan --preset ${{ parameters.preset }}
```

Usage:

```yaml
stages:
  - template: templates/repolens-audit.yml
    parameters:
      preset: 'enterprise'
```

## Configuration Options

### Presets

| Preset | Description | Use Case |
|--------|-------------|----------|
| `opensource` | Standard open-source requirements | Public repositories, community projects |
| `enterprise` | Enterprise security and compliance | Corporate environments, regulated industries |
| `strict` | Maximum security and documentation | High-security applications, financial services |

### Output Formats

| Format | Description | Best For |
|--------|-------------|----------|
| `terminal` | Human-readable console output | Local development |
| `json` | Structured JSON data | Programmatic processing, dashboards |
| `sarif` | Static Analysis Results Interchange Format | Security tool integration |
| `markdown` | Markdown-formatted report | PR comments, documentation |
| `html` | Interactive HTML report | Archiving, sharing |

### Fail-On Severity

| Level | Exit Code | Description |
|-------|-----------|-------------|
| `critical` | 1 | Only fail on critical issues |
| `high` | 1 | Fail on high or critical issues |
| `medium` | 1 | Fail on medium, high, or critical |
| `low` | 1 | Fail on any finding |
| `none` | 0 | Never fail (audit only) |

## Best Practices

### 1. Use Appropriate Presets

- Use `opensource` for public repositories
- Use `enterprise` for internal corporate projects
- Use `strict` for security-critical applications

### 2. Enable Caching

All templates include caching configurations. This significantly speeds up subsequent runs by:
- Caching the RepoLens binary
- Caching audit results for unchanged files

### 3. Run on Pull Requests

Configure audits to run on pull requests for early detection:

```yaml
# GitHub Actions
on:
  pull_request:
    branches: [main]

# GitLab CI
rules:
  - if: $CI_PIPELINE_SOURCE == "merge_request_event"
```

### 4. Archive Reports

Always archive audit reports for historical analysis:

```yaml
# GitHub Actions
- uses: actions/upload-artifact@v4
  with:
    name: repolens-report
    path: audit-report.json

# GitLab CI
artifacts:
  paths:
    - audit-report.json
  expire_in: 90 days
```

### 5. Use SARIF for Security Integration

SARIF format integrates with security dashboards:
- GitHub Security tab
- GitLab Security Dashboard
- Azure DevOps Security Center

### 6. Schedule Nightly Audits

Run comprehensive audits on a schedule:

```yaml
# GitHub Actions
on:
  schedule:
    - cron: '0 2 * * *'

# GitLab CI
rules:
  - if: $CI_PIPELINE_SOURCE == "schedule"
```

## Troubleshooting

### Common Issues

#### RepoLens binary not found

Ensure the binary is in PATH and has execute permissions:

```bash
chmod +x /usr/local/bin/repolens
export PATH="/usr/local/bin:$PATH"
```

#### Docker permission denied

Run Docker commands with appropriate permissions or add the CI user to the docker group.

#### SARIF upload fails

Ensure the SARIF file is valid JSON and follows the SARIF 2.1.0 schema. RepoLens generates compliant SARIF by default.

#### Cache not working

Verify cache paths are correct and the CI platform supports caching:
- GitHub Actions: Uses `actions/cache@v4`
- GitLab CI: Uses `cache` keyword
- CircleCI: Uses `save_cache`/`restore_cache`

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success (no critical findings) |
| 1 | Critical findings detected |
| 2 | Warnings only |
| 127 | RepoLens binary not found |

### Getting Help

- [RepoLens Documentation](https://github.com/systm-d/repolens)
- [Issue Tracker](https://github.com/systm-d/repolens/issues)
- [Discussions](https://github.com/systm-d/repolens/discussions)
