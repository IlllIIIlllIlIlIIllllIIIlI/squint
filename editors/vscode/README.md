# squint for VS Code

Real-time SQL linting for dbt/Jinja SQL files, powered by the
[squint](https://github.com/IlllIIIlllIlIlIIllllIIIlI/squint) LSP server.

Diagnostics (errors and warnings) appear inline in the editor and in the
Problems panel as you type.

## Requirements

The `squint-lsp` binary must be installed separately:

```bash
cargo install squint --features lsp --bin squint-lsp
```

This places the binary at `~/.cargo/bin/squint-lsp`, which the extension
finds automatically.

## Configuration

| Setting | Default | Description |
|---|---|---|
| `squint.serverPath` | `""` | Absolute path to the `squint-lsp` binary. Leave empty to use the binary from `PATH` or `~/.cargo/bin`. |

Example `settings.json` entry for a non-standard install location:

```json
"squint.serverPath": "/home/you/projects/squint/target/release/squint-lsp"
```

## Building and installing

### Prerequisites

```bash
cd editors/vscode
npm install
```

### Compile

```bash
npm run compile
```

### Package as .vsix

```bash
npm run package
# Produces: squint-0.2.0.vsix
```

### Install in VS Code

```bash
code --install-extension squint-0.2.0.vsix
```

Or: Extensions panel → ··· menu → "Install from VSIX…"

## Supported file types

- `sql`
- `jinja-sql` (requires a grammar extension such as [Better Jinja](https://marketplace.visualstudio.com/items?itemName=samuelcolvin.jinjahtml))

## Configuration file

The LSP server reads `squint.toml` or `[tool.squint]` in `pyproject.toml` from
the workspace root at startup. Rule severity overrides configured there are
reflected in editor diagnostics.
