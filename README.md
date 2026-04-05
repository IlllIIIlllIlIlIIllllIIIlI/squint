# squint

A fast SQL linter and formatter for dbt/Jinja SQL files, written in Rust. Modelled on
[sqlfluff](https://docs.sqlfluff.com/) and [sqlfmt](https://sqlfmt.com/), with first-class
support for Jinja templating.

## Features

- **36 rules** covering capitalisation, layout, aliasing, references, structure, and more
- **Auto-fix** — rewrite files in place with `--fix`, or check for drift with `--check`
- **Jinja-aware** — tokenises `{{ }}` and `{% %}` blocks without stripping them
- **`-- noqa` suppression** — per-line and per-rule (`-- noqa: CP01,LT05`)
- **`-- fmt: off` blocks** — opt out of formatting for hand-crafted sections
- **Severity levels** — configure rules as `error` (exit 1) or `warning` (exit 0)
- **JSON output** — for CI dashboards, editor plugins, and scripts
- **LSP server** — real-time diagnostics in VS Code, Neovim, Helix, and any LSP client
- **Fast** — ~100 µs to lint a 240-line file; 64 files in parallel in ~1.4 ms

**[Full documentation →](https://IlllIIIlllIlIlIIllllIIIlI.github.io/squint)**

## Installation

**pip** (no Rust required — downloads a pre-built binary):

```bash
pip install squint
# or with uv
uv add --dev squint
```

**cargo** (compiles from source):

```bash
cargo install squint
```

**Pre-built binaries** for Linux, macOS, and Windows are attached to each
[GitHub Release](https://github.com/IlllIIIlllIlIlIIllllIIIlI/squint/releases).

**LSP server** (optional, feature-gated):

```bash
cargo install squint --features lsp --bin squint-lsp
```

## Quick start

```bash
# Lint all SQL files in a directory
squint models/

# Auto-fix violations
squint --fix models/

# CI gate — exit 1 if any file would be changed by --fix
squint --check models/

# Lint a single file piped from stdin
cat models/my_model.sql | squint --stdin-filename models/my_model.sql
```

## Usage

```
squint [OPTIONS] [FILES]...
```

Files and directories can be mixed. Directories are walked recursively for `*.sql` files,
respecting `.gitignore`.

| Flag | Description |
|---|---|
| `--fix` | Rewrite files in place, applying all auto-fixable violations |
| `--check` | Exit 1 if any file would be changed by `--fix`, without writing (CI gate) |
| `--rules <IDs>` | Comma-separated rule IDs to run, e.g. `--rules CP01,LT05` |
| `--max-line-length <N>` | Override the `LT05` line length limit |
| `-q / --quiet` | Show only the violation count per file |
| `--format <fmt>` | Output format: `text` (default) or `json` |
| `--output <file>` | Write violations to a file in addition to stdout |
| `--exclude <pattern>` | Glob pattern to exclude (repeatable) |
| `--stdin-filename <NAME>` | Read SQL from stdin; report violations under this filename |

Exit codes: `0` = no errors, `1` = one or more `error`-severity violations (or `--check`
detected drift), `2` = I/O error.

## Configuration

Create `squint.toml` in your project root, or use `[tool.squint]` in `pyproject.toml`.
CLI flags override config values.

```toml
# Paths to exclude (matched relative to the config file)
exclude = ["target/**", "**/node_modules/**", "vendor/*.sql"]

[rules.LT05]
max_line_length = 120        # default: 120

[rules.AL06]
min_alias_length = 1         # default: 1 (0 = no minimum)
max_alias_length = 0         # default: 0 (0 = no maximum)

[rules.CV03]
select_clause_trailing_comma = "forbid"  # "forbid" (default) | "require"

[rules.CV04]
prefer_count_1 = false       # false = require COUNT(*) (default); true = require COUNT(1)

[rules.AM06]
group_by_and_order_by_style = "explicit"  # "explicit" | "implicit" | "consistent"

# Per-rule severity overrides (all rules default to "error")
[rules.severity]
LT05 = "warning"   # long lines are warnings, not errors
CP01 = "error"     # keyword casing is still an error
```

## Rules

All rules enforce **lowercase** SQL by default (keywords, identifiers, function names,
type names, boolean/null literals). Fixable rules are marked with ✓.

### Capitalisation

| ID | Description | Fixable |
|---|---|---|
| CP01 | Keywords must be lowercase (`SELECT`, `FROM`, `WHERE`, …) | ✓ |
| CP02 | Unquoted identifiers must be lowercase | ✓ |
| CP03 | Function names must be lowercase (`COUNT`, `COALESCE`, …) | ✓ |
| CP04 | Boolean/null literals must be lowercase (`TRUE`, `FALSE`, `NULL`) | ✓ |
| CP05 | Data type names must be lowercase (`INT`, `VARCHAR`, `TIMESTAMP`, …) | ✓ |

### Layout

| ID | Description | Fixable |
|---|---|---|
| LT01 | No space before comma; no consecutive spaces mid-line; no space between function name and `(` | ✓ |
| LT02 | Indentation must use spaces (no tabs) and be a multiple of 4 | |
| LT03 | Lines must not have trailing whitespace (spaces or tabs before the newline) | ✓ |
| LT05 | Lines must not exceed `max_line_length` characters (default 120) | |
| LT06 | No space between a function name and `(` — e.g. `count (id)` → `count(id)` | ✓ |
| LT07 | CTE closing `)` must be on its own line | |
| LT08 | A blank line is required after each CTE closing `)` | |
| LT09 | Clauses in standard order: `SELECT → FROM → WHERE → GROUP BY → HAVING → ORDER BY → LIMIT` | |
| LT10 | `DISTINCT`/`ALL` modifier must be on the same line as `SELECT` | |
| LT11 | Set operators (`UNION`, `INTERSECT`, `EXCEPT`) must be on their own line | |
| LT12 | File must end with exactly one trailing newline | ✓ |

### Aliasing

| ID | Description | Fixable |
|---|---|---|
| AL02 | Column aliases must use explicit `AS` keyword | |
| AL03 | Expressions in `SELECT` must have an alias | |
| AL04 | Table aliases must be unique within a query | |
| AL05 | Table aliases that are defined but never referenced | |
| AL06 | Table alias length must be within configured bounds | |
| AL08 | Column aliases in a `SELECT` must be unique (case-insensitive) | |
| AL09 | A column must not be aliased to itself (`col AS col`) | |

### Ambiguous

| ID | Description | Fixable |
|---|---|---|
| AM01 | `SELECT DISTINCT` with `GROUP BY` is redundant | |
| AM02 | `UNION` must be followed by `ALL` or `DISTINCT` | |
| AM05 | Implicit comma joins in `FROM` clauses are forbidden; use explicit `JOIN` syntax | |
| AM06 | `GROUP BY`/`ORDER BY` must use a consistent reference style | |

### Convention

| ID | Description | Fixable |
|---|---|---|
| CV03 | Trailing comma policy in `SELECT` clauses (forbid or require) | |
| CV04 | Consistent row-counting syntax: `COUNT(*)` vs `COUNT(1)` | |
| CV05 | `NULL` comparisons must use `IS NULL`/`IS NOT NULL` | ✓ |
| CV10 | Identifiers must use a consistent quoting style within a file | |

### References

| ID | Description | Fixable |
|---|---|---|
| RF01 | Qualified column references must use a known table alias | |
| RF02 | Wildcard (`*`) column references are not allowed; list columns explicitly | |

### Structure

| ID | Description | Fixable |
|---|---|---|
| ST03 | CTEs that are defined but never referenced | |
| ST08 | `COUNT(DISTINCT *)` is not valid SQL | |

### Jinja

| ID | Description | Fixable |
|---|---|---|
| JJ01 | Jinja tags must have single-space padding: `{{ col }}`, `{% if cond %}` | |

## Suppression

### Per-line: `-- noqa`

```sql
SELECT A  -- noqa            suppresses all rule violations on this line
FROM T    -- noqa: CP01      suppresses only CP01 on this line
WHERE X = 1  -- noqa: CP01, LT05   suppresses CP01 and LT05
```

Rule IDs are case-insensitive. `-- noqa` also suppresses auto-fixes on that line.

### Block: `-- fmt: off` / `-- fmt: on`

```sql
-- fmt: off
SELECT A, B,   -- hand-crafted alignment, not touched by the linter
       C
-- fmt: on
SELECT d FROM t  -- linting resumes here
```

`-- fmt: off` inline (after SQL on a line) suppresses only that line. A standalone
`-- fmt: off` with no matching `-- fmt: on` suppresses to end of file.

## Severity levels

Rules default to `error` severity (exit 1). Override per rule in config:

```toml
[rules.severity]
LT05 = "warning"
```

Warnings are reported but do not affect the exit code. JSON output includes a `"severity"`
field per violation.

## JSON output

```bash
squint --format json models/
```

```json
[
  {
    "path": "models/my_model.sql",
    "violations": [
      { "line": 5, "col": 1, "rule_id": "CP01", "message": "...", "severity": "error" }
    ],
    "fixed": false
  }
]
```

## LSP server

Build the LSP binary (requires `--features lsp`):

```bash
cargo build --release --features lsp --bin squint-lsp
```

### Neovim (nvim-lspconfig)

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.squint then
  configs.squint = {
    default_config = {
      cmd = { '/path/to/squint-lsp' },
      filetypes = { 'sql' },
      root_dir = lspconfig.util.root_pattern('squint.toml', 'pyproject.toml', '.git'),
      settings = {},
    },
  }
end

lspconfig.squint.setup {}
```

The LSP server loads `squint.toml` from the working directory at startup, so
per-rule severity overrides are respected in editor diagnostics.

## pre-commit

Add to your `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: https://github.com/IlllIIIlllIlIlIIllllIIIlI/squint
    rev: v0.1.0
    hooks:
      - id: squint
```

pre-commit will build the binary from source on first run (requires Rust installed on
the CI runner or developer machine). Subsequent runs use the cached build.

**Available hooks:**

| Hook ID | Behaviour |
|---|---|
| `squint` | Lint staged `.sql` files; fail the commit if any `error`-severity violations exist |
| `squint-fix` | Lint and auto-fix staged `.sql` files in place; pre-commit re-stages the changes |

**Lint only (recommended for CI):**

```yaml
- repo: https://github.com/IlllIIIlllIlIlIIllllIIIlI/squint
  rev: v0.1.0
  hooks:
    - id: squint
```

**Lint + auto-fix on commit (recommended for local development):**

```yaml
- repo: https://github.com/IlllIIIlllIlIlIIllllIIIlI/squint
  rev: v0.1.0
  hooks:
    - id: squint-fix
```

**Run only specific rules:**

```yaml
- repo: https://github.com/IlllIIIlllIlIlIIllllIIIlI/squint
  rev: v0.1.0
  hooks:
    - id: squint
      args: [--rules, "CP01,LT05,LT12"]
```

**Suppress a violation inline** without disabling the whole hook:

```sql
SELECT A  -- noqa: CP01
FROM T
```

## Contributing

### Running tests

```bash
cargo test                        # all tests
cargo test --test layout          # one integration test group
cargo test --test noqa            # noqa suppression tests
```

### Adding a rule

The short steps:

1. Create `src/rules/<group>/<id>.rs` and implement the `Rule` trait
2. Re-export from the group's `mod.rs`
3. Instantiate in `build_rules()` in `src/lib.rs`
4. Add integration tests in `tests/<group>.rs`

### Running benchmarks

```bash
cargo bench                       # Criterion microbenchmarks
./scripts/bench_compare.sh        # wall-clock comparison vs sqlfluff/sqlfmt
```

### Fuzz testing

```bash
cargo install cargo-fuzz
cargo +nightly fuzz run fuzz_lint fuzz/seeds/fuzz_lint -- -max_total_time=60
```

See `fuzz/fuzz_targets/` for all three targets (`fuzz_lex`, `fuzz_lint`, `fuzz_fix`)
and `CONTRIBUTING.md` for full instructions.

## License

MIT — see [LICENSE](LICENSE).
