# Changelog

All notable changes to this project will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
This project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

<!-- When cutting a release:
  1. Rename [Unreleased] to [x.y.z] - YYYY-MM-DD
  2. Add a new empty [Unreleased] section at the top
  3. Update the comparison links at the bottom of this file
-->

## [Unreleased]

## [0.2.0] - 2026-04-05

### Added

#### Rules (36 total)

**Capitalisation** — all rules enforce lowercase output and are auto-fixable:
- `CP01` — SQL keywords must be lowercase (`SELECT` → `select`)
- `CP02` — Unquoted identifiers must be lowercase
- `CP03` — Function names must be lowercase (`COUNT` → `count`)
- `CP04` — Boolean/null literals must be lowercase (`TRUE` → `true`, `NULL` → `null`)
- `CP05` — Data type names must be lowercase (`INT` → `int`, `VARCHAR` → `varchar`)

**Layout:**
- `LT01` — No space before comma; no consecutive spaces mid-line (auto-fix)
- `LT02` — Indentation must use spaces, not tabs, and be a multiple of 4
- `LT03` — No trailing whitespace on any line (auto-fix)
- `LT05` — Lines must not exceed `max_line_length` characters (default 120; configurable)
- `LT06` — No space between a function name and its `(` (auto-fix)
- `LT07` — CTE closing `)` must be on its own line
- `LT08` — A blank line is required after each CTE closing `)`
- `LT09` — SQL clauses must appear in the standard order (`SELECT → FROM → WHERE → GROUP BY → HAVING → ORDER BY → LIMIT`)
- `LT10` — `DISTINCT`/`ALL` modifier must be on the same line as `SELECT`
- `LT11` — Set operators (`UNION`, `INTERSECT`, `EXCEPT`) must be on their own line
- `LT12` — File must end with exactly one trailing newline (auto-fix)

**Aliasing:**
- `AL02` — Column aliases must use explicit `AS` keyword
- `AL03` — Expressions in `SELECT` must have an alias
- `AL04` — Table aliases must be unique within a query
- `AL05` — Table aliases that are defined but never referenced
- `AL06` — Table alias length must be within configured bounds (configurable)
- `AL08` — Column aliases in a `SELECT` must be unique (case-insensitive)
- `AL09` — A column must not be aliased to itself (`col AS col`)

**Ambiguous:**
- `AM01` — `SELECT DISTINCT` with `GROUP BY` is redundant
- `AM02` — `UNION` must be followed by `ALL` or `DISTINCT`
- `AM05` — Implicit comma joins in `FROM` are forbidden; use explicit `JOIN` syntax
- `AM06` — `GROUP BY`/`ORDER BY` must use a consistent column reference style (configurable)

**Convention:**
- `CV03` — Trailing comma policy in `SELECT` clauses (configurable: `forbid` or `require`)
- `CV04` — Consistent row-counting syntax: `COUNT(*)` vs `COUNT(1)` (configurable)
- `CV05` — `NULL` comparisons must use `IS NULL` / `IS NOT NULL` (auto-fix)
- `CV10` — Identifiers must use a consistent quoting style within a file

**References:**
- `RF01` — Qualified column references must use a known table alias
- `RF02` — Wildcard (`*`) column references are not allowed; list columns explicitly

**Structure:**
- `ST03` — CTEs that are defined but never referenced
- `ST08` — `COUNT(DISTINCT *)` is not valid SQL

**Jinja:**
- `JJ01` — Jinja tags must have single-space padding: `{{ col }}`, `{% if cond %}`

#### CLI
- `--fix` — Rewrite files in place, applying all auto-fixable violations
- `--check` — Exit 1 if any file would be changed by `--fix`, without writing (CI gate)
- `--rules <IDs>` — Run only the specified comma-separated rule IDs
- `--max-line-length <N>` — Override the `LT05` line length limit
- `-q / --quiet` — Show only the violation count per file
- `--format json` — Machine-readable JSON output
- `--output <file>` — Write violations to a file in addition to stdout
- `--exclude <pattern>` — Glob pattern to exclude files (repeatable)
- `--stdin-filename <NAME>` — Read SQL from stdin; report violations under this filename
- Violation summary line at end of text output (e.g. `Found 12 violations in 3 files.`)
- Parallel file processing via `rayon`; sorted deterministic output

#### Suppression
- `-- noqa` — Suppress all rule violations on a line
- `-- noqa: CP01,LT05` — Suppress specific rules on a line (case-insensitive)
- `-- fmt: off` / `-- fmt: on` — Block suppression; opt out of all linting for a region
- `-- fmt: off` inline — Suppress a single line only

#### Severity levels
- `Error` (default) and `Warning` severity per rule
- Exit code 1 only when at least one `Error`-severity violation exists
- Per-rule severity override in `squint.toml` under `[rules.severity]`

#### Configuration
- `squint.toml` config file; loaded by walking up from `cwd`
- Per-rule configuration for `LT05`, `AL06`, `CV03`, `CV04`, `AM06`
- `exclude` glob patterns at the top level

#### LSP server
- `squint-lsp` binary, feature-gated behind `--features lsp`
- Full document sync; publishes diagnostics on open/change
- Neovim (`nvim-lspconfig`) integration documented in `README.md`

#### Infrastructure
- GitHub Actions CI: build, test, `rustfmt`, `clippy` on every push and PR
- GitHub Actions benchmark regression check on PRs (Criterion + `critcmp`)
- Criterion microbenchmarks; Hyperfine wall-clock comparison vs sqlfluff/sqlfmt
- `.pre-commit-hooks.yaml` — `squint` (lint) and `squint-fix` (lint + fix) hooks
- MIT license
- cargo-fuzz integration: three fuzz targets (`fuzz_lex`, `fuzz_lint`, `fuzz_fix`); 30-second CI smoke run on every PR, 5-minute weekly scheduled run

### Fixed

- **Panic on unterminated string literals or Jinja/block-comment blocks containing multi-byte UTF-8 characters** — `lex_string`, `lex_jinja_expr`, `lex_jinja_stmt`, `lex_jinja_comment`, and `lex_block_comment` all returned `Filter::Skip` when their closing delimiter was missing. Logos advances past the opening bytes without yielding them, so they are never appended to `pending_prefix`. This made the next token's prefix shorter than the actual byte gap; fix offsets computed as `spos - prefix.len()` could then land inside a multi-byte character. Two failure modes: `replace_range` panic (intercepted by the char-boundary guard in `apply_fixes`) and a `str` slice panic in `byte_to_line` which is called with `fix.start` before `apply_fixes` is reached. Fixed by: (1) all five callbacks now bump to end of input and emit on missing delimiter; (2) `byte_to_line` uses `.as_bytes()` slicing instead of `str` slicing. Found by `cargo fuzz`.

### Changed

- **Config file renamed** from `.sql-linter.toml` to `squint.toml`. Rename your config file before upgrading.
- **Pre-commit hook IDs renamed**: `sql-linter` → `squint`, `sql-linter-fix` → `squint-fix`. Update your `.pre-commit-config.yaml` accordingly.
- **`pyproject.toml` support**: config can now be placed under `[tool.squint]` in `pyproject.toml`. squint checks `squint.toml` first, then `pyproject.toml` with a `[tool.squint]` section, at each level of the directory walk.

[Unreleased]: https://github.com/IlllIIIlllIlIlIIllllIIIlI/squint/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/IlllIIIlllIlIlIIllllIIIlI/squint/releases/tag/v0.2.0
