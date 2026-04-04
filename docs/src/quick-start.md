# Quick Start

## Lint a directory

```bash
squint models/
```

Output:

```
models/staging/stg_orders.sql:3:1: [error] [CP01] Expected lowercase keyword, got 'SELECT'
models/staging/stg_orders.sql:3:8: [error] [CP01] Expected lowercase keyword, got 'FROM'
models/marts/fct_orders.sql:12:1: [error] [LT05] Line too long (143 > 120 characters)

Found 3 errors in 2 files.
```

## Auto-fix violations

```bash
squint --fix models/
```

Rewrites files in place. All fixable rules are applied (see the [fixable column](rules/overview.md)
in the rules reference). After fixing, any remaining unfixable violations are reported.

## CI gate

```bash
squint --check models/
```

Exits `1` if any file would be changed by `--fix`. Does not write any files.
Use this in CI to enforce that committed SQL is already formatted:

```yaml
# .github/workflows/ci.yml
- name: Check SQL formatting
  run: squint --check models/
```

## Run specific rules

```bash
squint --rules CP01,LT05 models/
```

## Lint from stdin

```bash
cat models/my_model.sql | squint --stdin-filename models/my_model.sql
```

## Typical dbt project setup

1. Add a `.sql-linter.toml` at the project root (see [Configuration](configuration.md))
2. Add a pre-commit hook (see [pre-commit Integration](pre-commit.md))
3. Add `squint --check models/` to your CI pipeline

That's it. The linter walks directories recursively, respects `.gitignore`, and processes
files in parallel.
