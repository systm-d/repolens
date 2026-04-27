# PDF Reports

`repolens report --format pdf` generates a printable, brandable audit report
suitable for formal audits and compliance reviews. The renderer is pure Rust
(`printpdf`); no system font, `wkhtmltopdf`, or other native binary is needed.

## Quickstart

```bash
repolens report --format pdf -o audit.pdf
repolens report --format pdf -o audit.pdf --branding ./branding.toml
repolens report --format pdf -o audit.pdf --detailed
```

Verify the result with `qpdf --check audit.pdf` (zero exit code on success).

## Report Structure

Generated PDFs contain, in order:

1. **Cover page** — logo (centered, ≤ 200×80 pt), repository name in
   `primary_color` (24 pt bold), optional subtitle in `secondary_color`
   (14 pt), generation date, RepoLens version, configuration hash, and a
   coloured bottom band.
2. **Table of contents** — section names plus the page numbers where they
   begin.
3. **Summary** — finding counts by severity (Critical / Warning / Info)
   colour-coded `#D73A49` / `#FB8500` / `#0366D6`, followed by the top-10
   critical findings.
4. **Per-category sections** — one heading per category plus a four-column
   table (path, line, severity, message). Findings overflow to additional
   pages automatically.
5. **Annexes** — the applied branding TOML, the rule list, RepoLens version
   and config hash. When the report has more than 5 000 findings, Info-level
   findings are aggregated by category in the annex (Critical and Warning
   remain in the body).

`Severity::Pass` is **not** part of the public severity enum, so the report
covers `Critical`, `Warning`, and `Info` only.

## Branding

Provide a TOML file via `--branding <path>`. All fields are optional; missing
or invalid values fall back to the documented defaults and emit a `WARN`.

```toml
[branding]
logo_path        = "assets/logo.png"     # PNG/JPG, ≤ 5 MB; ignored if missing
primary_color    = "#0052CC"             # hex 6 or 8 chars; default #0052CC
secondary_color  = "#172B4D"             # hex 6 or 8 chars; default #172B4D
text_color       = "#000000"             # hex 6 or 8 chars; default #000000
font_family      = "Helvetica"           # one of Helvetica, Times, Courier
footer_text      = "Confidential — Acme" # ≤ 200 chars; rendered every page
header_text      = "Acme Corp"           # ≤ 200 chars; empty = omitted
cover_subtitle   = "Q2 2026 Compliance"  # ≤ 100 chars
```

`--branding` is **only** consulted for `--format pdf`. Passing it with another
format prints a warning and is otherwise ignored.

### Color format

Colors must match the regex `^#[0-9A-Fa-f]{6}([0-9A-Fa-f]{2})?$` — i.e.
either `#RRGGBB` or `#RRGGBBAA`. Each 8-bit channel is divided by 255 and
rounded to three decimal places before becoming a PDF `rg`/`RG` operator
(e.g. `#0052CC → 0.000 0.322 0.800`). Anything outside the regex resets that
field to its default.

### Logo handling

The renderer accepts PNG and JPEG via the `image` crate. The image is
centered horizontally on the cover and clamped to a `200 × 80 pt` bounding
box, preserving aspect ratio. Logos larger than 5 MB on disk, missing files,
and decode failures all log a `WARN` and produce a PDF without the logo
rather than failing.

### Font fallback

`genpdf`-style projects need bundled TTF data, but `printpdf`'s built-in PDF
fonts are sufficient for an audit report. RepoLens accepts the three
standard families and falls back to **Helvetica** for anything else:

| Requested family       | Resolved family |
|------------------------|-----------------|
| `Helvetica` (default)  | Helvetica       |
| `Times`, `Times-Roman` | Times           |
| `Courier`              | Courier         |
| anything else          | Helvetica + WARN |

Custom external fonts (`Inter`, custom CJK families, …) are out of scope for
v1.

## Layout limits

The renderer applies several mitigations to avoid pathological layouts:

| Situation                    | Mitigation                                            |
|------------------------------|-------------------------------------------------------|
| Cell text > 80 chars         | Wraps after `/`, `_`, `-`, or `.`                     |
| Cell text > 250 chars        | Truncated to 247 characters + `…`; full text stays in annex |
| Category > 200 findings      | Overflow moves to a `Category (cont.)` page; large overflow goes to the annex |
| > 5 000 findings total       | Info severity is aggregated in the annex (count per category) |
| Glyph absent from font       | `printpdf` skips it (built-in fonts cover Windows-1252) |

## Output validation

```bash
# Structural validity
qpdf --check audit.pdf

# Text extraction (snapshot or grep-based assertions)
pdftotext audit.pdf - | head

# Listing internal objects (debugging)
qpdf --qdf --object-streams=disable audit.pdf - | less
```

The integration test suite (`tests/pdf_branding_test.rs`) demonstrates each
of these checks. Tests that depend on `qpdf` or `pdftotext` are skipped at
runtime when those binaries are not on `PATH` so the suite remains green
even on minimal CI runners.

## Performance

Generating a report for 1 000 findings completes in roughly **9–10 ms** in
release mode on a laptop, well below the 30-second `ubuntu-latest` budget.
The Criterion benchmark `pdf_generation` (`benches/pdf_benchmark.rs`)
samples 100 / 1 000 / 5 000 findings and is the canonical pre-merge check.

```bash
cargo bench --bench pdf_benchmark
```

## Out of scope (v1)

- Detached PAdES signatures (`--signed`) — planned for v2.
- Runtime fallback to Typst — planned for v2.
- Native CJK fonts and font subsetting — planned for v2.
