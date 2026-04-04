// benches/linter_bench.rs
//
// Criterion microbenchmarks for the squint pipeline.
//
// Run all:              cargo bench
// Run one group:        cargo bench -- tokenize
// Dry-run (no timing):  cargo bench -- --test
// HTML report:          open target/criterion/report/index.html

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rayon::prelude::*;
use squint::{
    analysis::clause_map,
    lexer::Lexer,
    linter::lint_source,
    parser::Parser,
    rules::{
        aliasing::ImplicitAliases,
        capitalisation::{
            FunctionCasing, IdentifierCasing, KeywordCasing, LiteralCasing, TypeCasing,
        },
        convention::IsNull,
        layout::{Indent, LineLength, Spacing},
        structure::UnusedCte,
        Rule,
    },
};
use std::path::Path;

// ── Fixture loader ────────────────────────────────────────────────────────────

fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("benches/fixtures")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("could not read fixture '{}': {}", name, e))
}

// ── Benchmark groups ──────────────────────────────────────────────────────────

/// Lexer::tokenize() only — measures regex matching cost per file size.
fn bench_tokenize(c: &mut Criterion) {
    let small = fixture("small.sql");
    let medium = fixture("medium.sql");
    let large = fixture("large.sql");

    let mut g = c.benchmark_group("tokenize");
    for (label, sql) in [("small", &small), ("medium", &medium), ("large", &large)] {
        g.bench_with_input(BenchmarkId::new("Lexer", label), sql, |b, sql| {
            b.iter(|| black_box(Lexer::new(black_box(sql.as_str())).tokenize()))
        });
    }
    g.finish();
}

/// Lexer + Parser::parse() — measures full tokenise-and-build-node-list cost.
fn bench_parse(c: &mut Criterion) {
    let small = fixture("small.sql");
    let medium = fixture("medium.sql");
    let large = fixture("large.sql");

    let mut g = c.benchmark_group("parse");
    for (label, sql) in [("small", &small), ("medium", &medium), ("large", &large)] {
        g.bench_with_input(BenchmarkId::new("Lexer+Parser", label), sql, |b, sql| {
            b.iter(|| {
                let tokens = Lexer::new(black_box(sql.as_str())).tokenize();
                black_box(Parser::new(tokens).parse())
            })
        });
    }
    g.finish();
}

/// analysis::clause_map() on pre-parsed nodes — isolates this shared helper,
/// which is called by many rules (AM01, AM06, CV03, RF01, table_aliases, select_items).
fn bench_clause_map(c: &mut Criterion) {
    let small_src = fixture("small.sql");
    let medium_src = fixture("medium.sql");
    let large_src = fixture("large.sql");
    let small_n = Parser::new(Lexer::new(&small_src).tokenize()).parse();
    let medium_n = Parser::new(Lexer::new(&medium_src).tokenize()).parse();
    let large_n = Parser::new(Lexer::new(&large_src).tokenize()).parse();

    let mut g = c.benchmark_group("clause_map");
    for (label, nodes) in [
        ("small", &small_n),
        ("medium", &medium_n),
        ("large", &large_n),
    ] {
        g.bench_with_input(BenchmarkId::new("analysis", label), nodes, |b, nodes| {
            b.iter(|| black_box(clause_map(black_box(nodes))))
        });
    }
    g.finish();
}

/// Full lint_source() with a representative rule set — end-to-end pipeline cost.
fn bench_lint_all(c: &mut Criterion) {
    let small = fixture("small.sql");
    let medium = fixture("medium.sql");
    let large = fixture("large.sql");

    // Build rules once outside the timed loop
    let rules: Vec<Box<dyn Rule>> = vec![
        Box::new(KeywordCasing),
        Box::new(IdentifierCasing),
        Box::new(FunctionCasing),
        Box::new(LiteralCasing),
        Box::new(TypeCasing),
        Box::new(ImplicitAliases),
        Box::new(Spacing),
        Box::new(Indent),
        Box::new(LineLength::new(120)),
        Box::new(IsNull),
        Box::new(UnusedCte),
    ];
    let rule_refs: Vec<&dyn Rule> = rules.iter().map(|r| r.as_ref()).collect();

    let mut g = c.benchmark_group("lint_all");
    for (label, sql) in [("small", &small), ("medium", &medium), ("large", &large)] {
        g.bench_with_input(BenchmarkId::new("lint_source", label), sql, |b, sql| {
            b.iter(|| black_box(lint_source(black_box(sql.as_str()), black_box(&rule_refs))))
        });
    }
    g.finish();
}

/// Single-rule cost on the medium fixture — useful for profiling individual rules.
fn bench_single_rule(c: &mut Criterion) {
    let medium = fixture("medium.sql");

    let cp01: Box<dyn Rule> = Box::new(KeywordCasing);
    let lt01: Box<dyn Rule> = Box::new(Spacing);
    let cp01_refs: Vec<&dyn Rule> = vec![cp01.as_ref()];
    let lt01_refs: Vec<&dyn Rule> = vec![lt01.as_ref()];

    let mut g = c.benchmark_group("single_rule/medium");
    g.bench_function("CP01_KeywordCasing", |b| {
        b.iter(|| {
            black_box(lint_source(
                black_box(medium.as_str()),
                black_box(&cp01_refs),
            ))
        })
    });
    g.bench_function("LT01_Spacing", |b| {
        b.iter(|| {
            black_box(lint_source(
                black_box(medium.as_str()),
                black_box(&lt01_refs),
            ))
        })
    });
    g.finish();
}

/// Full per-file cost including read_to_string — measures warm-cache I/O + pipeline.
/// Compare with bench_lint_all (string-only) to isolate I/O overhead.
fn bench_file_io(c: &mut Criterion) {
    let small_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("benches/fixtures/small.sql");
    let medium_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("benches/fixtures/medium.sql");
    let large_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("benches/fixtures/large.sql");

    let rules: Vec<Box<dyn Rule>> = vec![
        Box::new(KeywordCasing),
        Box::new(IdentifierCasing),
        Box::new(FunctionCasing),
        Box::new(LiteralCasing),
        Box::new(TypeCasing),
        Box::new(ImplicitAliases),
        Box::new(Spacing),
        Box::new(Indent),
        Box::new(LineLength::new(120)),
        Box::new(IsNull),
        Box::new(UnusedCte),
    ];
    let rule_refs: Vec<&dyn Rule> = rules.iter().map(|r| r.as_ref()).collect();

    let mut g = c.benchmark_group("file_io");
    for (label, path) in [
        ("small", &small_path),
        ("medium", &medium_path),
        ("large", &large_path),
    ] {
        g.bench_with_input(BenchmarkId::new("read+lint", label), path, |b, path| {
            b.iter(|| {
                let source = std::fs::read_to_string(black_box(path)).unwrap();
                black_box(lint_source(black_box(&source), black_box(&rule_refs)))
            })
        });
    }
    g.finish();
}

/// Multi-file throughput: serial vs rayon, for N copies of medium.sql.
/// Shows real parallelism gain on this machine's CPU count.
fn bench_parallel(c: &mut Criterion) {
    let medium = fixture("medium.sql");

    let rules: Vec<Box<dyn Rule>> = vec![
        Box::new(KeywordCasing),
        Box::new(IdentifierCasing),
        Box::new(FunctionCasing),
        Box::new(LiteralCasing),
        Box::new(TypeCasing),
        Box::new(ImplicitAliases),
        Box::new(Spacing),
        Box::new(Indent),
        Box::new(LineLength::new(120)),
        Box::new(IsNull),
        Box::new(UnusedCte),
    ];
    let rule_refs: Vec<&dyn Rule> = rules.iter().map(|r| r.as_ref()).collect();

    let mut g = c.benchmark_group("parallel_files");

    for &n in &[1usize, 4, 16, 64] {
        let sources: Vec<&str> = std::iter::repeat(medium.as_str()).take(n).collect();

        g.bench_with_input(BenchmarkId::new("serial", n), &sources, |b, sources| {
            b.iter(|| {
                sources
                    .iter()
                    .map(|s| lint_source(black_box(s), black_box(&rule_refs)))
                    .collect::<Vec<_>>()
            })
        });

        g.bench_with_input(BenchmarkId::new("rayon", n), &sources, |b, sources| {
            b.iter(|| {
                sources
                    .par_iter()
                    .map(|s| lint_source(black_box(s), black_box(&rule_refs)))
                    .collect::<Vec<_>>()
            })
        });
    }
    g.finish();
}

/// Output formatting cost in isolation: write violations to a Vec<u8> sink.
/// Compares unbuffered writeln! vs BufWriter to show batching benefit.
fn bench_output(c: &mut Criterion) {
    use std::io::{BufWriter, Write};

    let medium = fixture("medium.sql");
    let rules: Vec<Box<dyn Rule>> = vec![
        Box::new(KeywordCasing),
        Box::new(IdentifierCasing),
        Box::new(FunctionCasing),
        Box::new(LiteralCasing),
        Box::new(TypeCasing),
        Box::new(ImplicitAliases),
        Box::new(Spacing),
        Box::new(Indent),
        Box::new(LineLength::new(120)),
        Box::new(IsNull),
        Box::new(UnusedCte),
    ];
    let rule_refs: Vec<&dyn Rule> = rules.iter().map(|r| r.as_ref()).collect();
    let violations = lint_source(&medium, &rule_refs);
    let path = Path::new("medium.sql");

    let mut g = c.benchmark_group("output");

    g.bench_function("unbuffered_sink", |b| {
        b.iter(|| {
            let mut sink = std::io::sink();
            for v in black_box(&violations) {
                writeln!(
                    sink,
                    "{}:{}:{}: [{}] {}",
                    path.display(),
                    v.line,
                    v.col,
                    v.rule_id,
                    v.message
                )
                .unwrap();
            }
        })
    });

    g.bench_function("bufwriter_vec", |b| {
        b.iter(|| {
            let buf = Vec::with_capacity(violations.len() * 80);
            let mut w = BufWriter::new(buf);
            for v in black_box(&violations) {
                writeln!(
                    w,
                    "{}:{}:{}: [{}] {}",
                    path.display(),
                    v.line,
                    v.col,
                    v.rule_id,
                    v.message
                )
                .unwrap();
            }
            black_box(w.into_inner().unwrap())
        })
    });

    g.finish();
}

// ── Wire up ───────────────────────────────────────────────────────────────────

criterion_group!(
    benches,
    bench_tokenize,
    bench_parse,
    bench_clause_map,
    bench_lint_all,
    bench_single_rule,
    bench_file_io,
    bench_parallel,
    bench_output,
);
criterion_main!(benches);
