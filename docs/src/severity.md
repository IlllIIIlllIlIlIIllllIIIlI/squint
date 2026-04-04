# Severity Levels

Every rule has a severity: **error** or **warning**. All rules default to `error`.

## Effect on exit code

| Severity | Reported | Affects exit code |
|---|---|---|
| `error` | Yes | Yes — exit 1 if any errors exist |
| `warning` | Yes | No — exit 0 even if warnings exist |

This lets teams adopt rules incrementally: configure new rules as `warning` during
a rollout period, then promote them to `error` once the codebase is clean.

## Configuring severity

Override per rule in `.sql-linter.toml`:

```toml
[rules.severity]
LT05 = "warning"   # long lines are warnings
CP02 = "warning"   # identifier casing is a warning during rollout
CP01 = "error"     # keyword casing is always an error
```

Only rules you want to change need to appear here. All others remain at their default
(`error`).

## Text output

Severity is shown in brackets before the rule ID:

```
models/stg_orders.sql:3:1: [error] [CP01] Expected lowercase keyword, got 'SELECT'
models/stg_orders.sql:5:45: [warning] [LT05] Line too long (143 > 120 characters)
```

The summary line distinguishes errors from warnings:

```
Found 1 error and 1 warning in 1 file.
```

## JSON output

The `severity` field is included per violation:

```json
{
  "line": 3,
  "col": 1,
  "rule_id": "CP01",
  "message": "Expected lowercase keyword, got 'SELECT'",
  "severity": "error"
}
```

## LSP diagnostics

The LSP server maps `error` → `DiagnosticSeverity::ERROR` and `warning` →
`DiagnosticSeverity::WARNING`, so your editor shows them with the appropriate
indicators (red vs yellow underlines in most themes).
