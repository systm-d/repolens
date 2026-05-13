# RepoLens JSON Schemas

This directory contains JSON Schema definitions for RepoLens output formats.

## audit-report.schema.json

**JSON Schema draft-07** for the audit report JSON output.

### Usage

#### Include schema reference in output

```bash
repolens report --format json --schema
```

This adds a `$schema` field to the JSON output pointing to the schema URI.

#### Validate output against the schema

```bash
repolens report --format json --schema --validate
```

This validates the JSON output against the embedded schema before emitting it.

#### Export the schema

```bash
repolens schema --output schemas/audit-report.schema.json
```

#### Display the schema

```bash
repolens schema
```

### Schema Structure

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `$schema` | string | No | JSON Schema reference URI (included with `--schema`) |
| `repository_name` | string | Yes | Name of the audited repository |
| `preset` | string | Yes | Audit preset (opensource, enterprise, strict) |
| `findings` | array | Yes | List of audit findings |
| `metadata` | object | No | Report metadata |
| `summary` | object | No | Aggregated finding counts |

### Finding Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `rule_id` | string | Yes | Unique rule identifier (e.g., SEC001) |
| `category` | string | Yes | Finding category |
| `severity` | string | Yes | Severity: critical, warning, info |
| `message` | string | Yes | Short description of the finding |
| `location` | string/null | No | File location (e.g., src/config.rs:42) |
| `description` | string/null | No | Detailed description |
| `remediation` | string/null | No | Suggested fix |

### Metadata Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `version` | string | Yes | RepoLens version |
| `timestamp` | string | Yes | ISO 8601 timestamp |
| `schema_version` | string | Yes | Schema version (1.0.0) |

### Summary Object

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `total` | integer | Yes | Total number of findings |
| `by_severity` | object | Yes | Counts per severity level |
| `by_category` | object | Yes | Counts per category |

### Example Output

```json
{
  "$schema": "https://github.com/systm-d/repolens/schemas/audit-report.schema.json",
  "repository_name": "my-project",
  "preset": "opensource",
  "findings": [
    {
      "rule_id": "SEC001",
      "category": "secrets",
      "severity": "critical",
      "message": "Hardcoded API key detected",
      "location": "src/config.rs:42",
      "description": "A hardcoded secret was found in the source code",
      "remediation": "Move the secret to environment variables"
    }
  ],
  "metadata": {
    "version": "1.0.0",
    "timestamp": "2026-01-29T12:00:00Z",
    "schema_version": "1.0.0"
  },
  "summary": {
    "total": 1,
    "by_severity": {
      "critical": 1,
      "warning": 0,
      "info": 0
    },
    "by_category": {
      "secrets": 1
    }
  }
}
```

### Validation

The schema can be used with any JSON Schema draft-07 compatible validator. For example, using `ajv`:

```bash
npm install -g ajv-cli
ajv validate -s schemas/audit-report.schema.json -d report.json
```

Or programmatically in Rust using the `jsonschema` crate (which RepoLens uses internally).
