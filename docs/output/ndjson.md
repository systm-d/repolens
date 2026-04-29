# NDJSON output

NDJSON (Newline-Delimited JSON) emits **one JSON object per line**, terminated
by a single `\n` (LF). It’s the right format when you want to stream findings
into a log pipeline, a SIEM, or a `jq`-based filter chain — each line is
independently parseable, no parser needs to buffer the whole document.

Available on `repolens plan`, `repolens report`, and `repolens compare`.

## Shape of one line

```json
{"rule_id":"SEC001","category":"secrets","severity":"critical","file":"src/config.ts","line":42,"column":null,"message":"API key detected","description":"...","remediation":"...","project":"my-project"}
```

| Field         | Type            | Notes                                                                  |
| ------------- | --------------- | ---------------------------------------------------------------------- |
| `rule_id`     | string          | Unique rule identifier (e.g. `SEC001`, `DOC001`).                       |
| `category`    | string          | Rule category (e.g. `secrets`, `docs`, `security`).                    |
| `severity`    | string          | One of `critical`, `warning`, `info`.                                  |
| `file`        | string \| null  | File path parsed from `Finding.location` (`null` if no location).     |
| `line`        | integer \| null | Line number parsed from `Finding.location` (`null` if missing/unparseable). |
| `column`      | integer \| null | Always `null` — reserved for forward compatibility.                    |
| `message`     | string          | Short message.                                                         |
| `description` | string \| null  | Detailed description (`null` if not provided).                         |
| `remediation` | string \| null  | Suggested fix (`null` if not provided).                                |
| `project`     | string          | The scanned project name (`AuditResults.repository_name`).             |

For `repolens compare`, each line additionally carries
`"change":"added"` (regression) or `"change":"resolved"` (improvement),
and the `project` field is omitted (compare runs over two reports).

The schema is published as
[`schemas/finding.schema.json`](../../schemas/finding.schema.json) (JSON Schema
draft-07).

## Line separator

Lines are separated by a single LF (`\n`). There is **no** CRLF and **no**
BOM. The file ends with a trailing `\n` after the last line, so each entry is
self-delimited.

## Memory profile

The renderer streams each finding directly into a single output buffer; it
does **not** collect per-line strings into a `Vec` first. On a 10 000-finding
fixture, peak RSS stays well under 50 MB.

## Worked examples

### Stream and filter critical findings with `jq`

```bash
repolens report --format ndjson -o findings.ndjson
jq -c 'select(.severity == "critical")' findings.ndjson
```

`jq -c` keeps each result on one line, so the output is itself NDJSON.

### Pipe straight to a log shipper

```bash
repolens report --format ndjson | logger -t repolens
```

Or, for Vector / Fluent Bit:

```bash
repolens report --format ndjson | nc -U /run/vector.sock
```

### Count findings per category with `jq`

```bash
jq -r '.category' findings.ndjson | sort | uniq -c | sort -rn
```

### Pull just the file path with `awk` after stripping JSON

```bash
jq -r 'select(.file != null) | .file' findings.ndjson | sort -u
```

### Compare two reports and stream the regressions

```bash
repolens compare \
  --base-file before.json \
  --head-file after.json \
  --format ndjson \
  | jq -c 'select(.change == "added" and .severity == "critical")'
```

### Validate a line against the schema

```bash
# Using ajv-cli, or any JSON-Schema validator:
ajv validate -s schemas/finding.schema.json -d <(head -1 findings.ndjson)
```

## Why not JSON?

JSON requires the consumer to parse the whole document before it can act on a
single record. NDJSON keeps the per-record contract while letting you `jq -c`,
`grep`, `head`, `tail`, `awk` through the stream — exactly the ergonomics ops
and SRE pipelines rely on.
