# Contributing

See [CONTRIBUTING.md](https://github.com/IlllIIIlllIlIlIIllllIIIlI/squint/blob/main/CONTRIBUTING.md)
in the repository for the full contributing guide, including:

- Prerequisites and getting started
- Running tests and code quality gates
- Commit conventions (Conventional Commits)
- Step-by-step guide to adding a new rule
- Key implementation notes (token model, clause context, bracket depth)
- Pull request checklist

## Quick reference

```bash
# Clone and build
git clone https://github.com/IlllIIIlllIlIlIIllllIIIlI/squint
cd squint
cargo build

# Run all tests
cargo test

# Code quality (same checks as CI)
cargo fmt --check
cargo clippy -- -D warnings
cargo test
cargo deny check

# Fuzz testing (requires nightly)
cargo install cargo-fuzz
cargo +nightly fuzz run fuzz_lint fuzz/seeds/fuzz_lint -- -max_total_time=60
```

## Architecture

The linter pipeline:

```
Source text
  └─ Lexer (logos DFA)
       └─ Vec<LexedToken>    zero-copy slices into source
            └─ Parser
                 └─ Vec<Node>    flat token list, no AST
                      └─ Rules
                           └─ Vec<Violation>
                           └─ Vec<Fix>
```

Key design decisions:

- **No AST.** Rules operate on a flat `Vec<Node>` with bracket depth tracking.
  This keeps the architecture simple and rules easy to write.
- **Zero-copy tokens.** `Node::value` is a `&'src str` slice into the original
  source — no heap allocation for token text.
- **Suppression is transparent.** Rules emit every violation they find.
  `lint_source` filters out suppressed lines after collecting from all rules.

## Adding a rule

See the [Contributing Guide](https://github.com/IlllIIIlllIlIlIIllllIIIlI/squint/blob/main/CONTRIBUTING.md#adding-a-new-rule)
for the 6-step walkthrough with full code examples.
