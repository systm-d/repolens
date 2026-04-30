# JUnit XML Output

RepoLens can emit audit findings as a JUnit XML test report so CI systems
that natively understand JUnit (GitLab CI, Jenkins, Bitbucket Pipelines,
CircleCI, GitHub Actions via reporters, …) can surface findings as
"failing tests" alongside your regular test suite.

## Enabling JUnit Output

The JUnit format is available on the `plan`, `report` and `compare`
commands via `--format junit`:

```bash
# Plan command
repolens plan --format junit -o repolens-plan.xml

# Report command
repolens report --format junit -o repolens-report.xml

# Compare command (only regressions are reported)
repolens compare \
  --base-file base.json \
  --head-file head.json \
  --format junit \
  -o repolens-compare.xml
```

When `report` is invoked without `-o`, it writes to
`repolens-report.xml`.

## Mapping

Each finding is converted into a `<testcase>`. The `<testcase>` is grouped
under a `<testsuite>` whose name is the finding's category (e.g. `secrets`,
`docs`, `security`, `workflows`, `quality`, …).

The severity of a finding controls which inner element is emitted, and
which counter is incremented at the `<testsuites>` level:

| Severity   | Inner element                                  | `<testsuites>` counter |
|------------|------------------------------------------------|------------------------|
| `Critical` | `<error type="critical" message="…">`          | `errors`               |
| `Warning`  | `<failure type="warning" message="…">`         | `failures`             |
| `Info`     | `<system-out>` (no `<failure>` / `<error>`)    | —                      |

The `name` attribute of each `<testcase>` is built from the rule id plus
its location (when known):

- `rule_id [path/to/file:line]` if a location is available
- `rule_id` otherwise

The `time` attribute is always `0.000` because the audit is a static
analysis — there is no per-rule duration.

The `skipped` attribute is always `0` because RepoLens has no concept of
"skipped" rules in its current data model.

### Compare command

`repolens compare --format junit` emits **only regressions** (findings in
the head report that are absent from the base report). Resolved findings
are silent — they have no failing-test analogue. Unchanged findings are
also omitted.

### Empty audits

When an audit produces no findings, the report contains a single empty
`<testsuites>` element with `tests="0" failures="0" errors="0"
skipped="0"`. CI systems should record this as a successful run with zero
tests.

## XML Safety

- The output starts with the standard declaration
  `<?xml version="1.0" encoding="UTF-8"?>`.
- All textual content (messages, locations, descriptions, remediation
  hints) is escaped via `quick-xml`'s `BytesText`/`BytesAttr` writers, so
  values containing `<`, `>`, `&`, `"`, `'` are rendered as standard XML
  entity references.
- No DOCTYPE is emitted and no external entities are referenced.

## CI Snippets

### GitLab CI

GitLab CI ingests JUnit reports through `artifacts:reports:junit`.
Failed tests appear in the merge request widget and on the pipeline page.

```yaml
repolens-audit:
  image: rust:latest
  script:
    - cargo install repolens
    - repolens report --format junit -o repolens-report.xml
  artifacts:
    when: always
    reports:
      junit: repolens-report.xml
    paths:
      - repolens-report.xml
```

For comparison runs (e.g. base vs. head on a merge request):

```yaml
repolens-compare:
  image: rust:latest
  script:
    - repolens compare \
        --base-file base.json \
        --head-file head.json \
        --format junit \
        --fail-on-regression \
        -o repolens-compare.xml
  artifacts:
    when: always
    reports:
      junit: repolens-compare.xml
```

### Jenkins

Jenkins consumes JUnit reports through the built-in `junit` step (from the
JUnit plugin). In a declarative `Jenkinsfile`:

```groovy
pipeline {
  agent any
  stages {
    stage('Audit') {
      steps {
        sh 'repolens report --format junit -o repolens-report.xml'
      }
      post {
        always {
          junit testResults: 'repolens-report.xml',
                allowEmptyResults: true,
                skipPublishingChecks: true
        }
      }
    }
  }
}
```

In a scripted pipeline:

```groovy
node {
  stage('Audit') {
    sh 'repolens report --format junit -o repolens-report.xml'
    junit 'repolens-report.xml'
  }
}
```

### Bitbucket Pipelines

Bitbucket Pipelines auto-discovers JUnit XML files in
`test-results/*.xml`. Write the report there to have results show up in
the pipeline's "Tests" tab.

```yaml
image: rust:latest

pipelines:
  default:
    - step:
        name: RepoLens audit
        script:
          - cargo install repolens
          - mkdir -p test-results
          - repolens report --format junit -o test-results/repolens.xml
        artifacts:
          - test-results/**
```

### GitHub Actions

GitHub Actions does not display JUnit reports natively, but several
community actions (e.g. `mikepenz/action-junit-report`,
`dorny/test-reporter`) can surface them as check annotations and PR
comments:

```yaml
jobs:
  audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: cargo install repolens
      - run: repolens report --format junit -o repolens-report.xml
      - name: Publish JUnit report
        if: always()
        uses: mikepenz/action-junit-report@v4
        with:
          report_paths: repolens-report.xml
```

## Reference

The full JUnit 10 XML Schema Definition (XSD) is included as a reference
under [`schemas/junit-10.xsd`](../../schemas/junit-10.xsd). RepoLens does
not validate its output against this schema at runtime — it is provided
for documentation and manual validation purposes only.
