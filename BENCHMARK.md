# Benchmark Results

Benchmarks compare `squint` against [sqlfluff](https://docs.sqlfluff.com/) and [sqlfmt](https://sqlfmt.com/) on three SQL fixtures of increasing size. All numbers are from a release build (`cargo build --release`).

## Fixtures

| Fixture | Lines | Description |
|---|---|---|
| `small.sql` | ~40 | Single dbt model, 2 CTEs, GROUP BY |
| `medium.sql` | ~240 | 8 CTEs, window functions, JOINs, Jinja blocks |
| `large.sql` | ~2400 | 10× medium with renamed CTEs, 10 statements |

Fixtures intentionally include violations (uppercase keywords, implicit aliases, `= NULL`) so all rules do real work.

## Wall-clock (hyperfine, 10 runs, 3 warmup)

Measures total process time including startup and file I/O. Run with:

```
./scripts/bench_compare.sh
```

| Fixture | squint | sqlfmt | sqlfluff | vs sqlfmt | vs sqlfluff |
|---|---|---|---|---|---|
| small | **2.4 ms** ± 0.4 ms | 157 ms ± 3 ms | 263 ms ± 18 ms | **67× faster** | **112× faster** |
| medium | **2.9 ms** ± 0.2 ms | 186 ms ± 6 ms | 271 ms ± 6 ms | **64× faster** | **93× faster** |
| large | **5.5 ms** ± 0.4 ms | 509 ms ± 14 ms | 265 ms ± 7 ms | **92× faster** | **48× faster** |

The small/medium times roughly halved vs the previous baseline (4.6 ms → 2.4 ms, 6.5 ms → 2.9 ms) due to parallel pipeline startup and LintContext deduplication. Large improved most dramatically (33.6 ms → 5.5 ms) as parallelism amortises the heavier per-file lint work across threads.

The large-file lead over sqlfluff now reverses vs sqlfmt: at 2400 lines sqlfmt's actual work begins to dominate its startup cost, making it slower than sqlfluff.

## Internal pipeline (criterion, release build)

Measures individual pipeline stages in isolation. Run with:

```
cargo bench
```

### Tokenize (`Lexer::tokenize`)

| Fixture | Time |
|---|---|
| small | 4.2 µs ± 0.1 µs |
| medium | 46 µs ± 0.5 µs |
| large | 468 µs ± 3 µs |

### Lex + Parse (`Lexer::tokenize` + `Parser::parse`)

| Fixture | Time |
|---|---|
| small | 4.9 µs ± 0.1 µs |
| medium | 51.8 µs ± 0.4 µs |
| large | 509 µs ± 4 µs |

### `analysis::clause_map` (on pre-parsed nodes)

Now computed once per `lint_source` call and shared across all rules via `LintContext`.

| Fixture | Time |
|---|---|
| small | 224 ns ± 1 ns |
| medium | 1.91 µs ± 0.02 µs |
| large | 21.4 µs ± 0.2 µs |

### Full lint (`lint_source`, 11 rules)

| Fixture | Time |
|---|---|
| small | 9.7 µs ± 0.1 µs |
| medium | 97 µs ± 0.8 µs |
| large | 1.06 ms ± 0.01 ms |

### Single rule on medium

| Rule | Time |
|---|---|
| CP01 `KeywordCasing` | 63.5 µs ± 0.3 µs |
| LT01 `Spacing` | 63.2 µs ± 0.1 µs |

### File I/O + lint (`read_to_string` + pipeline, warm OS cache)

| Fixture | Time | I/O overhead vs string-only |
|---|---|---|
| small | 15.7 µs ± 0.1 µs | +6 µs |
| medium | 109 µs ± 0.7 µs | +12 µs |
| large | 1.07 ms ± 0.01 ms | +11 µs |

`read_to_string` adds ~10–15 µs on warm cache (file already in the OS page cache). Cold-cache reads would be higher but aren't reproducibly benchmarkable in Criterion without root access.

### Multi-file parallel throughput (`lint_source`, N × medium.sql)

| N files | Serial | rayon | Speedup |
|---|---|---|---|
| 1 | 98 µs | 99 µs | 1.0× |
| 4 | 393 µs | 214 µs | **1.8×** |
| 16 | 1.62 ms | 444 µs | **3.6×** |
| 64 | 6.59 ms | 1.31 ms | **5.0×** |

Speedup grows with N as rayon's thread pool amortises scheduling overhead. Measured on this machine's CPU (see Environment).

### Output formatting cost (`writeln!` to a `Vec<u8>` sink)

| Mode | Time |
|---|---|
| Unbuffered sink (discards) | ~237 ps |
| BufWriter to Vec<u8> | ~2.4 µs |

The extra allocation cost of `BufWriter` is negligible. Real-world benefit comes from replacing per-violation `write` syscalls with block-buffered flushes when writing to terminal or file.

## Key findings

- **logos DFA + zero-copy is ~31× faster than the original regex loop.** Two-phase optimization: (1) logos DFA cut tokenizer from 1.48 ms → 94 µs (~16×); (2) zero-copy `Node::value: &str` eliminated per-token heap allocations, cutting another ~2× to 46 µs.
- **Full pipeline: 1.67 ms → 97 µs on medium (~17× faster).** Lex+parse went from 1.55 ms to 51.8 µs. Wall-clock for a single file: 6.5 ms → 2.9 ms including process startup.
- **`clause_map` recovered** from the logos-era 30% regression: zero-copy nodes are more cache-friendly, restoring its performance to below the original baseline (1.91 µs vs 1.88 µs original).
- **Rules and lexer now share roughly equal time.** With lex+parse at ~46 µs and full lint at 97 µs, rules consume ~51 µs (~52% of pipeline).
- **Parallel file processing scales to 5× on 64 files.** rayon `par_iter` over independent files gives 1.8× at 4 files, 3.6× at 16, 5.0× at 64. Overhead dominates at 1 file (~0%).
- **File I/O adds ~11 µs per file** on warm OS cache. `read_to_string` for a 10 KB SQL file costs ~10–15 µs — negligible relative to lint time for medium/large files.
- **Linear scaling holds.** Tokenize time scales ~10× from medium to large, matching file-size ratio.
- **Python startup dominates at small files.** sqlfluff and sqlfmt spend ~150–250 ms loading before processing a single byte. At large files their actual work begins to show.

## Optimization log

| Change | Expected gain | Measured gain | Notes |
|---|---|---|---|
| `phf` keyword map in `refine_keyword` | 5–10% lexer | ~0% (noise) | `refine_keyword` only runs on Name tokens (~25% of all tokens); regex loop dominated |
| `logos` DFA lexer replacing regex loop | 50–75% lexer | **~94% lexer / ~84% full lint** | DFA compiled at build time; eliminates 31 sequential `Regex::captures()` per token |
| Zero-copy `Node::value: &str` | 30–50% alloc reduction | **~50% tokenize / ~50% full lint** | Eliminates `to_string()` per token and two String clones per token in parser |
| Rule-level algorithmic fixes | 5–15% rules | **~0% on bench fixtures** | AL05 O(n×m)→O(n+a) HashMap; LT07/LT08 3-scan-per-CTE eliminated (fixed multi-CTE bug too); LT11 O(n) ptr::eq→index walk; RF01 deduped clause_map; all `to_lowercase()==` → `eq_ignore_ascii_case`. Gains are worst-case, not typical. |
| `LineIndex` pre-computed offset→line/col | 15–30% rules | **-27% medium / -77% large** | Ruff-inspired: scan source once at lint start, binary-search per violation. Eliminated O(offset) byte scan repeated once per violation. 131 µs → 95 µs (medium), 4.49 ms → 1.02 ms (large). |
| `LintContext` + dedup `clause_map` | ~10% per-file | **~0% measured** | Pre-compute clause_map once per lint_source call; share via LintContext across all 30 rules. Eliminated 5–7 redundant O(n) passes. Gain absorbed into measurement noise at this scale. |
| rayon parallel file processing | ~0% per-file | **1.8–5× multi-file** | Replace serial for loop with par_iter; collect results, sort by path, serial print. 1.8× at 4 files, 3.6× at 16, 5.0× at 64. Rule: Sync bound added to enable this. |
| BufWriter stdout + --output flag | ~0% per-file | batches output syscalls | Replace println! with writeln!(BufWriter). Add --output <file> for CI pipelines. |
| Directory walking via `ignore` crate | UX | UX | Accept directories as input; auto-discover .sql files; respects .gitignore. Same engine as ripgrep. |
| `fmt:off` suppression | 0% (feature) | **−4% tokenize** | `classify_comment` reclassifies `-- fmt: off`/`on` in the `LineComment` branch only. Removing the two competing DFA patterns simplified the logos automaton, saving ~2 µs on tokenize/medium. `source.contains("fmt:")` fast-path in `compute_fmt_off_ranges` keeps lint overhead <1% for files with no markers. |

## Environment

| | |
|---|---|
| CPU | see `lscpu` |
| OS | Linux 6.17.0-19-generic |
| Rust | `rustc --version` |
| sqlfluff | 4.1.0 |
| sqlfmt | 0.29.0 |
| hyperfine | 1.20.0 |
