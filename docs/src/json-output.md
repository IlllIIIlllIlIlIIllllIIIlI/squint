# JSON Output

Use `--format json` to get machine-readable output. Useful for CI dashboards, editor
plugins, SARIF converters, and scripts.

```bash
squint --format json models/
```

## Shape

A JSON array — one object per file processed (including files with zero violations):

```json
[
  {
    "path": "models/staging/stg_orders.sql",
    "violations": [
      {
        "line": 3,
        "col": 1,
        "rule_id": "CP01",
        "message": "Expected lowercase keyword, got 'SELECT'",
        "severity": "error"
      },
      {
        "line": 5,
        "col": 45,
        "rule_id": "LT05",
        "message": "Line too long (143 > 120 characters)",
        "severity": "warning"
      }
    ],
    "fixed": false
  },
  {
    "path": "models/staging/stg_customers.sql",
    "violations": [],
    "fixed": false
  }
]
```

### Fields

| Field | Type | Description |
|---|---|---|
| `path` | string | File path as given on the command line or walked from a directory |
| `violations` | array | List of violations; empty array if the file is clean |
| `violations[].line` | integer | 1-based line number |
| `violations[].col` | integer | 1-based column number |
| `violations[].rule_id` | string | Rule ID, e.g. `"CP01"` |
| `violations[].message` | string | Human-readable violation message |
| `violations[].severity` | string | `"error"` or `"warning"` |
| `fixed` | boolean | `true` if `--fix` was passed and the file was modified |

## Combined with --fix

```bash
squint --fix --format json models/
```

`"fixed": true` is set for files that were rewritten. Violations in the output are the
**remaining** violations after fixing (unfixable rules that still triggered).

## Piping to jq

```bash
# Count total violations
squint --format json models/ | jq '[.[].violations | length] | add'

# Show only errors
squint --format json models/ | jq '
  [.[] | select(.violations | length > 0) | {
    path,
    errors: [.violations[] | select(.severity == "error")]
  } | select(.errors | length > 0)]
'

# Files with violations
squint --format json models/ | jq '.[] | select(.violations | length > 0) | .path'
```
