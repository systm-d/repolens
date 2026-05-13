# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository status

**v2.0.2.** RepoLens is recentered on GitHub repository auditing: single provider (GitHub), 15 rule
categories, 5 output formats. Rust edition 2024, MSRV 1.85, dual license MIT OR Apache-2.0. Two CI
workflows: `ci.yml` (test matrix + clippy + fmt + coverage + security audit + packaging dry-run) and
`release.yml` (multi-platform binary build + crates.io publish + Docker image push to
`ghcr.io/systm-d/repolens` — `linux/amd64` only since v2.0.2 due to QEMU multi-arch build cost).
Canonical recentering decision:
`docs/superpowers/specs/2026-05-13-repolens-recentering-design.md`.

What's in the codebase right now:

- **CLI surface:** `repolens { init | plan | apply | report | compare | install-hooks | schema | completions | generate-man }`
- **15 rule categories** (all registered in `src/rules/engine.rs`):
  - `secrets` — detect hardcoded secrets, API keys, and credentials in source files
  - `files` — large files, `.gitignore` configuration and recommended entries
  - `docs` — README, LICENSE, and documentation quality checks
  - `security` — security best practices: CODEOWNERS, dependency lock files
  - `workflows` — GitHub Actions: hardcoded secrets, explicit permission declarations
  - `quality` — test directories, linting configuration presence
  - `dependencies` — dependency vulnerability checks via OSV API
  - `licenses` — license presence, SPDX compliance, and dependency license analysis
  - `docker` — Dockerfile presence, `.dockerignore`, pinned base image tags
  - `git` — large binary files, `.gitattributes` presence
  - `custom` — user-defined rules via regex patterns or shell commands in config
  - `codeowners` — CODEOWNERS file presence, validity, and GitHub release tags
  - `history` — conventional commit compliance, giant commit detection
  - `issues` — stale issues and stale pull requests hygiene
  - `metadata` — repository description, topics/tags configuration
- **5 output formats:** terminal (colored), JSON, Markdown, SARIF, HTML
- **Edition 2024, MSRV 1.85**, 1033 unit tests passing
- **2 CI workflows:** `ci.yml` and `release.yml`

## Product in one line

RepoLens audits GitHub repositories for open-source and enterprise compliance; the differentiator is
the **plan/apply split** — every audit produces a typed `ActionPlan` that an operator can review and
selectively apply, rather than executing fixes immediately.

## Architecture

```
src/
├── main.rs            — entry point; dispatches Commands enum
├── lib.rs             — public re-exports
├── cli/               — subcommand dispatch + output formatters
│   ├── commands/       — one file per top-level subcommand
│   └── output/         — terminal, json, markdown, sarif, html
├── config/            — config loading + preset selection
│   └── presets/        — baked-in preset definitions
├── rules/             — rules engine
│   ├── engine.rs       — parallel rule execution, category dispatch
│   ├── categories/     — 15 category modules
│   ├── patterns/       — shared regex and pattern helpers
│   └── results.rs      — Finding + AuditResults types
├── actions/           — ActionPlan + per-action executors
├── cache/             — incremental audit result caching
├── compare/           — diff two audit reports
├── hooks/             — git hook installation and management
├── providers/         — GitHub API (octocrab + gh CLI fallback)
├── scanner/           — filesystem + git tree iteration
└── utils/             — prerequisites checks, exit codes
```

## Module discipline

`rules/` is pure logic — no IO, no network. Each category module receives scanner output and emits
`Vec<Finding>`. New rules must not import from `providers/` or `actions/`.

`actions/` is pure planning: it turns `AuditResults` into a `Vec<Action>`. No network calls, no file
writes happen inside this module. Execution is the job of each action's executor, invoked by the
`apply` command.

`providers/` is the sole boundary with GitHub. All GitHub API calls originate here. No other module
reaches for `octocrab` or shells out to `gh`.

`cli/output/` is presentation only: it renders an `AuditResults` or `ActionPlan` to a string. No
side effects. Each format (terminal, json, markdown, sarif, html) is an independent file.

## Non-obvious constraints

These are load-bearing rules — violating them changes what the product is:

- **Plan/apply split is non-negotiable.** Every fix is computed first as an `Action`, then
  optionally applied. `repolens plan` produces the `ActionPlan`; `repolens apply` executes it.
  They are not the same command with a dry-run flag.
- **Presets are static at build time.** `.repolens.toml` overrides preset values; the preset
  definitions themselves are baked in under `src/config/presets/`. Do not accept arbitrary preset
  names from config — an unknown name is an error, not an implicit rule list.
- **Provider is GitHub-only.** `src/providers/` has exactly one trait implementation. Multi-provider
  work is explicitly out of scope until a new design doc lands (see the recentering spec).
- **No `Co-authored-by` in commits or PRs.** Single primary author per commit. Mentioning
  contributors in the commit body without the `Co-authored-by:` trailer is allowed.
- **No `feat!:` or similar `!` Conventional Commit shorthand.** The commit-msg hook rejects it.
  Use `feat: ...` + `BREAKING CHANGE: ...` in the body instead.
- **GitHub authentication is dual-mode.** `GITHUB_TOKEN` env var is preferred (no `gh` CLI
  required); fallback is `gh auth token`. Both code paths must be tested and exercised in CI.
- **`VALID_CATEGORIES` in `src/rules/constants.rs` must mirror the categories registered in
  `src/rules/engine.rs`.** Adding a category requires updating both files; the unit test in
  `constants.rs` asserts the count. The CLI `--only` / `--skip` flags rely on this constant.

## Working conventions

- Tests live in `tests/` (integration) and inline `#[cfg(test)] mod tests` (unit). Per-rule unit
  tests are inside `src/rules/categories/<category>.rs`.
- `cargo test --all` runs everything. `cargo test --lib` skips integration tests; useful while
  iterating on rule logic.
- Snapshot tests with `insta` are not yet required for existing tests; new tests that produce stable
  structured output should prefer snapshots.
- Use `tracing` for operator-facing diagnostic logs. The audit report (terminal / JSON / Markdown /
  SARIF / HTML) is the user-facing artifact.
- French is allowed in user-facing strings, error messages, and documentation. Code identifiers stay
  English.
- Configuration lives in `.repolens.toml` at the repository root.

## Distribution

Primary distribution is via **GitHub Releases** on `systm-d/repolens`: `release.yml` builds
multi-platform binaries (Linux x86\_64, Linux aarch64, macOS Apple Silicon, Windows x86\_64) on
`v*.*.*` tags. Packaging metadata under `packaging/` covers Homebrew (macOS), AUR (Arch Linux),
Debian `.deb`, and Scoop (Windows). Docker image published to `ghcr.io/systm-d/repolens`. The
crates.io path (`cargo install repolens`) is also supported.

## Common commands

```bash
cargo build                              # build the binary
cargo run -- init                        # write a default .repolens.toml
cargo run -- plan --preset opensource    # smoke the CLI
cargo run -- report --format html        # generate an HTML report
cargo test --all                         # all tests (unit + integration)
cargo test --lib                         # unit tests only (faster iteration)
cargo clippy --all-targets -- -D warnings
cargo fmt
cargo deny check                         # license + advisory audit
```

## Reference

- `docs/superpowers/specs/2026-05-13-repolens-recentering-design.md` — v2.0.0 recentering decision.
- `docs/superpowers/specs/` — per-version design specs.
- `docs/superpowers/plans/` — implementation plans (Plans A, B, C).
- `../../system/guardians/` — sibling Rust project. Edition, MSRV, license, and CI workflow
  structure are deliberately aligned with guardians; when in doubt about engineering conventions,
  check there first.
