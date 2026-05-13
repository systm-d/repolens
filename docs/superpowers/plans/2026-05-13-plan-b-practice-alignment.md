# Plan B — Practice Alignment (v2.0.0)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax.

**Goal:** Align RepoLens engineering practices on the sibling project `guardians`: bump Rust edition + MSRV, adopt dual MIT/Apache-2.0 licensing, consolidate the 6 GitHub Actions workflows into 2, fix the workflow broken by Plan A's PDF removal.

**Architecture:** Five sequential tasks. T1 is cheap admin (license file). T2 is the highest-risk change (edition bump). T3 is a one-line fix. T4 is the largest task (workflow consolidation). T5 is verification. After Plan B, RepoLens looks like guardians on the engineering layer.

**Tech Stack:** Rust 2024, cargo (1.85+), GitHub Actions.

**Spec:** [2026-05-13-repolens-recentering-design.md](../specs/2026-05-13-repolens-recentering-design.md)

**Prerequisites:** Plan A complete. HEAD at the v2.0.0 CHANGELOG commit (`7d2e070` or later).

---

## Task 1: Dual MIT + Apache-2.0 license

Match guardians' license posture (the dual MIT/Apache-2.0 model is the de-facto Rust ecosystem standard).

**Files:**
- Create: `LICENSE-APACHE`
- Rename: `LICENSE` → `LICENSE-MIT` (if the project has a `LICENSE` file)
- Modify: `Cargo.toml` (line 8) — change `license = "MIT"` to `license = "MIT OR Apache-2.0"`
- Modify: `README.md` — update the license section

- [ ] **Step 1: Find current license file location**

```bash
ls LICENSE LICENSE-MIT LICENSE-APACHE 2>&1
```

If a file named `LICENSE` exists, rename it to `LICENSE-MIT`. If `LICENSE-MIT` already exists, keep it as-is.

```bash
[ -f LICENSE ] && git mv LICENSE LICENSE-MIT
```

- [ ] **Step 2: Add `LICENSE-APACHE`**

Create `LICENSE-APACHE` with the Apache License 2.0 text. The canonical text is at https://www.apache.org/licenses/LICENSE-2.0.txt — fetch it once and write it to the repo:

```bash
curl -fsSL https://www.apache.org/licenses/LICENSE-2.0.txt -o LICENSE-APACHE
wc -l LICENSE-APACHE  # expect 202 lines
```

- [ ] **Step 3: Update Cargo.toml**

Change:

```toml
license = "MIT"
```

To:

```toml
license = "MIT OR Apache-2.0"
```

- [ ] **Step 4: Update README license section**

Find the license section in `README.md` (`grep -n "## License\|## Licence\|MIT License" README.md`). Replace with:

```markdown
## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
```

- [ ] **Step 5: Verify**

```bash
cargo check 2>&1 | tail -3   # license metadata is validated by cargo
cargo publish --dry-run --allow-dirty 2>&1 | grep -i "license\|error" | head -10
```

Expected: no license warnings or errors.

- [ ] **Step 6: Commit**

```bash
git add LICENSE-APACHE LICENSE-MIT Cargo.toml Cargo.lock README.md
git commit -m "chore: adopt dual MIT/Apache-2.0 license

Aligns RepoLens on the de-facto Rust ecosystem licensing model
and the sibling project guardians."
```

---

## Task 2: Rust edition 2024 + MSRV 1.85

The biggest behavioral change in this plan. Edition 2024 introduces new keywords (`gen`), changes around `unsafe` blocks in extern fns, lifetime capture rules in `impl Trait`, and changes to closure scope. `cargo fix --edition` automates most migrations.

**Files:**
- Modify: `Cargo.toml` (lines 4-5) — `edition = "2024"`, `rust-version = "1.85"`
- Modify: source files (auto-edited by `cargo fix --edition`)

- [ ] **Step 1: Confirm a recent toolchain is installed**

```bash
rustc --version
```

Required: `rustc 1.85.0` or later. If older, install via `rustup`:

```bash
rustup install 1.85.0
rustup default 1.85.0
rustc --version
```

- [ ] **Step 2: Run `cargo fix --edition` against the current (2021) edition**

```bash
cargo fix --edition --allow-dirty --allow-staged 2>&1 | tail -20
```

This will modify source files in place to be edition-2024-compatible while still on the 2021 edition.

- [ ] **Step 3: Bump edition and MSRV in `Cargo.toml`**

Change:

```toml
edition = "2021"
rust-version = "1.74"
```

To:

```toml
edition = "2024"
rust-version = "1.85"
```

- [ ] **Step 4: Run a clean build**

```bash
cargo clean
cargo check 2>&1 | tail -15
```

If there are errors, read them carefully — edition-2024 introduces breaking changes around:
- `let` chains in conditions
- `gen` is now a reserved keyword (rename any `gen` identifiers)
- `unsafe` extern fn calls must be `unsafe`
- Some closure capture rules tightened

Fix each error individually.

- [ ] **Step 5: Run fmt + clippy + tests**

```bash
cargo fmt
cargo fmt --check    # confirm fmt is idempotent
cargo clippy --all-targets -- -D warnings 2>&1 | tail -10
cargo test --all 2>&1 | tail -5
```

Edition 2024 may introduce new clippy lints (e.g., `unused_lifetimes`); address each one.

- [ ] **Step 6: Verify `cargo publish --dry-run` still works**

```bash
cargo publish --dry-run --allow-dirty 2>&1 | tail -5
```

- [ ] **Step 7: Commit**

```bash
git add -A
git commit -m "chore: bump to Rust 2024 edition and MSRV 1.85

Aligns RepoLens on the modern Rust ecosystem baseline used by
guardians and removes the 2-year-old MSRV (was 1.74). Source files
auto-migrated by cargo fix --edition."
```

---

## Task 3: Remove broken `pdf-benchmark` CI job

Plan A removed the PDF formatter. The `.github/workflows/ci.yml` still has a job named `pdf-benchmark` that runs `cargo bench --bench pdf_benchmark` against a benchmark file that no longer exists. The CI will fail on this job.

**Files:**
- Modify: `.github/workflows/ci.yml` — delete the `pdf-benchmark:` job entirely (lines 56-84 in the pre-Plan-A version; line numbers may have shifted)

- [ ] **Step 1: Find the broken job**

```bash
grep -n "pdf-benchmark\|pdf_benchmark" .github/workflows/ci.yml
```

- [ ] **Step 2: Delete the entire `pdf-benchmark:` job block from `ci.yml`**

The job spans from the `pdf-benchmark:` line down to (but not including) the next sibling job (`package:` based on Plan A's pre-state). Delete all those lines.

- [ ] **Step 3: Verify YAML is still valid**

```bash
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/ci.yml'))"
```

Expected: no output (no error).

- [ ] **Step 4: Confirm no more pdf references in any workflow**

```bash
grep -rn "pdf\|PDF\|brand" .github/workflows/
```

Expected: zero matches.

- [ ] **Step 5: Commit**

```bash
git add .github/workflows/ci.yml
git commit -m "ci: remove pdf-benchmark job

The PDF formatter was removed in Plan A; the CI job pointing at
benches/pdf_benchmark.rs would now fail."
```

---

## Task 4: Consolidate 6 workflows → 2

Current state: 6 workflows (`ci.yml`, `code-quality.yml`, `create-release.yml`, `docker.yml`, `nightly.yml`, `release.yml`) totaling 1865 lines with significant overlap (security audit lives in both `ci.yml` and `code-quality.yml`).

Target state (mirroring guardians):
- **`ci.yml`** — every-PR validation (check, fmt, clippy strict, test, coverage, security audit + cargo deny, package validation). Triggered on PR/push.
- **`release.yml`** — tag-driven release pipeline (build matrix, GitHub release, crates.io publish, Docker image push). Triggered on `v*.*.*` tags.

**Files:**
- Modify: `.github/workflows/ci.yml` (merge useful jobs from `code-quality.yml` and `nightly.yml`)
- Modify: `.github/workflows/release.yml` (merge useful jobs from `docker.yml` and `create-release.yml`)
- Delete: `.github/workflows/code-quality.yml`
- Delete: `.github/workflows/create-release.yml`
- Delete: `.github/workflows/docker.yml`
- Delete: `.github/workflows/nightly.yml`

- [ ] **Step 1: Read every workflow file to know what's in it**

```bash
wc -l .github/workflows/*.yml
for f in .github/workflows/*.yml; do echo "=== $f ==="; grep -E "^  [a-z][a-z_-]*:" "$f"; done
```

- [ ] **Step 2: Decide what each merged file MUST contain**

**Target `ci.yml` jobs:**
- `check` (cargo check)
- `fmt` (cargo fmt --check)
- `clippy` (cargo clippy --all-targets -- -D warnings)
- `test` (cargo test --all on ubuntu-22.04 AND ubuntu-24.04 matrix — mirrors guardians)
- `coverage` (cargo tarpaulin, fail-under 90 — keep current threshold)
- `security` (cargo audit + cargo deny check)
- `package` (cargo package + metadata validation)

**Target `release.yml` jobs:**
- `build` (multi-platform binary build matrix)
- `changelog` (auto-generate release notes)
- `release` (GitHub release creation)
- `publish-crate` (cargo publish)
- `docker` (build + push docker image to ghcr.io/systm-d/repolens)

Jobs to drop:
- `pdf-benchmark` (already removed in Task 3)
- `outdated-dependencies` (informational only; cargo audit covers actual vulnerabilities)
- `semgrep-analysis` (third-party tool, adds noise; keep only if user wants it)
- `code-metrics` (informational only)
- `quality-report` (aggregator; not needed)
- `nightly.yml`'s `skip-notification` (cosmetic)
- `create-release.yml`'s `prepare-release` (manual tag + release.yml does it)
- `announce-discussion` from current release.yml (optional UX nicety; drop unless user objects)

- [ ] **Step 3: Write the new `ci.yml`**

Start from the current `ci.yml`, remove the broken pdf-benchmark job (already done in Task 3), then merge in:
- The security audit job from `code-quality.yml` (cargo deny) — verify it's not a duplicate of the existing `security` job
- The test matrix from guardians: `os: [ubuntu-22.04, ubuntu-24.04]` (replace `ubuntu-latest`)

Use this skeleton:

```yaml
name: CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  test:
    name: test (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-22.04, ubuntu-24.04]
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2
      - name: cargo fmt --check
        run: cargo fmt --all -- --check
      - name: cargo clippy
        run: cargo clippy --all-targets -- -D warnings
      - name: cargo test
        run: cargo test --all

  coverage:
    name: coverage
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo install cargo-tarpaulin --locked
      - name: tarpaulin
        run: cargo tarpaulin --all-features --out Xml --output-dir coverage --fail-under 90 -- --skip e2e_
      - uses: codecov/codecov-action@v4
        with:
          files: ./coverage/cobertura.xml
          fail_ci_if_error: false

  security:
    name: security
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo install cargo-audit cargo-deny --locked
      - run: cargo audit
      - run: cargo deny check advisories licenses

  package:
    name: package
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo package --list
      - run: cargo publish --dry-run
```

Note: the heavy tarpaulin `--exclude-files` list from the current `ci.yml` is preserved as-is (the implementer should copy it over verbatim into the new `coverage` step).

- [ ] **Step 4: Write the new `release.yml`**

Take the current `release.yml` as the base. It already has build/changelog/release/publish-crate. Merge in the Docker build job from `docker.yml`. Drop `announce-discussion`.

The Docker job should:
- Run after the binary build succeeds
- Build the multi-arch image (linux/amd64, linux/arm64)
- Push to `ghcr.io/systm-d/repolens` (note: this matches the URLs migrated in this session)
- Tag the image as both `latest` and the version (e.g. `2.0.0`)

- [ ] **Step 5: Delete the four redundant workflow files**

```bash
git rm .github/workflows/code-quality.yml
git rm .github/workflows/create-release.yml
git rm .github/workflows/docker.yml
git rm .github/workflows/nightly.yml
```

- [ ] **Step 6: Validate YAML**

```bash
for f in .github/workflows/*.yml; do
  python3 -c "import yaml; yaml.safe_load(open('$f'))" && echo "$f OK"
done
```

All files must report `OK`.

- [ ] **Step 7: Lint workflows with actionlint (optional but recommended)**

```bash
command -v actionlint && actionlint .github/workflows/*.yml || echo "actionlint not installed, skipping"
```

If `actionlint` is installed, fix every issue it reports.

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "ci: consolidate 6 workflows into 2 (ci + release)

Aligns workflow organization on the sibling project guardians.
- ci.yml: every-PR validation (test matrix, coverage, security, package)
- release.yml: tag-driven release (build, changelog, GitHub release, crates.io, docker)

Removed: code-quality.yml, create-release.yml, docker.yml, nightly.yml.
Their useful jobs are merged into ci.yml and release.yml. Dropped:
outdated-dependencies, semgrep-analysis, code-metrics, quality-report
(informational-only jobs that added noise without blocking value)."
```

---

## Task 5: Final verification

- [ ] **Step 1: Clean build under the new edition + MSRV**

```bash
cargo clean
cargo check 2>&1 | tail -5
cargo fmt --check
cargo clippy --all-targets -- -D warnings 2>&1 | tail -10
cargo test --all 2>&1 | tail -8
```

Expected: all clean.

- [ ] **Step 2: Verify Cargo.toml metadata is publish-ready**

```bash
cargo publish --dry-run --allow-dirty 2>&1 | tail -10
```

Expected: `Packaged N files`, `Verifying repolens v2.0.0`, no errors.

- [ ] **Step 3: Verify license files exist**

```bash
ls -la LICENSE-MIT LICENSE-APACHE
head -1 LICENSE-MIT
head -1 LICENSE-APACHE
```

Expected: both files exist; LICENSE-APACHE starts with `Apache License`.

- [ ] **Step 4: Verify exactly 2 workflows remain**

```bash
ls .github/workflows/
```

Expected: only `ci.yml` and `release.yml`.

- [ ] **Step 5: Run actionlint if available**

```bash
command -v actionlint && actionlint .github/workflows/*.yml || echo "actionlint not installed"
```

- [ ] **Step 6: Report status**

No commit needed for this task (just verification). Move on to Plan C.

---

## Out of scope

- **`wiremock` / `insta` adoption** — deferred. Adopting these for existing tests would touch hundreds of files; the value-vs-risk ratio is poor for a recentering release. Future tests should use these patterns, but Plan B does NOT refactor existing tests.
- **Removing git hooks** — `.git/hooks/` is local-only (not tracked). The user can delete them manually if desired. The `repolens install-hooks` command stays (it's a user-facing feature).
- **CLAUDE.md rewrite, wiki migration** — Plan C.
- **Publishing v2.0.0** — separate user-gated step at the end of Plan C.
