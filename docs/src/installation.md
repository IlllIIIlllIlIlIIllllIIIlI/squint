# Installation

## pip (Python projects)

If your project already uses pip or a `requirements.txt`, this is the easiest option:

```bash
pip install squint
```

This downloads a pre-built binary wheel for your platform — no Rust toolchain needed.
Works with pip ≥ 21, Python ≥ 3.8.

For dbt projects using `uv`:
```bash
uv add --dev squint
```

## From crates.io

```bash
cargo install squint
```

This compiles from source. Requires [Rust](https://rustup.rs/) stable.

## From source

```bash
git clone https://github.com/IlllIIIlllIlIlIIllllIIIlI/squint
cd squint
cargo install --path .
```

## Pre-built binaries

Pre-built binaries for Linux (x86_64, aarch64), macOS (x86_64, Apple Silicon), and
Windows (x86_64) are attached to each [GitHub Release](https://github.com/IlllIIIlllIlIlIIllllIIIlI/squint/releases).

Download the archive for your platform, extract it, and place the binary somewhere on
your `PATH`.

## LSP server (optional)

The language server binary is feature-gated and must be built explicitly:

```bash
cargo install squint --features lsp --bin squint-lsp
```

Or from source:

```bash
cargo install --path . --features lsp --bin squint-lsp
```

See [Editor Integration](editor-integration.md) for setup instructions.

## Verify installation

```bash
squint --version
```

## pre-commit

No manual installation needed — pre-commit builds the binary from source on first use.
See [pre-commit Integration](pre-commit.md).
