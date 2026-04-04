# CLI Reference

```
squint [OPTIONS] [FILES]...
```

Files and directories can be mixed as positional arguments. Directories are walked
recursively for `*.sql` files, respecting `.gitignore`.

## Options

| Flag | Description |
|---|---|
| `--fix` | Rewrite files in place, applying all auto-fixable violations. Remaining violations are reported after fixing. Cannot be combined with `--check`. |
| `--check` | Exit 1 if any file would be changed by `--fix`, without writing. Standard CI gate. Cannot be combined with `--fix`. |
| `--rules <IDs>` | Comma-separated rule IDs to run, e.g. `CP01,LT05`. Runs all rules by default. |
| `--max-line-length <N>` | Override the `LT05` line length limit. Equivalent to `[rules.LT05] max_line_length = N` in config. |
| `-q, --quiet` | Show only the violation count per file, not individual violations. |
| `--format <fmt>` | Output format: `text` (default) or `json`. |
| `--output <file>` | Write violations to a file in addition to stdout. |
| `--exclude <pattern>` | Glob pattern to exclude (repeatable). Merged with the `exclude` list in config. |
| `--stdin-filename <NAME>` | Read SQL from stdin and report violations under this filename. Cannot be combined with positional file arguments. |
| `--version` | Print version and exit. |
| `-h, --help` | Print help and exit. |

## Exit codes

| Code | Meaning |
|---|---|
| `0` | No `error`-severity violations (warnings are allowed) |
| `1` | One or more `error`-severity violations; or `--check` detected drift |
| `2` | I/O error (file not found, permission denied, etc.) |

## Output format

### Text (default)

```
path/to/file.sql:LINE:COL: [SEVERITY] [RULE_ID] message
```

Example:

```
models/stg_orders.sql:3:1: [error] [CP01] Expected lowercase keyword, got 'SELECT'
models/stg_orders.sql:5:45: [warning] [LT05] Line too long (143 > 120 characters)

Found 1 error and 1 warning in 1 file.
```

### JSON (`--format json`)

See [JSON Output](json-output.md).

## Environment

The linter reads `squint.toml` by walking up from the current working directory.
CLI flags always override config file values.
