# Configuration

Create `.sql-linter.toml` in your project root (or any ancestor directory). The linter
walks up from the current working directory to find the nearest config file.

CLI flags always override config file values.

## Full reference

```toml
# Glob patterns to exclude from linting (relative to the config file).
# Merged with any --exclude flags passed on the command line.
exclude = [
    "target/**",
    "**/node_modules/**",
    "vendor/*.sql",
]

# ── Layout ────────────────────────────────────────────────────────────────────

[rules.LT05]
max_line_length = 120        # default: 120
ignore_comment_lines = false # parsed but not yet enforced

# ── Aliasing ──────────────────────────────────────────────────────────────────

[rules.AL06]
min_alias_length = 1         # default: 1  (0 = no minimum)
max_alias_length = 0         # default: 0  (0 = no maximum)

# ── Convention ────────────────────────────────────────────────────────────────

[rules.CV03]
# "forbid" (default) — trailing comma before FROM is not allowed
# "require"          — trailing comma before FROM is required
select_clause_trailing_comma = "forbid"

[rules.CV04]
# false (default) — require COUNT(*)
# true            — require COUNT(1)
prefer_count_1 = false

# ── Ambiguous ─────────────────────────────────────────────────────────────────

[rules.AM06]
# "explicit"   (default) — all GROUP BY / ORDER BY refs must be column names
# "implicit"             — all refs must be positional numbers
# "consistent"           — either style allowed, but not mixed within one clause
group_by_and_order_by_style = "explicit"

# ── Severity overrides ────────────────────────────────────────────────────────
# All rules default to "error". Override per rule to "warning" (reported but
# exit 0) or back to "error".

[rules.severity]
LT05 = "warning"   # long lines are warnings, not hard errors
CP02 = "warning"   # identifier casing is a warning during rollout
```

## Severity levels

See [Severity Levels](severity.md) for a full explanation of how error vs warning
affects exit codes, output, and JSON.

## Disabling rules

There is no per-rule `enabled = false` option. To skip a rule entirely, use `--rules`
on the command line to run only the rules you want:

```bash
squint --rules CP01,LT03,LT05 models/
```

Or suppress individual lines with `-- noqa` — see [Suppression](suppression.md).
