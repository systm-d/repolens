# CSV / TSV output

The `csv` and `tsv` formats emit one row per finding, suitable for reporting
pipelines, spreadsheet imports, and quick `cut`/`awk` analysis.

Available on `repolens plan`, `repolens report`, and `repolens compare`.

## Header

The header is fixed and identical for CSV and TSV:

```
rule_id,category,severity,file,line,column,message,description,remediation,project
```

| Column        | Source                                                        |
| ------------- | ------------------------------------------------------------- |
| `rule_id`     | `Finding.rule_id`                                             |
| `category`    | `Finding.category`                                            |
| `severity`    | `Finding.severity` (lowercased: `critical` / `warning` / `info`) |
| `file`        | `Finding.location` split on the last `:` — empty if absent    |
| `line`        | `Finding.location` after the last `:` — empty if absent       |
| `column`      | Always empty (reserved for forward compatibility)             |
| `message`     | `Finding.message`                                             |
| `description` | `Finding.description` — empty if absent                       |
| `remediation` | `Finding.remediation` — empty if absent                       |
| `project`     | `AuditResults.repository_name`                                |

For `repolens compare`, the columns instead are
`change,rule_id,category,severity,file,line,column,message,description,remediation`
(no `project`; `change` is `added` or `resolved`).

## CLI flags

| Flag                  | Default | Effect                                                                           |
| --------------------- | ------- | -------------------------------------------------------------------------------- |
| `--format csv`        | —       | Comma-separated, RFC 4180 quoting                                                |
| `--format tsv`        | —       | Tab-separated; tabs in cells → 4 spaces, newlines → 1 space                      |
| `--csv-delimiter`     | `,`     | Override the CSV delimiter (`,` `;` `|` …). Ignored in TSV mode.                 |
| `--csv-bom`           | off     | Prepend a UTF-8 BOM (`EF BB BF`) so Excel autodetects UTF-8. Ignored in TSV mode (warns). |
| `--csv-keep-newlines` | off     | Keep newlines inside CSV cells (the cell will be quoted). Ignored in TSV mode.   |

If any `--csv-*` flag is passed with `--format` other than `csv` or `tsv`, a
`[WARN]` is logged to stderr and the flag is ignored.

## Quoting & escaping

* **CSV (RFC 4180)**: cells containing `,` (or the chosen delimiter), `"`, `\r`,
  or `\n` are wrapped in double quotes; embedded `"` is escaped as `""`. Default
  behaviour replaces newlines with a single space; use `--csv-keep-newlines` to
  preserve them inside a quoted cell.
* **TSV**: there is no quoting — instead, embedded tabs are replaced by 4
  spaces and embedded newlines by 1 space, so each finding stays on one line.

## Worked examples

### Filter critical findings with `awk`

```bash
repolens report --format csv -o findings.csv
awk -F',' 'NR==1 || $3 == "critical"' findings.csv
```

### Project the rule and message columns with `cut`

```bash
repolens report --format tsv -o findings.tsv
cut -f1,7 findings.tsv | head
```

### Convert to JSON with `jq` (after a quick `csv2json`)

```bash
# Using miller (mlr) — handy for ad-hoc CSV/TSV → JSON conversions.
mlr --c2j cat findings.csv | jq '.[] | select(.severity=="critical")'
```

### Group findings by category with `awk`

```bash
awk -F',' 'NR>1 {n[$2]++} END {for (k in n) print k, n[k]}' findings.csv
```

## Importing into spreadsheets

### Microsoft Excel

Excel’s default CSV import uses your locale’s separator. To make it
unambiguous, generate a UTF-8 BOM-prefixed file with the comma delimiter:

```bash
repolens report --format csv --csv-bom -o findings.csv
```

Open the file with **File → Open** (not paste); Excel will pick UTF-8 thanks
to the BOM. If your locale uses `;` as a separator, regenerate with
`--csv-delimiter ';'`.

### Google Sheets

Google Sheets imports UTF-8 cleanly without a BOM:

```bash
repolens report --format csv -o findings.csv
```

Then **File → Import → Upload** and choose “Replace current sheet”.
Google Sheets autodetects `,` and `\t` separators, so TSV works equally well.
