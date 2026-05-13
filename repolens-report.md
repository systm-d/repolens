# RepoLens Audit Report

**Repository:** systm-d/repolens
**Preset:** opensource
**Generated:** 2026-05-13 11:34:38 UTC
**RepoLens Version:** 2.0.0

## Summary

| Severity | Count |
|----------|-------|
| Critical | 4 |
| Warning | 128 |
| Info | 27 |

## Critical Issues

These issues must be resolved before proceeding.

### DEP002-GHSA-97wc-2hqc-cjgr - Vulnerability GHSA-97wc-2hqc-cjgr (CVSS: 7.3) found in ahash 0.7.8

**Location:** `Cargo.lock`

### DEP002-GHSA-fpf5-4jw8-67x8 - Vulnerability GHSA-fpf5-4jw8-67x8 (CVSS: 7.5) found in ahash 0.7.8

**Location:** `Cargo.lock`

### DEP002-GHSA-f89h-2fjh-2r9q - Vulnerability GHSA-f89h-2fjh-2r9q (CVSS: 7.8) found in ahash 0.7.8

**Location:** `Cargo.lock`

### DEP002-GHSA-89vp-x53w-74fx - Vulnerability GHSA-89vp-x53w-74fx (CVSS: 8.8) found in ahash 0.7.8

**Location:** `Cargo.lock`

## Warnings

These issues should be addressed.

### DOC005 - CONTRIBUTING file is missing

### DOC006 - CODE_OF_CONDUCT file is missing

### DOC007 - SECURITY policy file is missing

### SEC011 - Vulnerability alerts are disabled

### SEC012 - Dependabot security updates are disabled

### SEC015 - GitHub Actions allows all actions

### WF002 - Workflow missing explicit permissions

**Location:** `.github/workflows/ci.yml`

### WF004 - Job 'check-ci-success' missing timeout-minutes

**Location:** `.github/workflows/code-quality.yml`

### WF004 - Job 'dependency-audit' missing timeout-minutes

**Location:** `.github/workflows/code-quality.yml`

### WF004 - Job 'security-audit' missing timeout-minutes

**Location:** `.github/workflows/code-quality.yml`

### WF004 - Job 'outdated-dependencies' missing timeout-minutes

**Location:** `.github/workflows/code-quality.yml`

### WF004 - Job 'clippy-analysis' missing timeout-minutes

**Location:** `.github/workflows/code-quality.yml`

### WF004 - Job 'code-metrics' missing timeout-minutes

**Location:** `.github/workflows/code-quality.yml`

### WF004 - Job 'semgrep-analysis' missing timeout-minutes

**Location:** `.github/workflows/code-quality.yml`

### WF004 - Job '# codeql-analysis' missing timeout-minutes

**Location:** `.github/workflows/code-quality.yml`

### WF004 - Job '#   permissions' missing timeout-minutes

**Location:** `.github/workflows/code-quality.yml`

### WF004 - Job '#   steps' missing timeout-minutes

**Location:** `.github/workflows/code-quality.yml`

### WF004 - Job '#       with' missing timeout-minutes

**Location:** `.github/workflows/code-quality.yml`

### WF004 - Job '# sonarcloud' missing timeout-minutes

**Location:** `.github/workflows/code-quality.yml`

### WF004 - Job '#   steps' missing timeout-minutes

**Location:** `.github/workflows/code-quality.yml`

### WF004 - Job '#       with' missing timeout-minutes

**Location:** `.github/workflows/code-quality.yml`

### WF004 - Job '#       env' missing timeout-minutes

**Location:** `.github/workflows/code-quality.yml`

### WF004 - Job 'quality-report' missing timeout-minutes

**Location:** `.github/workflows/code-quality.yml`

### WF004 - Job 'calculate-version' missing timeout-minutes

**Location:** `.github/workflows/create-release.yml`

### WF004 - Job 'check-ci' missing timeout-minutes

**Location:** `.github/workflows/create-release.yml`

### WF004 - Job 'check-code-quality' missing timeout-minutes

**Location:** `.github/workflows/create-release.yml`

### WF004 - Job 'prepare-release' missing timeout-minutes

**Location:** `.github/workflows/create-release.yml`

### WF004 - Job 'check-prerequisites' missing timeout-minutes

**Location:** `.github/workflows/nightly.yml`

### WF004 - Job 'quality-gates' missing timeout-minutes

**Location:** `.github/workflows/nightly.yml`

### WF004 - Job 'build' missing timeout-minutes

**Location:** `.github/workflows/nightly.yml`

### WF004 - Job 'skip-notification' missing timeout-minutes

**Location:** `.github/workflows/nightly.yml`

### WF004 - Job 'build-and-push' missing timeout-minutes

**Location:** `.github/workflows/docker.yml`

### WF004 - Job 'verify' missing timeout-minutes

**Location:** `.github/workflows/docker.yml`

### WF004 - Job 'build' missing timeout-minutes

**Location:** `.github/workflows/release.yml`

### WF004 - Job 'changelog' missing timeout-minutes

**Location:** `.github/workflows/release.yml`

### WF004 - Job 'release' missing timeout-minutes

**Location:** `.github/workflows/release.yml`

### WF004 - Job 'publish-crate' missing timeout-minutes

**Location:** `.github/workflows/release.yml`

### WF004 - Job 'announce-discussion' missing timeout-minutes

**Location:** `.github/workflows/release.yml`

### WF004 - Job 'check' missing timeout-minutes

**Location:** `.github/workflows/ci.yml`

### WF004 - Job 'fmt' missing timeout-minutes

**Location:** `.github/workflows/ci.yml`

### WF004 - Job 'clippy' missing timeout-minutes

**Location:** `.github/workflows/ci.yml`

### WF004 - Job 'test' missing timeout-minutes

**Location:** `.github/workflows/ci.yml`

### WF004 - Job 'pdf-benchmark' missing timeout-minutes

**Location:** `.github/workflows/ci.yml`

### WF004 - Job 'package' missing timeout-minutes

**Location:** `.github/workflows/ci.yml`

### WF004 - Job 'coverage' missing timeout-minutes

**Location:** `.github/workflows/ci.yml`

### WF004 - Job 'security' missing timeout-minutes

**Location:** `.github/workflows/ci.yml`

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

### DEP002-GHSA-88q9-cmp2-c2vq - Vulnerability GHSA-88q9-cmp2-c2vq (CVSS: 4.3) found in ahash 0.7.8

**Location:** `Cargo.lock`

### DEP002-GHSA-g588-cjg3-6g78 - Vulnerability GHSA-g588-cjg3-6g78 found in ahash 0.7.8

**Location:** `Cargo.lock`

### DEP002-GHSA-h9hm-m2xj-4rq9 - Vulnerability GHSA-h9hm-m2xj-4rq9 found in ahash 0.7.8

**Location:** `Cargo.lock`

### DEP002-GHSA-pvmv-cwg8-v6c8 - Vulnerability GHSA-pvmv-cwg8-v6c8 found in ahash 0.7.8

**Location:** `Cargo.lock`

### DEP002-GHSA-xv59-967r-8726 - Vulnerability GHSA-xv59-967r-8726 found in ahash 0.7.8

**Location:** `Cargo.lock`

### DEP002-GHSA-cwfq-rfcr-8hmp - Vulnerability GHSA-cwfq-rfcr-8hmp found in ahash 0.7.8

**Location:** `Cargo.lock`

### DEP002-GHSA-gq4h-3grw-2rhv - Vulnerability GHSA-gq4h-3grw-2rhv found in ahash 0.7.8

**Location:** `Cargo.lock`

### DEP002-GHSA-438q-jx8f-cccv - Vulnerability GHSA-438q-jx8f-cccv (CVSS: 5.3) found in ahash 0.7.8

**Location:** `Cargo.lock`

### DEP002-GHSA-jv4h-j224-23cc - Vulnerability GHSA-jv4h-j224-23cc found in ahash 0.7.8

**Location:** `Cargo.lock`

### DEP002-GHSA-w5p8-4jcx-2j6r - Vulnerability GHSA-w5p8-4jcx-2j6r found in ahash 0.7.8

**Location:** `Cargo.lock`

### DEP002-GHSA-qg8r-f7x3-25f7 - Vulnerability GHSA-qg8r-f7x3-25f7 found in ahash 0.7.8

**Location:** `Cargo.lock`

### DEP002-GHSA-5qv7-j6w5-fr4m - Vulnerability GHSA-5qv7-j6w5-fr4m found in ahash 0.7.8

**Location:** `Cargo.lock`

### DEP002-GHSA-q2qq-hmj6-3wpp - Vulnerability GHSA-q2qq-hmj6-3wpp found in ahash 0.7.8

**Location:** `Cargo.lock`

### DEP002-GHSA-3v94-mw7p-v465 - Vulnerability GHSA-3v94-mw7p-v465 found in ahash 0.7.8

**Location:** `Cargo.lock`

### DEP002-GHSA-p8xm-42r7-89xg - Vulnerability GHSA-p8xm-42r7-89xg found in ahash 0.7.8

**Location:** `Cargo.lock`

### DEP002-GHSA-ff9q-rm55-q7qr - Vulnerability GHSA-ff9q-rm55-q7qr found in ahash 0.7.8

**Location:** `Cargo.lock`

### DEP002-GHSA-qxrw-f6fh-34r7 - Vulnerability GHSA-qxrw-f6fh-34r7 found in ahash 0.7.8

**Location:** `Cargo.lock`

### DEP002-GHSA-qcxq-75wr-5cm8 - Vulnerability GHSA-qcxq-75wr-5cm8 found in ahash 0.7.8

**Location:** `Cargo.lock`

### DEP002-GHSA-84jc-3hj2-hwc7 - Vulnerability GHSA-84jc-3hj2-hwc7 found in ahash 0.7.8

**Location:** `Cargo.lock`

### DEP002-GHSA-r5fr-9gmv-jggh - Vulnerability GHSA-r5fr-9gmv-jggh found in ahash 0.7.8

**Location:** `Cargo.lock`

### DEP002-GHSA-gpxg-fx2g-qxj2 - Vulnerability GHSA-gpxg-fx2g-qxj2 (CVSS: 6.1) found in ahash 0.7.8

**Location:** `Cargo.lock`

### DEP002-GHSA-22w3-693w-x895 - Vulnerability GHSA-22w3-693w-x895 found in ahash 0.7.8

**Location:** `Cargo.lock`

### DEP002-GHSA-95q8-x6r6-672m - Vulnerability GHSA-95q8-x6r6-672m (CVSS: 5.3) found in ahash 0.7.8

**Location:** `Cargo.lock`

### DEP002-GHSA-jmxc-hhwx-gvv3 - Vulnerability GHSA-jmxc-hhwx-gvv3 (CVSS: 5.3) found in ahash 0.7.8

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

### HIST002 - 1 commit(s) with more than 50 files changed

---

*Report generated by [RepoLens](https://github.com/systm-d/repolens)*
