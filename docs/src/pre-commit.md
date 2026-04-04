# pre-commit Integration

[pre-commit](https://pre-commit.com/) is the standard hook manager for dbt projects.
The linter ships a `.pre-commit-hooks.yaml` so you can add it directly.

## Setup

Add to your `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: https://github.com/IlllIIIlllIlIlIIllllIIIlI/squint
    rev: v0.1.0   # pin to a release tag
    hooks:
      - id: sql-linter
```

Run `pre-commit install` once to register the hooks. On the first run, pre-commit
builds the binary from source (requires Rust on the machine). The binary is cached
for subsequent runs.

## Available hooks

| Hook ID | What it does |
|---|---|
| `sql-linter` | Lint staged SQL files and exit 1 on any violations |
| `sql-linter-fix` | Auto-fix staged SQL files in place |

### Lint only

```yaml
- id: sql-linter
```

Fails the commit if any violations are found. The developer must fix violations
(or suppress them with `-- noqa`) before committing.

### Auto-fix on commit

```yaml
- id: sql-linter-fix
```

Rewrites staged files in place before the commit. If files are modified, pre-commit
will abort the commit and ask you to `git add` the fixed files. This is the most
frictionless setup for teams.

## Config file

The linter reads `.sql-linter.toml` from the working directory at hook execution time.
Project-level config (severity overrides, line length, etc.) is applied automatically.

## Pinning a version

Always pin to a tag (`rev: v0.1.0`) rather than a branch name. pre-commit caches the
binary per rev, so floating tags like `main` defeat caching and force a rebuild on every
run.
