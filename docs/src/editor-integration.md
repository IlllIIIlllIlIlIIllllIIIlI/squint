# Editor Integration (LSP)

The linter ships a Language Server Protocol (LSP) server binary that provides real-time
diagnostics in any LSP-capable editor.

## Building the LSP server

The LSP server is feature-gated. Build it with:

```bash
cargo install squint --features lsp --bin squint-lsp
```

Or from source:

```bash
cargo build --release --features lsp --bin squint-lsp
# Binary: target/release/squint-lsp
```

## Protocol

- Transport: stdio
- Document sync: full (re-lints the full document on every change)
- Capabilities: `textDocument/publishDiagnostics`
- Config: reads `squint.toml` or `[tool.squint]` in `pyproject.toml` from the working directory at startup

## Neovim (nvim-lspconfig)

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.squint then
  configs.squint = {
    default_config = {
      cmd = { vim.fn.expand('~/.cargo/bin/squint-lsp') },
      filetypes = { 'sql' },
      root_dir = lspconfig.util.root_pattern('squint.toml', 'pyproject.toml', '.git'),
      settings = {},
    },
  }
end

lspconfig.squint.setup {}
```

## Helix (`languages.toml`)

```toml
[[language]]
name = "sql"
language-servers = ["squint"]

[language-server.squint]
command = "squint-lsp"
```

## VS Code

A minimal extension is included in the repository under `editors/vscode/`.
It is not yet published to the Marketplace, so install it manually:

```bash
cd editors/vscode
npm install
npm run compile
npm run package        # produces squint-0.1.0.vsix
code --install-extension squint-0.1.0.vsix
```

The extension activates automatically for `sql` files. For `jinja-sql`
support, also install the
[Better Jinja](https://marketplace.visualstudio.com/items?itemName=samuelcolvin.jinjahtml)
grammar extension.

### Configuration

| Setting | Default | Description |
|---|---|---|
| `squint.serverPath` | `""` | Path to the `squint-lsp` binary. Leave empty to use `PATH` or `~/.cargo/bin`. |

## Severity in diagnostics

The LSP server maps rule severity to LSP diagnostic severity:

- `error` → `DiagnosticSeverity::ERROR` (red underline in most themes)
- `warning` → `DiagnosticSeverity::WARNING` (yellow underline)

Per-rule severity overrides in `squint.toml` are respected.
