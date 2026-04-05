# squint

A fast SQL linter for [dbt](https://www.getdbt.com/) and Jinja SQL files, written in Rust.
Modelled on [sqlfluff](https://docs.sqlfluff.com/) and [sqlfmt](https://sqlfmt.com/), with
first-class Jinja support and a focus on speed.

## Features

- **36 rules** covering capitalisation, layout, aliasing, references, structure, and more
- **Auto-fix** — rewrite files in place with `--fix`, or check for drift with `--check`
- **Jinja-aware** — tokenises `{{ }}` and `{% %}` blocks without stripping them
- **`-- noqa` suppression** — per-line and per-rule (`-- noqa: CP01,LT05`)
- **`-- fmt: off` blocks** — opt out of formatting for hand-crafted sections
- **Severity levels** — configure rules as `error` (exit 1) or `warning` (exit 0)
- **JSON output** — for CI dashboards, editor plugins, and scripts
- **LSP server** — real-time diagnostics in VS Code, Neovim, Helix, and any LSP client
- **Fast** — ~100 µs to lint a 240-line file; 64 files in ~1.4 ms on a modern laptop

## Design philosophy

The linter enforces **lowercase SQL** by default — keywords, function names, type names,
boolean/null literals, and unquoted identifiers. This matches the style used by
[sqlfmt](https://sqlfmt.com/) and is the default for most dbt projects. All capitalisation
rules are auto-fixable.

The linter has no AST — it operates on a flat token list produced by a
[Logos](https://github.com/maciejhirsz/logos)-based DFA tokeniser. This keeps the
architecture simple and makes it easy to add new rules.

## Getting help

- **GitHub Issues**: [IlllIIIlllIlIlIIllllIIIlI/squint/issues](https://github.com/IlllIIIlllIlIlIIllllIIIlI/squint/issues)
- **Source**: [IlllIIIlllIlIlIIllllIIIlI/squint](https://github.com/IlllIIIlllIlIlIIllllIIIlI/squint)
