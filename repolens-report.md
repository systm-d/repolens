# RepoLens Audit Report

**Repository:** systm-d/repolens
**Preset:** opensource
**Generated:** 2026-05-13 12:00:20 UTC
**RepoLens Version:** 2.0.0

## Summary

| Severity | Count |
|----------|-------|
| Critical | 0 |
| Warning | 70 |
| Info | 21 |

## Warnings

These issues should be addressed.

### DOC005 - CONTRIBUTING file is missing

### DOC006 - CODE_OF_CONDUCT file is missing

### DOC007 - SECURITY policy file is missing

### WF002 - Workflow missing explicit permissions

**Location:** `.github/workflows/ci.yml`

### WF004 - Job 'test' missing timeout-minutes

**Location:** `.github/workflows/ci.yml`

### WF004 - Job 'coverage' missing timeout-minutes

**Location:** `.github/workflows/ci.yml`

### WF004 - Job 'security' missing timeout-minutes

**Location:** `.github/workflows/ci.yml`

### WF004 - Job 'package' missing timeout-minutes

**Location:** `.github/workflows/ci.yml`

### WF004 - Job 'build' missing timeout-minutes

**Location:** `.github/workflows/release.yml`

### WF004 - Job 'changelog' missing timeout-minutes

**Location:** `.github/workflows/release.yml`

### WF004 - Job 'release' missing timeout-minutes

**Location:** `.github/workflows/release.yml`

### WF004 - Job 'publish-crate' missing timeout-minutes

**Location:** `.github/workflows/release.yml`

### WF004 - Job 'docker' missing timeout-minutes

**Location:** `.github/workflows/release.yml`

### DEP001-GHSA-h395-gr6q-cpjc - Vulnerability GHSA-h395-gr6q-cpjc found in jsonwebtoken 9.3.1

**Location:** `Cargo.lock`

### DEP001-GHSA-cq8v-f236-94qc - Vulnerability GHSA-cq8v-f236-94qc found in rand 0.8.5

**Location:** `Cargo.lock`

### DEP001-GHSA-82j2-j2ch-gfr8 - Vulnerability GHSA-82j2-j2ch-gfr8 found in rustls-webpki 0.101.7

**Location:** `Cargo.lock`

### DEP001-GHSA-965h-392x-2mh5 - Vulnerability GHSA-965h-392x-2mh5 found in rustls-webpki 0.101.7

**Location:** `Cargo.lock`

### DEP001-GHSA-xgp8-3hg3-c2mh - Vulnerability GHSA-xgp8-3hg3-c2mh found in rustls-webpki 0.101.7

**Location:** `Cargo.lock`

### LIC004 - Dependency 'anyhow' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'async-trait' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'chrono' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'clap' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'clap_complete' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'clap_complete_nushell' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'clap_mangen' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'colored' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'console' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'dialoguer' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'dirs' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'globset' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'ignore' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'indicatif' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'jsonschema' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'lazy_static' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'metrics' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'metrics-prometheus' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'minijinja' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'octocrab' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'rayon' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'regex' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'reqwest' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'serde' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'serde_json' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'serde_yaml' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'sha2' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'similar' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'tera' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'thiserror' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'tokio' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'toml' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'tracing' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'tracing-subscriber' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'url' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'walkdir' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'which' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'assert_cmd' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'criterion' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'futures' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'insta' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'predicates' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'pretty_assertions' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'serial_test' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'tempfile' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'clap' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'clap_complete' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'clap_complete_nushell' has no license specified

**Location:** `Cargo.toml`

### LIC004 - Dependency 'clap_mangen' has no license specified

**Location:** `Cargo.toml`

### DOCKER005 - No HEALTHCHECK instruction in Dockerfile

**Location:** `Dockerfile`

### GIT003 - Sensitive file 'src/rules/categories/secrets.rs' may be tracked in repository

**Location:** `src/rules/categories/secrets.rs`

### GIT003 - Sensitive file 'src/rules/patterns/secrets.rs' may be tracked in repository

**Location:** `src/rules/patterns/secrets.rs`

---

*Report generated by [RepoLens](https://github.com/systm-d/repolens)*
