# Contributing

Thank you for considering a contribution. This document covers everything you need to
get started: running the project locally, the test and lint workflow, and how to add a
new rule.

## Prerequisites

- [Rust](https://rustup.rs/) stable (the CI matrix tests stable only)
- `cargo` — comes with Rust

Optional tools:

| Tool | Install | Used for |
|---|---|---|
| `cargo-audit` | `cargo install cargo-audit` | Dependency vulnerability scanning |
| `git-cliff` | `cargo install git-cliff` | Automated changelog generation |
| `taplo-cli` | `cargo install taplo-cli` | TOML formatter (used by pre-commit hook) |
| `hyperfine` | system package | Wall-clock benchmarks vs sqlfluff/sqlfmt |
| `critcmp` | `cargo install critcmp` | Comparing two Criterion baselines |
| `cargo-fuzz` | `cargo install cargo-fuzz` | Fuzz testing (requires nightly) |

## Getting started

```bash
git clone https://github.com/IlllIIIlllIlIlIIllllIIIlI/squint
cd squint
cargo build
cargo test

# Install pre-commit hooks (recommended)
pip install pre-commit
pre-commit install
```

## Running the linter on itself

```bash
cargo build --release
./target/release/squint src/
```

## Testing

```bash
cargo test                        # all unit + integration tests
cargo test --test layout          # one integration test group only
cargo test --test noqa            # noqa suppression tests
cargo test -- --nocapture         # show println! output during tests
```

Test files live in `tests/`, one per rule group. Shared helpers are in
`tests/common/mod.rs`:

```rust
pub fn check(rule: impl Rule, sql: &str) -> Vec<Violation>
pub fn fix(rule: impl Rule, sql: &str) -> String
```

## Code quality gates

These must all pass before a PR can merge — the same checks run in CI:

```bash
cargo fmt --check          # formatting
cargo clippy -- -D warnings  # lints
cargo test                 # all tests
cargo audit                # dependency vulnerability scan (optional locally)
cargo deny check           # license, ban, and advisory check
```

Run `cargo fmt` (without `--check`) to auto-format before committing.

## Fuzz testing

Three fuzz targets exercise the lexer, lint pipeline, and fix pipeline against
arbitrary UTF-8 input. They require a nightly toolchain (installed automatically
from `fuzz/rust-toolchain.toml`).

```bash
# Install cargo-fuzz (one-time)
cargo install cargo-fuzz

# Run a target against the seed corpus for 60 seconds
cargo +nightly fuzz run fuzz_lex   fuzz/seeds/fuzz_lex   -- -max_total_time=60
cargo +nightly fuzz run fuzz_lint  fuzz/seeds/fuzz_lint  -- -max_total_time=60
cargo +nightly fuzz run fuzz_fix   fuzz/seeds/fuzz_fix   -- -max_total_time=60

# If a crash is found, reproduce it
cargo +nightly fuzz run fuzz_lint fuzz/artifacts/fuzz_lint/crash-<hash>
```

Fuzz targets live in `fuzz/fuzz_targets/`. Seed inputs are in `fuzz/seeds/`
(12 representative SQL inputs per target). CI runs a 30-second smoke-run on
every PR and a 5-minute run weekly. Crash artifacts are uploaded automatically
if a run fails.

## Commit conventions

This project uses [Conventional Commits](https://www.conventionalcommits.org/) so that
the changelog can be generated automatically with `git-cliff`. See
[RELEASING.md](RELEASING.md) for the full prefix table.

Quick reference:

```
feat(RF02): add wildcard column rule
fix(LT03): handle CRLF line endings correctly
perf: skip noqa scan when source contains no noqa comments
docs: update README installation instructions
test(AM05): add subquery edge-case tests
chore: bump logos to 0.15
```

## Adding a new rule

### 1. Create the rule file

```
src/rules/<group>/<id>.rs
```

Implement the `Rule` trait:

```rust
use crate::rules::{LintContext, Rule, Severity, Violation};

pub struct MyRule;

impl Rule for MyRule {
    fn id(&self) -> &'static str { "XX99" }

    fn check(&self, ctx: &LintContext<'_>) -> Vec<Violation> {
        let mut violations = Vec::new();
        for node in ctx.nodes {
            // ... inspection logic
            let (line, col) = ctx.line_index.offset_to_line_col(node.token.spos);
            violations.push(Violation {
                line,
                col,
                rule_id: self.id(),
                message: "...".to_string(),
                ..Default::default()   // severity defaults to Error
            });
        }
        violations
    }
}
```

For auto-fixable rules, also implement `fixes()`:

```rust
use crate::node::Node;
use crate::rules::Fix;

fn fixes(&self, _nodes: &[Node<'_>], source: &str) -> Vec<Fix> {
    vec![Fix { start, end, replacement: "...".to_string() }]
}
```

### 2. Export from the group mod

In `src/rules/<group>/mod.rs`:

```rust
pub mod xx99;
pub use xx99::MyRule;
```

### 3. Register in `build_rules()`

In `src/lib.rs`:

```rust
use rules::{
    // ...
    mygroup::MyRule,
};

pub fn build_rules(cfg: &Config) -> Vec<Box<dyn Rule>> {
    let raw: Vec<Box<dyn Rule>> = vec![
        // ...
        Box::new(MyRule),
    ];
```

### 4. Add config (if needed)

If the rule is configurable, add a struct to `src/config.rs` under `RulesConfig`:

```rust
#[derive(serde::Deserialize, Default)]
pub struct MyRuleConfig {
    #[serde(default = "default_my_option")]
    pub my_option: bool,
}

fn default_my_option() -> bool { false }
```

Then reference it in `build_rules()`: `Box::new(MyRule::new(cfg.rules.xx99.my_option))`.

### 5. Write tests

Add a unit test inside the rule file:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn violations(sql: &str) -> Vec<Violation> {
        crate::rules::check_rule(MyRule, sql)
    }

    #[test]
    fn test_ok() {
        assert!(violations("select a from t\n").is_empty());
    }

    #[test]
    fn test_flagged() {
        let v = violations("SELECT a FROM t\n");
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].rule_id, "XX99");
    }
}
```

Add integration tests in `tests/<group>.rs` using the `check` and `fix` helpers.

### 6. Update documentation

User-facing docs (`docs/`):
- Add the rule with bad/good SQL examples to `docs/src/rules/<group>.md`
- Add it to the all-rules table in `docs/src/rules/overview.md`

Everything else:
- Add it to the README rules table; update the rule count
- Add it to `CHANGELOG.md` under `[Unreleased] > Added`

## Key implementation notes

### Token model

Rules receive a flat `Vec<Node<'src>>` — there is no AST. Each node has:

```rust
node.value          // &str — zero-copy slice into source text
node.prefix         // String — whitespace before this token
node.token.token_type  // TokenType enum
node.token.spos     // byte offset (start, inclusive)
node.token.epos     // byte offset (end, exclusive)
node.bracket_depth  // depth before this token's bracket is applied
```

### Clause context

`ctx.clauses[i]` gives the SQL clause each node belongs to (`Select`, `From`,
`Where`, `GroupBy`, etc.). It is pre-computed by `lint_source` — do not call
`clause_map(nodes)` inside a rule.

### Bracket depth

`bracket_depth` is recorded *before* the bracket stack updates. So:
- `(` is at the depth of its *surrounding* context
- `)` is at `open_depth + 1`
- To find the matching `)` for a `(` at depth D: search forward for
  `BracketClose` with `bracket_depth == D + 1`

### Analysis helpers

`src/analysis.rs` provides shared helpers for common patterns:

| Function | Returns |
|---|---|
| `prev_non_ws(nodes, idx)` | `Option<usize>` — previous non-whitespace node |
| `next_non_ws(nodes, idx)` | `Option<usize>` — next non-whitespace node |
| `select_items_with_clauses(nodes, clauses)` | `Vec<SelectItem>` — SELECT list items |
| `table_aliases_with_clauses(nodes, clauses)` | `Vec<TableAlias>` — FROM clause aliases |
| `cte_definitions(nodes)` | `Vec<CteDef>` — WITH clause CTE definitions |

### Fixes

`Fix { start, end, replacement }` uses byte offsets. `apply_fixes` applies them
right-to-left so earlier offsets remain valid after each substitution. Overlapping
fixes are silently skipped. `fix_source` iterates until stable (max 10 passes).

### Suppression is transparent

Rules never check for `-- noqa` or `-- fmt: off` — that filtering is done by
`lint_source` and `fix_source` after collecting all violations and fixes. Rules
just emit everything they find.

## Pull request checklist

- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes (all existing tests still green)
- [ ] New tests added for the changed behaviour
- [ ] `CHANGELOG.md` updated under `[Unreleased]`
- [ ] Documentation updated (README, `docs/src/rules/`) if adding a rule
