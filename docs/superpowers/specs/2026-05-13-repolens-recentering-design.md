# RepoLens — Recentering Design (v2.0.0)

**Status:** Approved
**Date:** 2026-05-13
**Driver:** Kevin Delfour

## Context

RepoLens has drifted from its initial scope. The repository was started as **a CLI tool to audit GitHub repositories for best practices, security, and compliance**. Over the last six months development has shifted toward output-format proliferation and a planned (but unshipped) multi-provider story:

- **10 output formats** for a single provider (GitHub): terminal, JSON, Markdown, HTML, SARIF, CSV, TSV, NDJSON, JUnit XML, PDF (with branding)
- **The 4 most recent feature commits** all added output formats (PDF branding #232, JUnit #235, completions #234, CSV/TSV/NDJSON #233)
- The PDF formatter alone is **1 351 LOC** (31% of the entire output layer)
- `integrations/` directory contains template YAML for Azure DevOps, CircleCI, GitLab CI, Jenkins, GitHub Actions — **zero Rust code backing them**
- CLAUDE.md is **two versions stale** (claims 6 rule categories; reality is 16)

The output proliferation and PDF branding signal an unspoken pivot toward white-label reporting / B2B consulting. That's not what RepoLens is supposed to be.

## Decision

Recenter on the core mission. Cut the reporting layer to a defensible minimum. Align engineering practices on the sibling project `guardians`. Ship as **v2.0.0** with a single breaking-change release (no deprecation cycle — project has no significant external user base yet).

## Scope: What stays, what goes

### Kept (core mission)

- **CLI commands:** `init`, `plan`, `apply`, `report`, `compare`, `install-hooks`, `schema`, `completions`, `generate-man`
- **Output formats (5):** terminal, JSON, Markdown, SARIF, HTML
- **Rules engine:** all 16 categories (secrets, files, docs, security, workflows, quality, licenses, dependencies, docker, git, codeowners, custom, metadata, issues, history, repository-hygiene)
- **Provider:** GitHub (octocrab + gh CLI fallback)
- **Packaging:** Homebrew, AUR, Debian, Scoop, Docker
- **Shell completions, man page generation** — standard CLI hygiene, not drift

### Removed

| Item | Reason | Approx. LOC removed |
|---|---|---|
| PDF formatter (`src/cli/output/pdf*`) | B2B reporting drift; `printpdf` + `image` deps | 1 351 |
| CSV formatter | Adjacent reporting drift | ~200 |
| TSV formatter | Adjacent reporting drift | ~100 |
| NDJSON formatter | Adjacent reporting drift | ~200 |
| JUnit XML formatter | CI integration drift; `quick-xml` dep | ~300 |
| `integrations/` directory | Template graveyard, no implementation | ~1 000 (configs) |
| `examples/github-action/` referencing nonexistent action | Inconsistent with current code | ~100 |
| Associated tests, benches, fixtures | Follow the formatters | ~250 |
| Crates: `printpdf`, `image`, `csv`, `quick-xml` | Free after formatter removal | 4 deps |

**Estimated net removal:** ~3 500 LOC + 4 dependencies + faster compile times.

## Practice alignment with `guardians`

| Dimension | Before (RepoLens 1.4.0) | After (v2.0.0) |
|---|---|---|
| Rust edition | 2021 | **2024** |
| MSRV (`rust-version`) | 1.74 | **1.85** |
| License | MIT | **MIT OR Apache-2.0** (dual) |
| CI workflows | 6 (ci, code-quality, nightly, create-release, release, docker) | **2** (ci, release) |
| CI matrix | unspecified | **ubuntu-22.04, ubuntu-24.04** (mirror guardians) |
| Clippy | `-D warnings` (partial) | `-D warnings` enforced in `ci.yml` |
| CLAUDE.md | brief, stale (lists 6 categories; reality is 16) | comprehensive — load-bearing rules, module discipline, conventions; mirrors guardians depth |
| Docs structure | `docs/` flat + `wiki/` | **`docs/superpowers/specs/`** + **`docs/superpowers/plans/`** per design |
| Test mocking | mixed | **`wiremock`** for HTTP, **`insta`** for snapshot tests |
| Git hooks | 3 custom (pre-commit, pre-push, commit-msg) | **None** — CI is source of truth (mirrors guardians) |

## Implementation strategy

Execution is broken into three sub-plans (each gets its own implementation plan):

1. **Plan A — Scope cuts.** Remove formatters, deps, tests, `integrations/`, stale examples. Verify nothing else depends on removed modules. Bump major to 2.0.0. Update CHANGELOG with breaking-change section.

2. **Plan B — Practice alignment.** Bump edition + MSRV, add `LICENSE-APACHE`, dual-license header, consolidate CI workflows, adopt `wiremock` + `insta` where it makes sense, remove git hooks (uninstall via `repolens install-hooks --uninstall`).

3. **Plan C — Documentation rewrite.** Rewrite CLAUDE.md to guardians depth. Migrate `wiki/` content into `docs/superpowers/specs/`. Update README to drop removed formats. Add a CONTRIBUTING.md, keep CODEOWNERS.

Each plan is independent and committed separately. Together they make v2.0.0.

## Out of scope

- **Multi-provider support** (GitLab, Bitbucket). Either commit to it as v3 or remove the aspirational `integrations/` directory — this design chooses the latter; reintroducing multi-provider would require its own design doc.
- **TUI (`ratatui`)**. Guardians has one; RepoLens has no obvious need for it.
- **Database / migrations**. Guardians is a stateful daemon; RepoLens is stateless audit, no need.
- **Crate ownership transfer on crates.io**. Crate stays owned by `kdelfour` for now; adding `github:systm-d:<team>` as co-owner is a separate decision.

## Validation criteria

v2.0.0 ships only if:

- `cargo check && cargo fmt --check && cargo clippy --all-targets -- -D warnings` clean
- `cargo test --all` green (expected ~950 tests, down from 1100)
- `cargo publish --dry-run` succeeds with new Cargo.toml metadata
- README accurately reflects shipped capabilities
- CLAUDE.md matches reality (rule category count, output format list, conventions)
- No references in code, docs, or packaging to removed formats

## Non-obvious risks

- **Existing report-comparison golden files.** `src/compare/` and snapshot tests may reference removed formats. Audit before merge.
- **Schema definitions.** `schemas/audit-report.schema.json` and `finding.schema.json` may still reference removed formats. Update or version them.
- **The bootstrap problem.** RepoLens audits itself via the pre-push hook. Removing hooks AND bumping the MSRV in the same release means re-running install-hooks after upgrade would need to ship a new hook script. Plan B should explicitly uninstall hooks during migration.
