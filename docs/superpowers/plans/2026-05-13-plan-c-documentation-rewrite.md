# Plan C — Documentation Rewrite (v2.0.0)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task.

**Goal:** Bring RepoLens documentation to the depth and structure of the sibling project `guardians`. Rewrite a stale CLAUDE.md, migrate the `wiki/` directory into `docs/`, and verify the README accurately reflects v2.0.0.

**Architecture:** Three tasks. T1 rewrites CLAUDE.md from scratch (the current 68-line file claims 6 rule categories when there are 16, and was written before Plans A/B). T2 collapses `wiki/` into `docs/` (the wiki was a separate manual; with the project recentered, a single user-facing docs directory is cleaner). T3 is final verification.

**Tech Stack:** Markdown only. No code changes.

**Spec:** [2026-05-13-repolens-recentering-design.md](../specs/2026-05-13-repolens-recentering-design.md)

**Prerequisites:** Plans A and B complete (commits c0f1c00 … 470159a).

---

## Task 1: Rewrite CLAUDE.md

The current `CLAUDE.md` (68 lines) is two versions stale: it claims 6 rule categories when 16 exist, references `apply` as a primary command without explaining the planner/apply split, and contains a section "Commit and PR Rules" that mixes load-bearing rules with style notes. Guardians' `CLAUDE.md` (141 lines) is a better template.

**Files:**
- Rewrite: `CLAUDE.md` (complete replacement, do NOT preserve sections that are stale)

- [ ] **Step 1: Read both files for reference**

```bash
cat CLAUDE.md
cat /home/kdelfour/Workspace/Professionel/Delfour.co/system/guardians/CLAUDE.md
```

- [ ] **Step 2: Inventory ground truth**

The rewrite must be factually correct. Gather data BEFORE writing:

```bash
# Cargo metadata
grep -E "^name|^version|^edition|^rust-version|^license" Cargo.toml

# CLI commands
grep -A1 "^pub struct.*Args\|^pub enum Command" src/cli/commands/mod.rs | head -40

# Rule categories
ls src/rules/categories/

# Output formats
ls src/cli/output/ | grep -v mod.rs

# Workflows
ls .github/workflows/

# Test counts
cargo test --lib --quiet 2>&1 | tail -3
```

- [ ] **Step 3: Write the new CLAUDE.md**

Use this exact structure (the section headings and order). For each section, write content that reflects the current state of v2.0.0:

```markdown
# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Repository status

[One paragraph: v2.0.0 — recentered on GitHub repo auditing. Single-provider (GitHub), 16 rule categories, 5 output formats. Brief mention of CI (ci.yml + release.yml) and packaging (Homebrew, AUR, Debian, Scoop, Docker). Cite the canonical spec at `docs/superpowers/specs/2026-05-13-repolens-recentering-design.md`.]

What's in the codebase right now:
- **CLI surface:** `repolens { init | plan | apply | report | compare | install-hooks | schema | completions | generate-man }`
- [List rule categories — there are 16: secrets, files, docs, security, workflows, quality, licenses, dependencies, docker, git, codeowners, custom, metadata, issues, history, repository-hygiene]
- [List output formats — 5: terminal, JSON, Markdown, SARIF, HTML]
- [Mention Cargo edition + MSRV — 2024 + 1.85]
- [Mention CI — 2 workflows: ci.yml (test matrix + coverage + security + package) and release.yml (build matrix + crates.io + docker)]

## Product in one line

[Sentence describing what RepoLens is — the differentiator being the plan/apply split where the audit produces a typed `ActionPlan` that an operator can review and selectively apply.]

## Architecture

```
src/
├── main.rs            — entry point
├── lib.rs             — public re-exports
├── cli/               — subcommand dispatch + output formatters
│   ├── commands/       — one file per top-level subcommand
│   └── output/         — terminal, json, markdown, sarif, html
├── config/            — config loading + preset selection
├── rules/             — rules engine
│   ├── engine.rs       — parallel rule execution
│   ├── categories/     — 16 categories
│   └── results.rs      — Finding + AuditResults types
├── actions/           — ActionPlan + executors
├── providers/         — GitHub (octocrab + gh CLI fallback)
├── scanner/           — filesystem + git tree iteration
├── compare/           — diff two audit reports
└── utils/             — prerequisites checks, etc.
```

## Module discipline

[Three short paragraphs:
1. `rules/` is pure logic — no IO. Findings are computed from scanner output.
2. `actions/` is pure planning — turns findings into a `Vec<Action>`. No execution.
3. `providers/` is the only place that talks to GitHub.
4. `cli/output/` is presentation only — renders an `AuditResults` to a string. No side effects.]

## Non-obvious constraints

[Bullet list of load-bearing rules — things that change what the product IS:
- **Plan/apply split is non-negotiable.** Every fix is computed first as an `Action`, then optionally applied. `repolens plan` and `repolens apply` are NOT the same command with a flag.
- **Presets are static at build time.** `.repolens.toml` overrides preset values; presets themselves are baked in. Don't accept "custom preset names" from config — that's a rule list, not a preset.
- **Provider is GitHub-only.** The `providers/` directory has exactly one trait impl. Multi-provider work is explicitly out of scope until a new design doc lands (see the recentering spec).
- **No Co-authored-by in commits or PRs.** Single primary author per commit. Mentioning contributors in the body without the `Co-authored-by:` trailer is allowed.
- **No `feat!:` or similar `!` Conventional Commit shorthand.** The commit-msg hook rejects it. Use `feat: ...` + `BREAKING CHANGE: ...` in the body.
- **GitHub authentication is dual-mode.** `GITHUB_TOKEN` env var is preferred (no `gh` CLI required); fallback is `gh` CLI. Both code paths must be tested.]

## Working conventions

[Bullet list:
- Tests live in `tests/` (integration) and inline `#[cfg(test)] mod tests` (unit). Per-rule tests are inside `src/rules/categories/<category>.rs`.
- `cargo test --all` runs everything. `cargo test --lib` skips integration; useful while iterating.
- Snapshot tests use `insta` when added (none required for existing tests; future tests should prefer snapshots for stable structured output).
- Use `tracing` for operator logs. The audit report is the user-facing artifact.
- French is allowed in user-facing strings (docs, error messages). Code identifiers stay English.]

## Distribution

[One paragraph: GitHub Releases as the primary distribution. Multi-platform binaries built by `release.yml` on `v*.*.*` tags. Packaging metadata in `packaging/` for Homebrew, AUR, Debian, Scoop. Docker image at `ghcr.io/systm-d/repolens`. `cargo install repolens` for the crates.io path.]

## Common commands

```bash
cargo build                              # build the binary
cargo run -- plan --preset opensource    # smoke the CLI
cargo run -- init                        # write a default .repolens.toml
cargo test --all                         # all tests
cargo test --lib                         # unit tests only (faster)
cargo clippy --all-targets -- -D warnings
cargo fmt
cargo deny check                         # license + advisory audit
```

## Reference

- `docs/superpowers/specs/2026-05-13-repolens-recentering-design.md` — v2.0.0 recentering decision.
- `docs/superpowers/specs/` and `docs/superpowers/plans/` — per-version design specs and implementation plans.
- `../../system/guardians/` — sibling Rust project. Engineering practices (edition, MSRV, license, workflow structure) are deliberately aligned with guardians.
```

Replace every bracketed placeholder with actual content derived from the inventory step. The result should be roughly 130-160 lines. No bracketed placeholders may remain.

- [ ] **Step 4: Verify factual accuracy**

```bash
# Each item in your CLAUDE.md must be true. Spot-check:
ls src/rules/categories/ | wc -l            # expect 16
ls src/cli/output/ | grep -v mod.rs | wc -l # expect 5
ls .github/workflows/ | wc -l               # expect 2
grep -E "^version|^edition|^rust-version" Cargo.toml
```

- [ ] **Step 5: Verify no stale references**

```bash
grep -inE "pdf|csv|tsv|ndjson|junit|brand|integrations/" CLAUDE.md
```

ZERO matches expected.

- [ ] **Step 6: Commit**

```bash
git add CLAUDE.md
git commit -m "docs: rewrite CLAUDE.md for v2.0.0

The old file (68 lines) claimed 6 rule categories when 16 exist,
referenced removed output formats, and mixed load-bearing rules
with style notes. The rewrite mirrors the depth and structure of
the sibling project guardians."
```

---

## Task 2: Migrate `wiki/` into `docs/`

The `wiki/` directory was a separate user-facing manual (13 markdown files in French covering installation, configuration, contribution, etc.). With v2.0.0 recentering, having two parallel doc trees (`docs/` AND `wiki/`) is friction. Collapse them.

**Files:**
- Move: `wiki/Architecture.md` → `docs/architecture.md`
- Move: `wiki/Bonnes-pratiques.md` → `docs/best-practices.md`
- Move: `wiki/Categories-de-regles.md` → `docs/rule-categories.md`
- Move: `wiki/Changelog-Automatique.md` → `docs/automatic-changelog.md`
- Move: `wiki/Configuration.md` → `docs/configuration.md`
- Move: `wiki/Contribution.md` → `docs/contributing.md`
- Move: `wiki/Custom-Rules.md` → `docs/custom-rules.md`
- Move: `wiki/Developpement.md` → `docs/development.md`
- Move: `wiki/Guide-d-utilisation.md` → `docs/usage.md`
- Move: `wiki/Installation.md` → already exists at `docs/installation.md` — MERGE if content differs, otherwise discard wiki version
- Move: `wiki/Presets.md` → `docs/presets.md`
- Drop: `wiki/Home.md` (wiki landing page; not needed in `docs/`)
- Drop: `wiki/README.md` (wiki meta file; not needed)

- [ ] **Step 1: Compare content of wiki/Installation.md with docs/installation.md**

```bash
diff wiki/Installation.md docs/installation.md | head -30
```

If they're substantially the same: discard `wiki/Installation.md`. If `wiki/Installation.md` has unique content: merge the unique sections into `docs/installation.md`.

- [ ] **Step 2: Move each wiki file with `git mv` + lowercase rename**

```bash
git mv wiki/Architecture.md docs/architecture.md
git mv wiki/Bonnes-pratiques.md docs/best-practices.md
git mv wiki/Categories-de-regles.md docs/rule-categories.md
git mv wiki/Changelog-Automatique.md docs/automatic-changelog.md
git mv wiki/Configuration.md docs/configuration.md
git mv wiki/Contribution.md docs/contributing.md
git mv wiki/Custom-Rules.md docs/custom-rules.md
git mv wiki/Developpement.md docs/development.md
git mv wiki/Guide-d-utilisation.md docs/usage.md
git mv wiki/Presets.md docs/presets.md
```

If `wiki/Installation.md` content is unique, merge it; otherwise: `git rm wiki/Installation.md`.

`git rm wiki/Home.md wiki/README.md`.

- [ ] **Step 3: Confirm the wiki directory is empty and remove it**

```bash
ls wiki/
[ -z "$(ls wiki/)" ] && rmdir wiki/
```

- [ ] **Step 4: Audit cross-links inside the migrated docs**

Many of the wiki files probably linked to each other with `[text](OtherWikiPage)` or `[[OtherPage]]` syntax. The new file names are lowercase-with-hyphens.

```bash
grep -rn "\[.*\](\(Architecture\|Bonnes-pratiques\|Categories-de-regles\|Changelog-Automatique\|Configuration\|Contribution\|Custom-Rules\|Developpement\|Guide-d-utilisation\|Installation\|Presets\|Home\)\.md)" docs/
grep -rn "\[\[" docs/ 2>&1 | head
```

For each match, rewrite the link to point at the new filename. E.g. `[Configuration](Configuration.md)` → `[Configuration](configuration.md)`. If the link points to a dropped file (Home.md / README.md), delete the entire link.

- [ ] **Step 5: Audit cross-links from outside docs/**

```bash
grep -rn "wiki/" README.md docs/ CLAUDE.md CHANGELOG.md 2>&1 | grep -v docs/superpowers
```

For each match, rewrite the link to point at the new `docs/` path or delete if no longer relevant.

- [ ] **Step 6: Audit for the `kdelfour/repolens.wiki.git` / GitHub wiki URL references**

```bash
grep -rn "\.wiki\.git\|github.com/.*/wiki" README.md docs/ CLAUDE.md
```

Update or delete each match.

- [ ] **Step 7: Translate the migrated docs' content if you want a uniform language**

This is optional and the user's call — current content is largely in French. The recommendation: leave as-is (French content is fine), but rewrite the FILE NAMES in English (per Step 2). If you want bilingual headings inside the files, do not do that here — out of scope.

- [ ] **Step 8: Verify no stale paths**

```bash
[ -d wiki/ ] && echo "FAIL: wiki/ still exists" || echo "wiki/ is gone"
grep -rn "wiki/" README.md docs/ CLAUDE.md 2>&1 | grep -v docs/superpowers
```

The second grep must return zero matches.

- [ ] **Step 9: Commit**

```bash
git add -A
git commit -m "docs: collapse wiki/ into docs/

Two parallel doc trees create friction. The wiki/ directory held a
separate French manual; v2.0.0 keeps a single docs/ tree with
lowercase-hyphenated file names. Cross-links updated.

Renamed files preserve full git history via git mv. Home.md and
the wiki meta README are dropped (not relevant in the merged tree)."
```

---

## Task 3: Final verification

- [ ] **Step 1: Confirm doc structure**

```bash
ls docs/
[ -d wiki/ ] && echo "FAIL: wiki/ exists" || echo "wiki/ removed"
ls docs/superpowers/specs/
ls docs/superpowers/plans/
```

Expected:
- `docs/` contains only `.md` files (user-facing docs) and the `superpowers/` subdir
- `wiki/` is gone
- `docs/superpowers/specs/` contains the recentering spec
- `docs/superpowers/plans/` contains Plans A, B, C

- [ ] **Step 2: Confirm CLAUDE.md is current**

```bash
wc -l CLAUDE.md                                            # expect 130-160 lines
grep -E "v2\.0\.0|16 rule|5 output" CLAUDE.md             # should match
grep -inE "pdf|csv|tsv|ndjson|junit|brand|integrations" CLAUDE.md  # must be ZERO
```

- [ ] **Step 3: Full build + test (sanity)**

```bash
cargo check 2>&1 | tail -3
cargo test --all 2>&1 | tail -5
cargo publish --dry-run --allow-dirty 2>&1 | tail -5
```

All must pass.

- [ ] **Step 4: Final git log**

```bash
git log --oneline c0f1c00..HEAD
```

You should see commits for Plans A, B, C cleanly grouped.

- [ ] **Step 5: Report status**

No commit needed for this task. After T3 reports DONE, the user-gated tagging + publication of v2.0.0 happens (separately).

---

## Out of scope

- **Translation of French content to English.** Wiki files are in French; staying French is fine. File names were renamed to English-style lowercase-with-hyphens; content stays as-is.
- **`schemas/README.md` rewrite.** Out of scope for this plan. Touch only if cross-link audit (T2 step 4) finds it broken.
- **`docs/QUALITY_GATES.md` / `docs/QUALITY_GATES_SUMMARY.md` review.** Existing docs from earlier work. Don't audit them in this plan unless they reference the wiki or removed formats — in which case fix.
- **Publishing v2.0.0 to crates.io.** Separate user-gated step after Plan C is complete.
- **Pushing to GitHub.** Same — separate gate.
