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
- Config: reads `squint.toml` from the working directory at startup

## Neovim (nvim-lspconfig)

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.squint then
  configs.squint = {
    default_config = {
      cmd = { vim.fn.expand('~/.cargo/bin/squint-lsp') },
      filetypes = { 'sql' },
      root_dir = lspconfig.util.root_pattern('squint.toml', '.git'),
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

A VS Code extension is planned but not yet available. In the meantime, you can use the
[Generic LSP Client](https://marketplace.visualstudio.com/items?itemName=kosz78.generic-lsp)
extension with:

```json
{
  "genericLsp.servers": [
    {
      "name": "squint",
      "command": "squint-lsp",
      "filetypes": ["sql"]
    }
  ]
}
```

## Severity in diagnostics

The LSP server maps rule severity to LSP diagnostic severity:

- `error` → `DiagnosticSeverity::ERROR` (red underline in most themes)
- `warning` → `DiagnosticSeverity::WARNING` (yellow underline)

Per-rule severity overrides in `squint.toml` are respected.
