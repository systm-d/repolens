# Plan A — Scope Cuts (v2.0.0)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove output-format proliferation (PDF, CSV, TSV, NDJSON, JUnit XML) and the unshipped `integrations/` graveyard to refocus RepoLens on its core mission.

**Architecture:** Each formatter is removed in isolation: delete the module file, remove its module declaration and pub use in `src/cli/output/mod.rs`, remove its variant from the three output enums (`OutputFormat`, `ReportFormat`, `CompareFormat`) in `src/cli/commands/mod.rs`, remove its dispatch arms in `plan.rs` / `report.rs` / `compare.rs`, then drop its tests/benches and Cargo dependency. After each formatter, `cargo check && cargo test --lib` must pass. Cargo.toml version bump and CHANGELOG update happen at the end.

**Tech Stack:** Rust 2021 (will move to 2024 in Plan B), Cargo, clap derive.

**Spec:** [2026-05-13-repolens-recentering-design.md](../specs/2026-05-13-repolens-recentering-design.md)

---

## Pre-flight

### Task 0: Disable the pre-push hook for this work

The repo has an active pre-push hook that calls installed `repolens` v1.3.0 from PATH. With the changes below, the installed binary will diverge from source and produce false positives. Plan B removes git hooks entirely; for Plan A we temporarily disable them.

**Files:**
- Modify: `.git/hooks/pre-push` (rename to `.git/hooks/pre-push.disabled`)

- [ ] **Step 1: Disable pre-push hook**

```bash
mv .git/hooks/pre-push .git/hooks/pre-push.disabled
```

- [ ] **Step 2: Verify it's gone**

Run: `ls .git/hooks/ | grep -v sample`
Expected: `commit-msg`, `pre-commit`, `pre-commit.repolens-backup`, `pre-push.disabled` (no plain `pre-push`).

---

## Format removals (each is a self-contained commit)

### Task 1: Remove JUnit XML formatter

**Files:**
- Delete: `src/cli/output/junit.rs`
- Delete: `tests/integration/output_junit.rs`
- Modify: `src/cli/output/mod.rs`
- Modify: `src/cli/commands/mod.rs:248-256` (OutputFormat enum)
- Modify: `src/cli/commands/mod.rs:260-269` (ReportFormat enum)
- Modify: `src/cli/commands/mod.rs:309-317` (CompareFormat enum)
- Modify: `src/cli/commands/plan.rs:210` (dispatch arm)
- Modify: `src/cli/commands/report.rs` (find and remove dispatch arm for `ReportFormat::Junit`)
- Modify: `src/cli/commands/compare.rs` (find and remove dispatch arm for `CompareFormat::Junit`)
- Modify: `Cargo.toml` (remove `[[test]] name = "output_junit"` block at lines 163-165, remove `quick-xml` dependency)

- [ ] **Step 1: Delete the formatter and its tests**

```bash
rm src/cli/output/junit.rs tests/integration/output_junit.rs
```

- [ ] **Step 2: Remove module declaration and re-export**

In `src/cli/output/mod.rs`, delete these lines:

```rust
pub(crate) mod junit;
```

```rust
pub(crate) use junit::render_findings as render_junit_findings;
pub use junit::JunitReport;
```

- [ ] **Step 3: Remove `Junit` from the three output enums**

In `src/cli/commands/mod.rs`, delete the `Junit,` variant from `OutputFormat`, `ReportFormat`, and `CompareFormat`.

- [ ] **Step 4: Remove dispatch arms**

In `src/cli/commands/plan.rs`, delete the `OutputFormat::Junit => Box::new(JunitReport::new()),` arm.

Run: `grep -n "Junit\|junit" src/cli/commands/report.rs src/cli/commands/compare.rs`
Then delete the matching arms.

- [ ] **Step 5: Remove the `[[test]]` block from Cargo.toml**

In `Cargo.toml`, delete:

```toml
[[test]]
name = "output_junit"
path = "tests/integration/output_junit.rs"
```

- [ ] **Step 6: Remove `quick-xml` dependency from Cargo.toml**

`quick-xml` is only used by the JUnit XML writer. Delete:

```toml
# XML writer for JUnit output
quick-xml = "0.37"
```

- [ ] **Step 7: Verify compile + tests**

Run: `cargo check 2>&1 | tail -5`
Expected: `Finished` with no errors and no `unused crate` warnings.

Run: `cargo test --lib --quiet 2>&1 | tail -5`
Expected: all tests pass.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat!: remove JUnit XML output format

BREAKING CHANGE: --format junit no longer supported.
Removes quick-xml dependency."
```

---

### Task 2: Remove NDJSON formatter

**Files:**
- Delete: `src/cli/output/ndjson.rs`
- Delete: `tests/output_ndjson_test.rs`
- Modify: `src/cli/output/mod.rs`
- Modify: `src/cli/commands/mod.rs` (remove `Ndjson` from 3 enums)
- Modify: `src/cli/commands/plan.rs:209` (dispatch arm)
- Modify: `src/cli/commands/report.rs` (dispatch arm)
- Modify: `src/cli/commands/compare.rs` (dispatch arm)

- [ ] **Step 1: Delete files**

```bash
rm src/cli/output/ndjson.rs tests/output_ndjson_test.rs
```

- [ ] **Step 2: Remove module + re-export from `src/cli/output/mod.rs`**

Delete `pub mod ndjson;` and `pub use ndjson::NdjsonOutput;`.

- [ ] **Step 3: Remove `Ndjson` variants from the three enums in `src/cli/commands/mod.rs`**

- [ ] **Step 4: Remove dispatch arms in plan.rs / report.rs / compare.rs**

Run: `grep -rn "Ndjson\|NdjsonOutput\|ndjson" src/cli/`
Delete every match.

- [ ] **Step 5: Verify + commit**

```bash
cargo check && cargo test --lib --quiet 2>&1 | tail -3
git add -A
git commit -m "feat!: remove NDJSON output format

BREAKING CHANGE: --format ndjson no longer supported."
```

---

### Task 3: Remove TSV formatter

TSV is implemented in the same file as CSV (uses `csv` crate with tab delimiter). Removing TSV alone is just removing its enum variants + dispatch.

**Files:**
- Modify: `src/cli/commands/mod.rs` (remove `Tsv` from 3 enums)
- Modify: `src/cli/commands/plan.rs:186, 203-208` (remove `format_is_csv_like` Tsv arm and dispatch)
- Modify: `src/cli/commands/report.rs` (dispatch arm)
- Modify: `src/cli/commands/compare.rs` (dispatch arm)

- [ ] **Step 1: Remove `Tsv` variants from the 3 enums in `src/cli/commands/mod.rs`**

- [ ] **Step 2: Update `format_is_csv_like` matcher in `plan.rs:186`**

Find:

```rust
let format_is_csv_like = matches!(args.format, OutputFormat::Csv | OutputFormat::Tsv);
```

Change to:

```rust
let format_is_csv_like = matches!(args.format, OutputFormat::Csv);
```

- [ ] **Step 3: Remove `OutputFormat::Tsv` dispatch arm in plan.rs**

Find `OutputFormat::Tsv => Box::new(...)` and delete.

- [ ] **Step 4: Remove Tsv arms in report.rs / compare.rs**

Run: `grep -rn "Tsv\|TsvOutput\|::Tsv" src/`
Delete every match.

- [ ] **Step 5: Verify + commit**

```bash
cargo check && cargo test --lib --quiet 2>&1 | tail -3
git add -A
git commit -m "feat!: remove TSV output format

BREAKING CHANGE: --format tsv no longer supported."
```

---

### Task 4: Remove CSV formatter (and `csv` dependency)

**Files:**
- Delete: `src/cli/output/csv.rs`
- Delete: `tests/output_csv_test.rs`
- Modify: `src/cli/output/mod.rs`
- Modify: `src/cli/commands/mod.rs` (remove `Csv` from 3 enums, remove `csv_delimiter`, `csv_bom`, `csv_keep_newlines` args on CompareArgs lines 294-304)
- Modify: `src/cli/commands/plan.rs:186, 194-208` (remove `format_is_csv_like` entirely, remove dispatch)
- Modify: `src/cli/commands/report.rs` (dispatch arm)
- Modify: `src/cli/commands/compare.rs` (dispatch arm + csv-specific arg handling)
- Modify: `Cargo.toml` (remove `csv = "1"` line)

- [ ] **Step 1: Delete files**

```bash
rm src/cli/output/csv.rs tests/output_csv_test.rs
```

- [ ] **Step 2: Remove module + re-export from `src/cli/output/mod.rs`**

Delete `pub mod csv;` and `pub use csv::CsvOutput;`.

- [ ] **Step 3: Remove `Csv` variants from 3 enums in `src/cli/commands/mod.rs`**

- [ ] **Step 4: Remove CSV-specific args from `CompareArgs`**

Delete `csv_delimiter`, `csv_bom`, `csv_keep_newlines` fields (lines 294-304).

- [ ] **Step 5: Simplify plan.rs**

Delete `let format_is_csv_like = …` line entirely (no longer needed if no consumers). Delete CSV dispatch arm. Audit any other references.

Run: `grep -n "format_is_csv_like\|csv_delimiter\|csv_bom\|csv_keep_newlines" src/cli/commands/plan.rs`
Delete all consumers.

- [ ] **Step 6: Remove CSV arms in report.rs / compare.rs**

Run: `grep -rn "Csv\|CsvOutput\|::Csv\|csv_" src/cli/commands/`
Delete every match. Also check `src/compare/mod.rs` for CSV-specific code.

- [ ] **Step 7: Remove `csv` dependency from Cargo.toml**

Delete:

```toml
# CSV / TSV output
csv = "1"
```

- [ ] **Step 8: Verify + commit**

```bash
cargo check 2>&1 | tail -5
cargo test --lib --quiet 2>&1 | tail -3
cargo build 2>&1 | grep -i "unused\|warning" | head -5
git add -A
git commit -m "feat!: remove CSV output format

BREAKING CHANGE: --format csv no longer supported.
Removes csv dependency."
```

---

### Task 5: Remove PDF formatter (and `printpdf` + `image` dependencies)

**Files:**
- Delete: `src/cli/output/pdf.rs` (and any sibling `pdf_*.rs` files)
- Delete: `tests/pdf_branding_test.rs`
- Delete: `benches/pdf_benchmark.rs`
- Modify: `src/cli/output/mod.rs`
- Modify: `src/cli/commands/mod.rs` (remove `Pdf` from `ReportFormat`)
- Modify: `src/cli/commands/report.rs` (dispatch arm + any pdf-specific args like branding)
- Modify: `Cargo.toml` (remove `printpdf`, `image`, the `[[bench]] name = "pdf_benchmark"` block)
- Modify: `Cargo.toml` (remove `lopdf` from dev-dependencies since only used for PDF inspection)

- [ ] **Step 1: Inventory PDF files**

Run: `ls src/cli/output/pdf* && find . -name "*pdf*" -path "*/src/*"`

If multiple `pdf_*.rs` exist (branding submodules), list them. Delete all.

- [ ] **Step 2: Delete files**

```bash
rm src/cli/output/pdf.rs tests/pdf_branding_test.rs benches/pdf_benchmark.rs
```

If there are pdf submodules, delete them too:

```bash
rm src/cli/output/pdf_*.rs 2>/dev/null
```

- [ ] **Step 3: Remove module + re-export from `src/cli/output/mod.rs`**

Delete `mod pdf;` and `pub use pdf::PdfReport;`.

- [ ] **Step 4: Remove `Pdf` from `ReportFormat` enum**

- [ ] **Step 5: Remove PDF dispatch + branding args in report.rs**

Run: `grep -rn "PdfReport\|::Pdf\|pdf_" src/cli/commands/`
Delete every match. Includes any `--brand-logo`, `--brand-color` CLI args if they exist.

- [ ] **Step 6: Remove `printpdf`, `image`, `lopdf` from Cargo.toml**

Delete:

```toml
# PDF report generation (pure Rust, no system dependency)
printpdf = { version = "0.7", default-features = false, features = ["embedded_images"] }
image = { version = "0.24", default-features = false, features = ["png", "jpeg"] }
```

And in dev-dependencies:

```toml
# PDF inspection in integration tests
lopdf = "0.32"
```

- [ ] **Step 7: Remove `[[bench]] name = "pdf_benchmark"` block from Cargo.toml**

Delete:

```toml
[[bench]]
name = "pdf_benchmark"
harness = false
```

- [ ] **Step 8: Verify + commit**

```bash
cargo check 2>&1 | tail -5
cargo test --lib --quiet 2>&1 | tail -3
git add -A
git commit -m "feat!: remove PDF output format and branding

BREAKING CHANGE: --format pdf no longer supported, --brand-* flags removed.
Removes printpdf, image, lopdf dependencies. ~1351 LOC + 3 deps removed."
```

---

## Auxiliary cleanups

### Task 6: Remove `integrations/` directory

The directory contains template YAML for Azure DevOps, CircleCI, GitLab CI, Jenkins, and a duplicated GitHub Actions template — none backed by Rust code. Per the spec, this is removed; reintroducing multi-provider needs its own design doc.

**Files:**
- Delete: `integrations/` (entire directory)
- Modify: `README.md` (remove any references to the integrations dir)

- [ ] **Step 1: Delete the directory**

```bash
rm -rf integrations/
```

- [ ] **Step 2: Find and remove README references**

Run: `grep -n "integrations/" README.md docs/ wiki/`
Delete or rewrite any line that points to the deleted directory.

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "chore!: remove unshipped integrations/ templates

The integrations/ directory contained template YAML for Azure DevOps,
CircleCI, GitLab CI, Jenkins, and GitHub Actions — none backed by Rust
provider implementations. Multi-provider support, if reintroduced, will
get its own design doc."
```

---

### Task 7: Remove stale `examples/github-action/`

These examples assume a published `systm-d/repolens@main` GitHub Action exists. There is no action repo yet. The example files would mislead users.

**Files:**
- Delete: `examples/github-action/` (basic.yml, pr-comment.yml, sarif-upload.yml)

- [ ] **Step 1: Delete the directory**

```bash
rm -rf examples/github-action/
```

- [ ] **Step 2: Check if `examples/` is now empty**

Run: `ls examples/`

If empty: `rmdir examples/`. Otherwise leave it.

- [ ] **Step 3: Find and remove README references**

Run: `grep -n "examples/github-action" README.md docs/`
Delete matching lines.

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "chore: remove examples referencing nonexistent GitHub Action"
```

---

## Finalization

### Task 8: Bump Cargo.toml to v2.0.0

**Files:**
- Modify: `Cargo.toml:3`

- [ ] **Step 1: Update version**

In `Cargo.toml`, change `version = "1.4.0"` to `version = "2.0.0"`.

- [ ] **Step 2: Run cargo check to update Cargo.lock**

```bash
cargo check 2>&1 | tail -3
```

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: bump version to 2.0.0"
```

---

### Task 9: Update CHANGELOG.md

**Files:**
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Add v2.0.0 entry above the v1.4.0 entry**

Open `CHANGELOG.md`. Below the `## [Unreleased]` line and above `## [1.4.0]`, insert:

```markdown
## [2.0.0] - 2026-05-13

### Removed (BREAKING CHANGES)

- **Output format**: PDF (with branding), CSV, TSV, NDJSON, JUnit XML have been removed.
  Use JSON, Markdown, HTML, or SARIF instead.
- **CLI args**: `--csv-delimiter`, `--csv-bom`, `--csv-keep-newlines`, PDF branding flags
  (`--brand-logo`, `--brand-color` if they existed) are gone.
- **`integrations/` directory** — template YAML for Azure DevOps, CircleCI, GitLab CI,
  Jenkins, and GitHub Actions has been removed. They were never backed by Rust
  provider implementations.
- **`examples/github-action/`** — referenced a nonexistent published GitHub Action.

### Changed

- Refocused on the core mission: audit GitHub repositories for best practices,
  security, and compliance. See `docs/superpowers/specs/2026-05-13-repolens-recentering-design.md`
  for the full rationale.
- Dependencies removed: `printpdf`, `image`, `csv`, `quick-xml`, `lopdf` (dev).
  Compile times reduced accordingly.

### Migration

If you used a removed format, your options are:
- **CSV / TSV / NDJSON** → use `--format json` and post-process with `jq` or your tool of choice.
- **JUnit XML** → use `--format sarif` (broader CI/CD support and richer schema).
- **PDF with branding** → use `--format html` and print to PDF from your browser, or pipe to `wkhtmltopdf` / `weasyprint`.
```

- [ ] **Step 2: Commit**

```bash
git add CHANGELOG.md
git commit -m "docs: add v2.0.0 entry to CHANGELOG"
```

---

### Task 10: Update README.md to drop removed formats

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Find references to removed formats**

Run: `grep -n "pdf\|csv\|tsv\|ndjson\|junit\|JUnit\|PDF\|CSV\|TSV\|NDJSON" README.md`

- [ ] **Step 2: Audit each match and either delete or rewrite**

Likely areas: the features bullet list, the "Output formats" section, examples under each command, the "CI/CD integration" mentions.

Replace any "10 output formats" / "9 formats" / similar claims with **"5 output formats: terminal, JSON, Markdown, SARIF, HTML"**.

- [ ] **Step 3: Verify no orphaned mentions**

Run: `grep -in "pdf\|csv\|tsv\|ndjson\|junit" README.md`
Expected: no matches.

- [ ] **Step 4: Commit**

```bash
git add README.md
git commit -m "docs: drop removed formats from README"
```

---

### Task 11: Final verification

- [ ] **Step 1: Clean build from scratch**

```bash
cargo clean
cargo check 2>&1 | tail -5
cargo build --release 2>&1 | tail -5
```

Expected: zero errors, zero warnings.

- [ ] **Step 2: Format + clippy**

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings 2>&1 | tail -10
```

Expected: clean.

- [ ] **Step 3: Full test suite**

```bash
cargo test --all 2>&1 | tail -10
```

Expected: all tests pass. Note the new total (was 1100, expect ~950 after format-test removals).

- [ ] **Step 4: Dry-run publish**

```bash
cargo publish --dry-run --allow-dirty 2>&1 | tail -10
```

Expected: `Packaged N files`, `Verifying repolens v2.0.0`, no errors. (Do NOT actually publish here — final publish is its own approval gate.)

- [ ] **Step 5: Sanity-check CLI**

```bash
./target/release/repolens --help | grep -i "format\|pdf\|csv\|tsv\|ndjson\|junit"
./target/release/repolens plan --help
./target/release/repolens report --help
./target/release/repolens compare --help
```

Expected: no references to the removed formats. Format options should only list `terminal`, `json`, `sarif`, `markdown`, `html` (per command).

- [ ] **Step 6: Push and announce**

```bash
git push origin main 2>&1 | tail -5
```

Then ask the user before:
- Tagging `v2.0.0`
- Publishing to crates.io
- Building Docker image
- Running Plan B (practice alignment) and Plan C (docs rewrite)

---

## Out of scope (handled by Plans B and C)

- Edition / MSRV / license bumps — Plan B
- CI workflow consolidation — Plan B
- CLAUDE.md rewrite — Plan C
- Wiki → docs migration — Plan C
- `wiremock` / `insta` adoption — Plan B
- Git hook permanent removal — Plan B (we only disabled in Task 0)
- Tagging and publishing v2.0.0 to crates.io — separate user-gated step at the end of Plan C
