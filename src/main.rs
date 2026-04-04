use clap::Parser;
use globset::{Glob, GlobSetBuilder};
use ignore::WalkBuilder;
use rayon::prelude::*;
use squint::{build_rules, config::Config, linter, rules};
use std::io::{BufWriter, Write};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(serde::Serialize)]
struct FileResult<'a> {
    path: String,
    violations: &'a [rules::Violation],
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    fixed: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    would_reformat: bool,
}

#[derive(Parser)]
#[command(
    name = "squint",
    about = "SQL linter for dbt/Jinja SQL files",
    long_about = "Lints SQL files for style issues. Config can be set in .sql-linter.toml."
)]
struct Cli {
    /// SQL files or directories to lint
    #[arg(required_unless_present = "stdin_filename")]
    files: Vec<std::path::PathBuf>,

    /// Read SQL from stdin; use this path as the reported filename
    #[arg(long, conflicts_with = "files", value_name = "FILENAME")]
    stdin_filename: Option<String>,

    /// Maximum line length (overrides config file)
    #[arg(long)]
    max_line_length: Option<usize>,

    /// Comma-separated list of rule IDs to run (default: all)
    #[arg(long, value_delimiter = ',')]
    rules: Option<Vec<String>>,

    /// Only show the violation count per file
    #[arg(short, long)]
    quiet: bool,

    /// Automatically fix violations where possible (rewrites files in place)
    #[arg(long)]
    fix: bool,

    /// Exit 1 if any file would be changed by --fix, without writing changes (CI gate)
    #[arg(long, conflicts_with = "fix")]
    check: bool,

    /// Write violations to this file in addition to stdout
    #[arg(long)]
    output: Option<std::path::PathBuf>,

    /// Glob patterns to exclude (may be repeated; merged with config `exclude`)
    #[arg(long = "exclude", value_name = "PATTERN")]
    exclude: Vec<String>,

    /// Output format: "text" (default) or "json"
    #[arg(long, default_value = "text", value_parser = ["text", "json"])]
    format: String,
}

/// Expand a mix of file and directory paths into a sorted list of .sql file paths,
/// excluding any path that matches one of the compiled `exclude` patterns.
///
/// Patterns are matched against the path relative to `base_dir`. Both the full
/// path string and each individual component are tried so that patterns like
/// `target/**` and `**/node_modules/**` work regardless of how the path was specified.
fn collect_paths(
    inputs: &[std::path::PathBuf],
    exclude: &globset::GlobSet,
    base_dir: &std::path::Path,
) -> Vec<std::path::PathBuf> {
    let mut paths: Vec<std::path::PathBuf> = Vec::new();
    for entry in inputs {
        if entry.is_dir() {
            WalkBuilder::new(entry)
                .build()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_some_and(|ft| ft.is_file()))
                .filter(|e| {
                    e.path()
                        .extension()
                        .and_then(|s| s.to_str())
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("sql"))
                })
                .filter(|e| !is_excluded(e.path(), exclude, base_dir))
                .for_each(|e| paths.push(e.into_path()));
        } else if !is_excluded(entry, exclude, base_dir) {
            paths.push(entry.clone());
        }
    }
    paths.sort();
    paths
}

/// Returns true if `path` matches any pattern in `exclude`.
/// Matches against the path relative to `base_dir` (falls back to the path as-is).
fn is_excluded(
    path: &std::path::Path,
    exclude: &globset::GlobSet,
    base_dir: &std::path::Path,
) -> bool {
    if exclude.is_empty() {
        return false;
    }
    let rel = path.strip_prefix(base_dir).unwrap_or(path);
    exclude.is_match(rel)
}

fn plural(n: usize, singular: &str, plural: &str) -> String {
    if n == 1 {
        format!("1 {}", singular)
    } else {
        format!("{} {}", n, plural)
    }
}

fn main() {
    let cli = Cli::parse();

    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut cfg = Config::load(&cwd);

    if let Some(max) = cli.max_line_length {
        cfg.rules.lt05.max_line_length = max;
    }

    let all_rules = build_rules(&cfg);

    let rules: Vec<&dyn rules::Rule> = match &cli.rules {
        Some(ids) => {
            let ids_upper: Vec<String> = ids.iter().map(|s| s.to_uppercase()).collect();
            all_rules
                .iter()
                .filter(|r| ids_upper.contains(&r.id().to_uppercase()))
                .map(|r| r.as_ref())
                .collect()
        }
        None => all_rules.iter().map(|r| r.as_ref()).collect(),
    };

    // ── stdin mode ───────────────────────────────────────────────────────────
    if let Some(filename) = &cli.stdin_filename {
        use std::io::Read;
        let mut source = String::new();
        std::io::stdin()
            .read_to_string(&mut source)
            .unwrap_or_else(|e| {
                eprintln!("error reading stdin: {}", e);
                std::process::exit(2);
            });

        let stdout = std::io::stdout();
        let mut out = BufWriter::new(stdout.lock());

        let (source_to_lint, was_fixed, check_would_change) = if cli.fix {
            let fixed = linter::fix_source(&source, rules.as_slice());
            let changed = fixed != source;
            // --fix on stdin: write fixed SQL back to stdout (can't rewrite the file).
            if changed {
                print!("{}", fixed);
            }
            (fixed, changed, false)
        } else if cli.check {
            let fixed = linter::fix_source(&source, rules.as_slice());
            let changed = fixed != source;
            (source, false, changed)
        } else {
            (source, false, false)
        };

        if cli.fix && was_fixed && !cli.quiet {
            writeln!(out, "{}: fixed", filename).unwrap();
        }
        if cli.check && check_would_change && !cli.quiet {
            writeln!(out, "{}: would reformat", filename).unwrap();
        }

        let violations = linter::lint_source(&source_to_lint, rules.as_slice());
        if cli.quiet {
            if !violations.is_empty() {
                writeln!(out, "{}: {} violation(s)", filename, violations.len()).unwrap();
            }
        } else {
            for v in &violations {
                writeln!(
                    out,
                    "{}:{}:{}: [{}] [{}] {}",
                    filename, v.line, v.col, v.severity, v.rule_id, v.message
                )
                .unwrap();
            }
        }

        // Summary line
        if cli.format != "json" {
            let total = violations.len();
            let error_count = violations
                .iter()
                .filter(|v| v.severity == rules::Severity::Error)
                .count();
            let warning_count = total - error_count;
            let summary = if total == 0 {
                "No violations found.".to_string()
            } else if error_count > 0 && warning_count > 0 {
                format!(
                    "Found {} and {} in 1 file.",
                    plural(error_count, "error", "errors"),
                    plural(warning_count, "warning", "warnings"),
                )
            } else {
                let kind = if warning_count > 0 {
                    ("warning", "warnings")
                } else {
                    ("violation", "violations")
                };
                format!("Found {} in 1 file.", plural(total, kind.0, kind.1))
            };
            writeln!(out, "{}", summary).unwrap();
        }

        out.flush().unwrap();

        let has_errors = violations
            .iter()
            .any(|v| v.severity == rules::Severity::Error);
        if check_would_change || has_errors {
            std::process::exit(1);
        }
        return;
    }

    // Build exclusion globset from config + CLI patterns
    let exclude_set = {
        let mut builder = GlobSetBuilder::new();
        for pattern in cfg.exclude.iter().chain(cli.exclude.iter()) {
            match Glob::new(pattern) {
                Ok(g) => {
                    builder.add(g);
                }
                Err(e) => eprintln!("warning: invalid exclude pattern '{}': {}", pattern, e),
            }
        }
        builder.build().unwrap_or_else(|e| {
            eprintln!("warning: could not build exclude patterns: {}", e);
            globset::GlobSet::empty()
        })
    };

    // Expand directories → sorted list of .sql files, respecting exclusions
    let all_paths = collect_paths(&cli.files, &exclude_set, &cfg.base_dir);

    let had_error = AtomicBool::new(false);
    let would_change = AtomicBool::new(false);

    // ── Parallel phase: read → fix → lint ────────────────────────────────────
    // Each result is (path, violations, was_fixed, check_would_change).
    let mut results: Vec<(std::path::PathBuf, Vec<rules::Violation>, bool, bool)> = all_paths
        .par_iter()
        .filter_map(|path| {
            let source = match std::fs::read_to_string(path) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: {}: {}", path.display(), e);
                    had_error.store(true, Ordering::Relaxed);
                    return None;
                }
            };

            let (source_to_lint, was_fixed, check_would_change) = if cli.fix {
                let fixed = linter::fix_source(&source, rules.as_slice());
                let changed = fixed != source;
                if changed {
                    if let Err(e) = std::fs::write(path, &fixed) {
                        eprintln!("error: {}: {}", path.display(), e);
                        had_error.store(true, Ordering::Relaxed);
                        return None;
                    }
                }
                (fixed, changed, false)
            } else if cli.check {
                let fixed = linter::fix_source(&source, rules.as_slice());
                let changed = fixed != source;
                if changed {
                    would_change.store(true, Ordering::Relaxed);
                }
                (source, false, changed)
            } else {
                (source, false, false)
            };

            let violations = linter::lint_source(&source_to_lint, rules.as_slice());
            Some((path.clone(), violations, was_fixed, check_would_change))
        })
        .collect();

    // Sort for deterministic output order (already sorted by collect_paths, but
    // par_iter may return results out of order).
    results.sort_by(|a, b| a.0.as_os_str().cmp(b.0.as_os_str()));

    // ── Serial output phase ───────────────────────────────────────────────────
    let stdout = std::io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    let mut file_out: Option<BufWriter<std::fs::File>> = cli.output.as_ref().map(|p| {
        BufWriter::new(
            std::fs::File::create(p)
                .unwrap_or_else(|e| panic!("cannot create output file '{}': {}", p.display(), e)),
        )
    });

    let mut had_errors = false; // true if any Error-severity violation exists

    if cli.format == "json" {
        let json_results: Vec<FileResult<'_>> = results
            .iter()
            .map(
                |(path, violations, was_fixed, check_would_change)| FileResult {
                    path: path.display().to_string(),
                    violations: violations.as_slice(),
                    fixed: *was_fixed,
                    would_reformat: *check_would_change,
                },
            )
            .collect();
        if results
            .iter()
            .any(|(_, v, _, _)| v.iter().any(|v| v.severity == rules::Severity::Error))
        {
            had_errors = true;
        }
        let json = serde_json::to_string_pretty(&json_results).unwrap();
        writeln!(out, "{}", json).unwrap();
        if let Some(ref mut f) = file_out {
            writeln!(f, "{}", json).unwrap();
        }
    } else {
        for (path, violations, was_fixed, check_would_change) in &results {
            if cli.fix && *was_fixed && !cli.quiet {
                writeln!(out, "{}: fixed", path.display()).unwrap();
                if let Some(ref mut f) = file_out {
                    writeln!(f, "{}: fixed", path.display()).unwrap();
                }
            }

            if cli.check && *check_would_change && !cli.quiet {
                writeln!(out, "{}: would reformat", path.display()).unwrap();
                if let Some(ref mut f) = file_out {
                    writeln!(f, "{}: would reformat", path.display()).unwrap();
                }
            }

            if violations
                .iter()
                .any(|v| v.severity == rules::Severity::Error)
            {
                had_errors = true;
            }

            if cli.quiet {
                if !violations.is_empty() {
                    writeln!(out, "{}: {} violation(s)", path.display(), violations.len()).unwrap();
                    if let Some(ref mut f) = file_out {
                        writeln!(f, "{}: {} violation(s)", path.display(), violations.len())
                            .unwrap();
                    }
                }
            } else {
                for v in violations {
                    writeln!(
                        out,
                        "{}:{}:{}: [{}] [{}] {}",
                        path.display(),
                        v.line,
                        v.col,
                        v.severity,
                        v.rule_id,
                        v.message
                    )
                    .unwrap();
                    if let Some(ref mut f) = file_out {
                        writeln!(
                            f,
                            "{}:{}:{}: [{}] [{}] {}",
                            path.display(),
                            v.line,
                            v.col,
                            v.severity,
                            v.rule_id,
                            v.message
                        )
                        .unwrap();
                    }
                }
            }
        }

        // ── Summary line ──────────────────────────────────────────────────
        let total: usize = results.iter().map(|(_, v, _, _)| v.len()).sum();
        let files_with_violations = results.iter().filter(|(_, v, _, _)| !v.is_empty()).count();
        let error_count = results
            .iter()
            .flat_map(|(_, v, _, _)| v.iter())
            .filter(|v| v.severity == rules::Severity::Error)
            .count();
        let warning_count = total - error_count;
        let files_fixed = results.iter().filter(|(_, _, fixed, _)| *fixed).count();
        let files_would_change = results.iter().filter(|(_, _, _, wc)| *wc).count();

        let mut parts: Vec<String> = Vec::new();
        if cli.fix && files_fixed > 0 {
            parts.push(format!("Fixed {}.", plural(files_fixed, "file", "files")));
        }
        if cli.check && files_would_change > 0 {
            parts.push(format!(
                "{} would be reformatted.",
                plural(files_would_change, "file", "files")
            ));
        }
        let violation_summary = if total == 0 {
            "No violations found.".to_string()
        } else if error_count > 0 && warning_count > 0 {
            format!(
                "Found {} and {} in {}.",
                plural(error_count, "error", "errors"),
                plural(warning_count, "warning", "warnings"),
                plural(files_with_violations, "file", "files"),
            )
        } else {
            let kind = if warning_count > 0 {
                ("warning", "warnings")
            } else {
                ("violation", "violations")
            };
            format!(
                "Found {} in {}.",
                plural(total, kind.0, kind.1),
                plural(files_with_violations, "file", "files"),
            )
        };
        parts.push(violation_summary);
        let summary = parts.join(" ");
        writeln!(out, "{}", summary).unwrap();
        if let Some(ref mut f) = file_out {
            writeln!(f, "{}", summary).unwrap();
        }
    }

    out.flush().unwrap();
    if let Some(ref mut f) = file_out {
        f.flush().unwrap();
    }

    if had_error.load(Ordering::Relaxed) {
        std::process::exit(2);
    } else if had_errors || would_change.load(Ordering::Relaxed) {
        std::process::exit(1);
    }
}
